//! Target configuration: the ordered list of products the updater maintains.
//!
//! Two shapes are accepted. The legacy single-product file (flat
//! `repository`/`channel`/`install_dir`/`service_name`/`asset_pattern` keys)
//! keeps behaving exactly as before and is interpreted as one target named
//! `guidance-runtime`. The multi-target form declares a `[[targets]]` array.
//!
//! The two shapes are deliberately *not* modelled as a serde untagged enum: an
//! untagged enum reports "data did not match any variant" when something is
//! wrong, and the specification requires the offending field to be named. So
//! deserialization goes through [`ConfigFile`], whose fields are all optional,
//! and validation afterwards produces messages that point at the field.

use crate::Channel;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

/// The name the legacy flat configuration maps onto.
pub const LEGACY_TARGET_NAME: &str = "guidance-runtime";

/// Whether a target's releases are readable without a credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Readable by anyone; no credential is consulted.
    #[default]
    Public,
    /// Requires the credential entry named by the configuration.
    Private,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Public => "public",
            Self::Private => "private",
        })
    }
}

/// One product the updater maintains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateTarget {
    /// Stable identifier, unique per configuration.
    pub name: String,
    /// `owner/repository` holding the releases.
    pub repository: String,
    #[serde(default)]
    pub visibility: Visibility,
    /// Name of the credential store entry. Required for private targets and
    /// never the secret itself.
    #[serde(default)]
    pub credential_entry: Option<String>,
    #[serde(default = "default_channel")]
    pub channel: Channel,
    /// Required in the `[[targets]]` form: each target must state where it is
    /// installed. Only the legacy flat form defaults this value, because that
    /// form predates the field being mandatory.
    pub install_dir: PathBuf,
    /// Present only for targets that control a service.
    #[serde(default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub asset_pattern: Option<String>,
}

fn default_channel() -> Channel {
    Channel::Stable
}
fn default_install_dir() -> PathBuf {
    PathBuf::from(r"C:\Program Files\Guidance Runtime")
}

impl UpdateTarget {
    /// The asset suffix to look for: the explicit override when configured,
    /// otherwise the suffix of the platform and architecture this binary runs
    /// on.
    ///
    /// An explicitly configured but empty value is rejected rather than
    /// treated as "unset": an empty suffix matches every archive, which is
    /// exactly the architecture-blind behaviour this type avoids.
    pub fn resolved_asset_pattern(&self) -> Result<String> {
        let Some(raw) = self.asset_pattern.as_deref() else {
            return Ok(
                crate::arch::asset_suffix_for_arch(crate::arch::running_arch())?.to_string(),
            );
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            bail!(
                "asset_pattern is set but blank; omit it to derive the pattern from the running \
                 architecture"
            );
        }
        if !trimmed.ends_with(".zip") {
            bail!("asset_pattern must end with .zip, got '{trimmed}'");
        }
        Ok(trimmed.to_string())
    }
}

/// The on-disk shape. Every field is optional so that both the legacy and the
/// multi-target form deserialize, and so validation can name what is missing.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    // Legacy flat fields.
    pub repository: Option<String>,
    pub channel: Option<Channel>,
    pub install_dir: Option<PathBuf>,
    pub service_name: Option<String>,
    pub asset_pattern: Option<String>,
    // Multi-target form.
    pub targets: Option<Vec<UpdateTarget>>,
}

/// A validated configuration. Callers only ever see this normalized form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateConfig {
    targets: Vec<UpdateTarget>,
    /// The shape the file had when it was loaded.
    ///
    /// Saving must reproduce it: rewriting a legacy flat file into the
    /// `[[targets]]` form on the first `channel` change would silently reshape
    /// an operator's file, which FR-003 forbids.
    shape: FileShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileShape {
    /// The flat single-product keys.
    Legacy,
    /// The `[[targets]]` array.
    Targets,
}

