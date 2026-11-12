//! Shared command bodies for update operations.
//!
//! Both the operator CLI (`guidance update …`) and the standalone updater
//! binary use these functions. Only rendering differs between the two; the
//! *install* paths stay separate by design: the CLI spawns `guidance-updater`
//! with `--wait-pid` so it can exit before its own binaries are replaced,
//! while the updater performs the actual install.

use crate::{AvailableUpdate, Channel, UpdateConfig, UpdateTarget};
use anyhow::{Context, Result};
use semver::Version;

/// Result of an update check, ready for rendering.
#[derive(Debug, PartialEq)]
pub enum UpdateOutcome {
    Available(AvailableUpdate),
    UpToDate(Channel),
}

impl UpdateOutcome {
    pub fn message(&self) -> String {
        match self {
            UpdateOutcome::Available(u) => {
                format!(
                    "Update available: {} -> {} ({})",
                    u.current, u.version, u.channel
                )
            }
            UpdateOutcome::UpToDate(channel) => {
                format!("No update available on {channel} channel")
            }
        }
    }
}

/// Reads or changes the channel of one target.
///
/// With `channel` given, this mutates the in-memory configuration and the
/// caller is responsible for saving it — the function performs no I/O, so a
/// rejected change cannot leave a half-written file behind. With `channel`
/// absent it only reports the current value.
///
/// A multi-target configuration requires `target`, because applying a channel
/// to every target at once would hide which product changed.
pub fn set_or_get_channel(
    config: &mut UpdateConfig,
    target: Option<&str>,
    channel: Option<Channel>,
) -> Result<String> {
    let name = match target {
        Some(n) => n.to_string(),
        None => config.single_target()?.name.clone(),
    };
    match channel {
        Some(c) => {
            config.set_target_channel(&name, c)?;
            Ok(format!("Update channel for target '{name}' set to {c}"))
        }
        None => {
            let current = config
                .targets()
                .iter()
                .find(|t| t.name == name)
                .with_context(|| format!("target '{name}' is not configured"))?;
            Ok(current.channel.to_string())
        }
    }
}

/// Checks for an update and returns a renderable outcome.
pub async fn check_update(
    target: &UpdateTarget,
    token: Option<&crate::Secret>,
    current: &Version,
) -> Result<UpdateOutcome> {
    match crate::check(target, token, current).await? {
        Some(update) => Ok(UpdateOutcome::Available(update)),
        None => Ok(UpdateOutcome::UpToDate(target.channel)),
    }
}

/// Builds the per-target check a legacy outcome represents.
///
/// The legacy path renders its historical message by default, but an operator
/// who asks for JSON must get a document in the same shape as the multi-target
/// one. The conversion is centralised so both binaries map the two historical
/// outcomes identically and neither can invent a different installed/offered
/// pair.
pub fn check_from_outcome(
    target: &UpdateTarget,
    outcome: &UpdateOutcome,
    current: &Version,
) -> crate::TargetCheck {
    use crate::{TargetCheck, TargetState};

    match outcome {
        UpdateOutcome::Available(update) => TargetCheck::new(
            &target.name,
            TargetState::UpdateAvailable,
            Some(update.current.to_string()),
            Some(update.version.to_string()),
            update.channel.to_string(),
            None,
        ),
        UpdateOutcome::UpToDate(channel) => TargetCheck::new(
            &target.name,
            TargetState::UpToDate,
            Some(current.to_string()),
            None,
            channel.to_string(),
            None,
        ),
    }
}

/// The same outcome against an arbitrary [`crate::Releases`] port.
///
/// This is the historical single-target path, kept separately so the legacy
/// equivalence test can pin its exact wording without reaching for the network.
pub async fn check_update_with(
    source: &dyn crate::Releases,
    target: &UpdateTarget,
    token: Option<&crate::Secret>,
    current: &Version,
) -> Result<UpdateOutcome> {
    match crate::check_with(source, target, token, current).await? {
        Some(update) => Ok(UpdateOutcome::Available(update)),
        None => Ok(UpdateOutcome::UpToDate(target.channel)),
    }
}

/// Decides whether a check run must use the historical single-product output
/// contract.
///
/// This predicate is the whole routing decision, pulled into one place and
/// tested directly: an independent review showed that both the legacy wording
/// and the legacy exit status live behind it, so a mutation of the rendering
/// functions alone could pass every other test while the binary printed
/// something else.
///
/// `--target` opts out because naming a target is an explicit request for the
/// per-target shape.
pub fn uses_legacy_output(config: &UpdateConfig, target: Option<&str>) -> bool {
    config.is_legacy_shape() && target.is_none()
}

