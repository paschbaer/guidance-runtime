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
    use crate::SERVICE;
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
                 on this machine; provision it as '{SERVICE}:{entry}'"
            )),
        )),
        // The store itself could not be consulted: a different remedy, so a
        // different state.
        Err(e) => Err((TargetState::CredentialStoreUnavailable, Some(e.to_string()))),
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
    use crate::{TargetCheck, TargetState, LEGACY_TARGET_NAME};

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

    match crate::selection::select(
        &target.repository,
        target.channel,
        current,
        &pattern,
        expected,
        releases,
    ) {
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