impl UpdateConfig {
    pub fn targets(&self) -> &[UpdateTarget] {
        &self.targets
    }

    /// Whether this configuration was loaded from the historical flat form.
    ///
    /// The CLI uses this to keep the pre-feature output contract for such a
    /// file: an operator who never asked for multiple targets must not see
    /// per-target output, and scripts matching the old wording must keep
    /// working (SC-001).
    pub fn is_legacy_shape(&self) -> bool {
        self.shape == FileShape::Legacy
    }

    /// Builds a configuration directly from already-validated targets.
    ///
    /// Used by tests and by callers that assemble targets programmatically;
    /// the validation rules are re-applied so this cannot be a back door
    /// around FR-004.
    pub fn from_config_targets(targets: Vec<UpdateTarget>) -> Self {
        validate(&targets).expect("programmatic targets must satisfy the same rules as a file");
        Self {
            targets,
            shape: FileShape::Targets,
        }
    }

    /// The single target, for call sites that predate multi-target support.
    ///
    /// Returns an error for a multi-target configuration rather than silently
    /// picking one: choosing a target implicitly is how a file meant for
    /// `guidance` would get installed over the runtime.
    pub fn single_target(&self) -> Result<&UpdateTarget> {
        match self.targets.as_slice() {
            [only] => Ok(only),
            many => bail!(
                "this operation works on a single target, but {} are configured ({}); \
                 name the target explicitly",
                many.len(),
                many.iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// A one-target view of this configuration, for `--target`.
    ///
    /// Narrowing happens before the fan-out, not while printing: the contract
    /// says `--target` narrows the run to one target, and a check that still
    /// contacts every other source (and consults their credential stores) both
    /// violates that and makes the JSON document carry more than one element.
    /// The file shape is preserved so saving behaviour is unchanged.
    pub fn select_target(&self, name: &str) -> Result<Self> {
        let target = self
            .targets
            .iter()
            .find(|t| t.name == name)
            .with_context(|| {
                format!(
                    "target '{name}' is not configured; configured targets: {}",
                    self.target_names().join(", ")
                )
            })?
            .clone();
        Ok(Self {
            targets: vec![target],
            shape: self.shape,
        })
    }

    /// Parses and validates a configuration document.
    pub fn parse(text: &str) -> Result<Self> {
        let file: ConfigFile = toml::from_str(text).context("parse update configuration")?;
        Self::from_file(file)
    }

    /// Validates an already-deserialized configuration document.
    pub fn from_file(file: ConfigFile) -> Result<Self> {
        let legacy_present = file.repository.is_some()
            || file.channel.is_some()
            || file.install_dir.is_some()
            || file.service_name.is_some()
            || file.asset_pattern.is_some();

        let (targets, shape) = match file.targets {
            Some(list) => {
                if legacy_present {
                    bail!(
                        "the configuration declares both the legacy flat keys and a [[targets]] \
                         list; use one form. The ambiguous field is 'targets'"
                    );
                }
                if list.is_empty() {
                    bail!("'targets' is present but empty; declare at least one target");
                }
                (list, FileShape::Targets)
            }
            None => {
                let Some(repository) = file.repository.clone() else {
                    bail!(
                        "'repository' is missing: declare either the legacy 'repository' key or a \
                         [[targets]] list"
                    );
                };
                (
                    vec![UpdateTarget {
                        name: LEGACY_TARGET_NAME.to_string(),
                        repository,
                        visibility: Visibility::Public,
                        credential_entry: None,
                        channel: file.channel.unwrap_or_default(),
                        install_dir: file.install_dir.unwrap_or_else(default_install_dir),
                        service_name: file.service_name,
                        asset_pattern: file.asset_pattern,
                    }],
                    FileShape::Legacy,
                )
            }
        };

        validate(&targets)?;
        Ok(Self { targets, shape })
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        Self::parse(&text)
    }

    /// Writes the configuration back in the shape it was loaded with.
    ///
    /// A legacy flat file stays flat and a `[[targets]]` file stays an array.
    /// Converting on save would silently reshape an operator's file the first
    /// time they touched an unrelated key, which FR-003 forbids.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?
        }
        let text = match self.shape {
            FileShape::Targets => {
                #[derive(Serialize)]
                struct Out<'a> {
                    targets: &'a [UpdateTarget],
                }
                toml::to_string_pretty(&Out {
                    targets: &self.targets,
                })?
            }
            FileShape::Legacy => {
                let target = self
                    .targets
                    .first()
                    .context("a legacy configuration must hold exactly one target")?;
                #[derive(Serialize)]
                struct Out<'a> {
                    repository: &'a str,
                    channel: Channel,
                    install_dir: &'a PathBuf,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    service_name: Option<&'a String>,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    asset_pattern: Option<&'a String>,
                }
                toml::to_string_pretty(&Out {
                    repository: &target.repository,
                    channel: target.channel,
                    install_dir: &target.install_dir,
                    service_name: target.service_name.as_ref(),
                    asset_pattern: target.asset_pattern.as_ref(),
                })?
            }
        };
        fs::write(path, text)?;
        Ok(())
    }

    /// Replaces one target's channel, preserving every other target.
    pub fn set_target_channel(&mut self, name: &str, channel: Channel) -> Result<()> {
        let target = self
            .targets
            .iter_mut()
            .find(|t| t.name == name)
            .with_context(|| format!("target '{name}' is not configured"))?;
        target.channel = channel;
        Ok(())
    }

    /// Names of every configured target, in configuration order.
    pub fn target_names(&self) -> Vec<&str> {
        self.targets.iter().map(|t| t.name.as_str()).collect()
    }
}