/// Parses the running crate version, used when no explicit version is given.
pub fn current_version() -> Result<Version> {
    Version::parse(env!("CARGO_PKG_VERSION")).context("parse crate version")
}

/// Checks every configured target, isolating each target's failure.
///
/// Always returns one [`crate::TargetCheck`] per configured target, in
/// configuration order (FR-008): a target that cannot be reached must not
/// hide the others.
pub async fn check_all(
    source: &dyn crate::Releases,
    store: &dyn crate::CredentialStore,
    config: &UpdateConfig,
    current: &Version,
) -> crate::UpdateRun {
    let mut checks = Vec::with_capacity(config.targets().len());
    for target in config.targets() {
        checks.push(check_one(source, store, target, current).await);
    }
    crate::UpdateRun::new(checks)
}

/// The credential for a target, or the outcome that replaces one.
///
/// A public target yields `Ok(None)` without consulting the store at all,
/// which is what keeps a public check working on a machine where no credential
/// was ever provisioned.
async fn token_for(
    store: &dyn crate::CredentialStore,
    target: &UpdateTarget,
) -> std::result::Result<Option<crate::Secret>, (crate::TargetState, Option<String>)> {
    use crate::{TargetState, Visibility};

    if target.visibility != Visibility::Private {
        return Ok(None);
    }
    let Some(entry) = target.credential_entry.as_deref() else {
        // Configuration validation already refuses this, so reaching it means
        // the target was built programmatically; report rather than panic.
        return Err((
            TargetState::Inaccessible,
            Some("no access to update source: the target names no credential entry".to_string()),
        ));
    };
    match store.get(entry).await {
        Ok(Some(secret)) => Ok(Some(secret)),
        // Named but not provisioned. This is an access problem, so the pinned
        // phrase stays the leading text, but the detail names the missing entry
        // and the step to take: an operator who never provisioned anything and
        // one whose request was refused have different problems, and telling
        // them apart is what makes the report actionable (FR-013).
        Ok(None) => Err((
            TargetState::Inaccessible,
            Some(format!(
                "no access to update source: no credential named '{entry}' is provisioned \
                 on this machine; {}",
                crate::credentials::provision_hint(entry)
            )),
        )),
        // The store itself could not be consulted: a different remedy, so a
        // different state.
        Err(e) => Err((TargetState::CredentialStoreUnavailable, Some(e.to_string()))),
    }
}

/// The installed version for a target, from its marker or the transitional
/// rule.
///
/// Used by both the check fan-out and the install orchestration so the two
/// cannot disagree about what is installed.
fn installed_version(target: &UpdateTarget, current: &Version) -> Option<String> {
    use crate::LEGACY_TARGET_NAME;

    match crate::marker::read_version(&target.install_dir, &target.name) {
        Some(version) => Some(version),
        None => {
            // Transitional rule: an installation that predates the marker must
            // not regress to "not installed". It applies only when the marker
            // is genuinely ABSENT: a corrupt or mismatched marker is not an
            // installation without a marker and must resolve to not_installed.
            // Scoped to the runtime target, because the running CLI's version
            // is only the runtime's version; claiming it for a foreign product
            // would be a false reading. Removal trigger: delete once every
            // install path writes a marker (see
            // `marker::legacy_runtime_present`).
            let absent = crate::marker::read_state(&target.install_dir).is_absent();
            if absent
                && target.name == LEGACY_TARGET_NAME
                && crate::marker::legacy_runtime_present(&target.install_dir)
            {
                Some(current.to_string())
            } else {
                None
            }
        }
    }
}

