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
pub async fn check_update(target: &UpdateTarget, current: &Version) -> Result<UpdateOutcome> {
    match crate::check(target, current).await? {
        Some(update) => Ok(UpdateOutcome::Available(update)),
        None => Ok(UpdateOutcome::UpToDate(target.channel)),
    }
}

/// The same outcome against an arbitrary [`crate::Releases`] port.
///
/// This is the historical single-target path, kept separately so the legacy
/// equivalence test can pin its exact wording without reaching for the network.
pub async fn check_update_with(
    source: &dyn crate::Releases,
    target: &UpdateTarget,
    current: &Version,
) -> Result<UpdateOutcome> {
    match crate::check_with(source, target, current).await? {
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
    config: &UpdateConfig,
    current: &Version,
) -> crate::UpdateRun {
    let mut checks = Vec::with_capacity(config.targets().len());
    for target in config.targets() {
        checks.push(check_one(source, target, current).await);
    }
    crate::UpdateRun::new(checks)
}

/// Checks a single target, converting every failure into a state instead of an
/// error, so the caller always gets a reportable result.
async fn check_one(
    source: &dyn crate::Releases,
    target: &UpdateTarget,
    current: &Version,
) -> crate::TargetCheck {
    use crate::{TargetCheck, TargetState, Visibility, LEGACY_TARGET_NAME};

    // Only the target this binary belongs to can report the running version as
    // its installed version. Any other product runs a different release, and
    // claiming this crate's version for it would be a false reading of FR-007.
    // Until the per-target version marker lands (T028), a foreign target is
    // reported as not installed, which is the honest answer.
    let installed = if target.name == LEGACY_TARGET_NAME {
        Some(current.to_string())
    } else {
        None
    };

    // No credential path exists yet (T022 supplies it), so a private target
    // cannot be read at all. This is checked BEFORE the installed-version
    // question: a machine that cannot reach the source needs to hear that, and
    // reporting "not installed" first would bury the actionable problem.
    // Superseded by T022.
    if target.visibility == Visibility::Private {
        return TargetCheck::new(
            &target.name,
            TargetState::Inaccessible,
            installed,
            None,
            target.channel.to_string(),
            Some("no access to update source".to_string()),
        );
    }

    if installed.is_none() {
        return TargetCheck::new(
            &target.name,
            TargetState::NotInstalled,
            None,
            None,
            target.channel.to_string(),
            None,
        );
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

    let releases = match source.list(&target.repository).await {
        Ok(r) => r,
        Err(e) => {
            return TargetCheck::new(
                &target.name,
                TargetState::NetworkError,
                installed,
                None,
                target.channel.to_string(),
                Some(e.to_string()),
            )
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

    match crate::selection::select(target.channel, current, &pattern, expected, releases) {
        Some(update) => TargetCheck::new(
            &target.name,
            TargetState::UpdateAvailable,
            installed,
            Some(update.version.to_string()),
            target.channel.to_string(),
            None,
        ),
        None => TargetCheck::new(
            &target.name,
            TargetState::UpToDate,
            installed,
            None,
            target.channel.to_string(),
            None,
        ),
    }
}