fn validate(targets: &[UpdateTarget]) -> Result<()> {
    let mut names = HashSet::new();
    let mut dirs: HashSet<&PathBuf> = HashSet::new();

    for target in targets {
        if target.name.trim().is_empty() {
            bail!("a target has an empty 'name'; every target needs a stable, non-empty name");
        }
        if !names.insert(target.name.as_str()) {
            bail!(
                "duplicate target name '{}'; target names must be unique",
                target.name
            );
        }
        if !dirs.insert(&target.install_dir) {
            bail!(
                "duplicate 'install_dir' for target '{}': two targets must not share an install \
                 directory, because installing one would overwrite the other",
                target.name
            );
        }
        if target.visibility == Visibility::Private {
            match target.credential_entry.as_deref() {
                Some(entry) if !entry.trim().is_empty() => {}
                _ => bail!(
                    "target '{}' is private but names no 'credential_entry'; a private target \
                     must name the credential store entry to read (the entry name, never the \
                     secret itself)",
                    target.name
                ),
            }
        }
        if target.repository.trim().is_empty() {
            bail!(
                "target '{}' has an empty 'repository'; expected owner/repository",
                target.name
            );
        }
        // Rejects a blank or non-.zip pattern with the field name in the
        // message, matching the shape of the other rules.
        target.resolved_asset_pattern()?;
    }

    // A rollback snapshot must not collide with another target's installation:
    // the transaction removes an existing backup directory before copying, so
    // such a collision would delete a product's files. `dirs` holds every
    // install directory, so this catches both directions.
    let mut backups: HashSet<PathBuf> = HashSet::new();
    for target in targets {
        let backup = crate::install::backup_path(&target.install_dir);
        if dirs.contains(&backup) {
            bail!(
                "target '{}' would use '{}' as its rollback directory, which is another \
                 target's install_dir; choose a different install_dir",
                target.name,
                backup.display()
            );
        }
        if !backups.insert(backup.clone()) {
            bail!(
                "two targets would use '{}' as their rollback directory; choose install \
                 directories whose names do not collide",
                backup.display()
            );
        }
    }
    Ok(())
}