/// Checks a single target, converting every failure into a state instead of an
/// error, so the caller always gets a reportable result.
async fn check_one(
    source: &dyn crate::Releases,
    store: &dyn crate::CredentialStore,
    target: &UpdateTarget,
    current: &Version,
) -> crate::TargetCheck {
    use crate::{TargetCheck, TargetState};

    let installed = installed_version(target, current);

    // Access is resolved BEFORE the installed-version question, by operator
    // decision: a user without a credential cannot obtain the product at all,
    // so telling them it is "not installed" would bury the actionable problem.
    let token = match token_for(store, target).await {
        Ok(t) => t,
        Err((state, detail)) => {
            return TargetCheck::new(
                &target.name,
                state,
                installed,
                None,
                target.channel.to_string(),
                detail,
            )
        }
    };

    if installed.is_none() {
        // A target we have no installed version for still gets its source read
        // before we call it "not installed". Otherwise an access problem on
        // such a target would be reported as a clean run, and FR-015's four
        // outcomes would collapse into one.
        match source
            .list(crate::Access {
                target: &target.name,
                repository: &target.repository,
                token: token.as_ref(),
            })
            .await
        {
            Err(e) if matches!(classify_source_error(&e), TargetState::Inaccessible) => {
                return TargetCheck::new(
                    &target.name,
                    TargetState::Inaccessible,
                    None,
                    None,
                    target.channel.to_string(),
                    Some(e.to_string()),
                )
            }
            Err(e) => {
                return TargetCheck::new(
                    &target.name,
                    TargetState::NetworkError,
                    None,
                    None,
                    target.channel.to_string(),
                    Some(e.to_string()),
                )
            }
            // The source answered: the honest report is still "not installed",
            // because there is no local version to compare against.
            Ok(_) => {
                return TargetCheck::new(
                    &target.name,
                    TargetState::NotInstalled,
                    None,
                    None,
                    target.channel.to_string(),
                    None,
                )
            }
        }
    }

    let pattern = match target.resolved_asset_pattern() {
        Ok(p) => p,
        Err(e) => {
            return TargetCheck::new(
                &target.name,
                TargetState::InvalidArtifact,
                installed,
                None,
                target.channel.to_string(),
                Some(e.to_string()),
            )
        }
    };

    let releases = match source
        .list(crate::Access {
            target: &target.name,
            repository: &target.repository,
            token: token.as_ref(),
        })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let state = classify_source_error(&e);
            return TargetCheck::new(
                &target.name,
                state,
                installed,
                None,
                target.channel.to_string(),
                Some(e.to_string()),
            );
        }
    };

    let expected = match crate::arch::asset_suffix_for_arch(crate::arch::running_arch()) {
        Ok(s) => s,
        Err(e) => {
            return TargetCheck::new(
                &target.name,
                TargetState::InvalidArtifact,
                installed,
                None,
                target.channel.to_string(),
                Some(e.to_string()),
            )
        }
    };

    // The baseline for "is this newer?" is the installed version read from the
    // marker, not the running binary's version. Using `current` here would
    // offer a downgrade (or re-offer the installed version) for any target
    // whose installation differs from this CLI's own release.
    let baseline = match installed.as_deref() {
        Some(text) => match Version::parse(text) {
            Ok(version) => version,
            Err(e) => {
                return TargetCheck::new(
                    &target.name,
                    TargetState::InvalidArtifact,
                    installed,
                    None,
                    target.channel.to_string(),
                    Some(format!("install marker names an invalid version: {e}")),
                )
            }
        },
        None => {
            // The early branch above returns before this point, so this is a
            // defensive path rather than a reachable state.
            return TargetCheck::new(
                &target.name,
                TargetState::NotInstalled,
                None,
                None,
                target.channel.to_string(),
                None,
            );
        }
    };

    match crate::selection::select_detailed(
        &target.repository,
        target.channel,
        &baseline,
        &pattern,
        expected,
        releases,
    ) {
        crate::selection::Selection::Available(update) => {
            if update.checksum_url.is_none() {
                // A newer release is selected, but it cannot be verified. This is
                // not "up to date": the release exists and is unusable, which is
                // exactly the invalid_artifact family.
                TargetCheck::new(
                    &target.name,
                    TargetState::InvalidArtifact,
                    installed,
                    None,
                    target.channel.to_string(),
                    Some(
                        "release has no SHA-256 checksum asset; refusing to offer an \
                         unverified update"
                            .to_string(),
                    ),
                )
            } else {
                TargetCheck::new(
                    &target.name,
                    TargetState::UpdateAvailable,
                    installed,
                    Some(update.version.to_string()),
                    target.channel.to_string(),
                    None,
                )
            }
        }
        crate::selection::Selection::NoArtifact => TargetCheck::new(
            &target.name,
            TargetState::InvalidArtifact,
            installed,
            None,
            target.channel.to_string(),
            Some("release exists but has no usable artifact for this platform".to_string()),
        ),
        crate::selection::Selection::None => TargetCheck::new(
            &target.name,
            TargetState::UpToDate,
            installed,
            None,
            target.channel.to_string(),
            None,
        ),
    }
}

/// Maps a source error onto the outcome state it represents.
///
/// A refusal is `Inaccessible`, everything else is a connectivity problem. The
/// error is inspected by type rather than by matching its text, so a future
/// rewording cannot silently move a refusal into the transport bucket.
fn classify_source_error(error: &anyhow::Error) -> crate::TargetState {
    match error.downcast_ref::<crate::github::SourceError>() {
        Some(crate::github::SourceError::Inaccessible) => crate::TargetState::Inaccessible,
        _ => crate::TargetState::NetworkError,
    }
}

/// What happened to one target during an install run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallEntry {
    /// The update was downloaded, verified and installed.
    Installed { target: String, version: String },
    /// The target reported no newer permitted release.
    UpToDate { target: String },
    /// The target has no local installation, so there is nothing to update.
    NotInstalled { target: String },
    /// The target could not be installed; the error is actionable text.
    Failed { target: String, error: String },
}

impl InstallEntry {
    pub fn target(&self) -> &str {
        match self {
            Self::Installed { target, .. }
            | Self::UpToDate { target }
            | Self::NotInstalled { target }
            | Self::Failed { target, .. } => target,
        }
    }

    pub fn render(&self) -> String {
        match self {
            Self::Installed { target, version } => format!("{target}  installed {version}"),
            Self::UpToDate { target } => format!("{target}  up to date"),
            Self::NotInstalled { target } => format!("{target}  not installed"),
            Self::Failed { target, error } => format!("{target}  install failed: {error}"),
        }
    }
}

/// The result of one install run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstallRun {
    pub entries: Vec<InstallEntry>,
}

impl InstallRun {
    /// Whether any target failed. A no-op run (up to date or not installed) is
    /// not a failure.
    pub fn failed(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| matches!(entry, InstallEntry::Failed { .. }))
    }

    /// `0` for a run without failures, `2` when any target failed. Configuration
    /// and usage errors stay `1` and never reach this type.
    pub fn exit_status(&self) -> u8 {
        if self.failed() {
            2
        } else {
            0
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            out.push_str(&entry.render());
            out.push('\n');
        }
        out
    }
}

/// Installs the available update for every target the configuration selects.
///
/// A target is installed only when it actually reports an available update:
/// up-to-date and not-installed targets are reported and skipped, never
/// installed speculatively. Failures are recorded per target and are never
/// counted as success (FR-020, FR-020b).
pub async fn install_all(
    source: &dyn crate::Releases,
    downloader: &dyn crate::Downloader,
    installer: &dyn crate::Installer,
    store: &dyn crate::CredentialStore,
    config: &UpdateConfig,
    current: &Version,
) -> InstallRun {
    let mut entries = Vec::with_capacity(config.targets().len());
    for target in config.targets() {
        // A target with no local installation is not an update candidate. This
        // is checked before the credential so a not-installed private target is
        // skipped honestly instead of reporting an access failure for an
        // installation that does not exist.
        if installed_version(target, current).is_none() {
            entries.push(InstallEntry::NotInstalled {
                target: target.name.clone(),
            });
            continue;
        }

        let token = match token_for(store, target).await {
            Ok(token) => token,
            Err((_, detail)) => {
                entries.push(InstallEntry::Failed {
                    target: target.name.clone(),
                    error: detail.unwrap_or_else(|| "no access to update source".to_string()),
                });
                continue;
            }
        };

        match crate::select_for_target(source, target, token.as_ref(), current).await {
            Ok(crate::selection::Selection::Available(update)) => {
                if update.checksum_url.is_none() {
                    entries.push(InstallEntry::Failed {
                        target: target.name.clone(),
                        error: "release has no SHA-256 checksum asset; refusing to install \
                                an unverified archive"
                            .to_string(),
                    });
                    continue;
                }
                let result = match crate::download_and_verify_with(
                    downloader,
                    &update,
                    token.as_ref(),
                )
                .await
                {
                    Ok(bytes) => installer.install(target, &update.version, &bytes),
                    Err(e) => Err(e),
                };
                match result {
                    Ok(()) => entries.push(InstallEntry::Installed {
                        target: target.name.clone(),
                        version: update.version.to_string(),
                    }),
                    Err(e) => entries.push(InstallEntry::Failed {
                        target: target.name.clone(),
                        error: e.to_string(),
                    }),
                }
            }
            Ok(crate::selection::Selection::None) => entries.push(InstallEntry::UpToDate {
                target: target.name.clone(),
            }),
            Ok(crate::selection::Selection::NoArtifact) => entries.push(InstallEntry::Failed {
                target: target.name.clone(),
                error: "release has no usable artifact for this platform".to_string(),
            }),
            Err(e) => entries.push(InstallEntry::Failed {
                target: target.name.clone(),
                error: e.to_string(),
            }),
        }
    }
    InstallRun { entries }
}
