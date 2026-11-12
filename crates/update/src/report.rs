//! Outcome model for a multi-target update check.
//!
//! A run inspects every configured target and produces exactly one
//! [`TargetCheck`] per target, regardless of whether that target could be
//! reached. [`TargetState`] is deliberately a closed enum: automation must be
//! able to tell an access problem from "up to date" without parsing prose, so
//! an open-ended string would defeat the purpose.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Every outcome a single target can report.
///
/// The set is closed by design. Adding a member is a change to the structured
/// output contract and must bump [`UpdateRun::spec_version`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetState {
    /// A permitted release newer than the installed version exists.
    UpdateAvailable,
    /// The source was read and nothing newer is permitted on the channel.
    UpToDate,
    /// No usable version marker and no recognized legacy installation.
    NotInstalled,
    /// The source refused the read: not found, unauthorized, forbidden, or a
    /// missing credential entry. Deliberately does not distinguish a
    /// non-existent repository from an unauthorized identity.
    Inaccessible,
    /// The source was not reached; transport failure.
    NetworkError,
    /// The release exists but no permitted artifact for this platform, or the
    /// checksum asset is absent.
    InvalidArtifact,
    /// The platform's secure credential store could not be consulted at all.
    CredentialStoreUnavailable,
}

impl TargetState {
    /// Whether this state means the run produced a usable answer for the
    /// target. Used to derive [`UpdateRun::exit_status`].
    pub fn is_failure(self) -> bool {
        matches!(
            self,
            Self::Inaccessible
                | Self::NetworkError
                | Self::InvalidArtifact
                | Self::CredentialStoreUnavailable
        )
    }
}

impl fmt::Display for TargetState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::UpdateAvailable => "update_available",
            Self::UpToDate => "up_to_date",
            Self::NotInstalled => "not_installed",
            Self::Inaccessible => "inaccessible",
            Self::NetworkError => "network_error",
            Self::InvalidArtifact => "invalid_artifact",
            Self::CredentialStoreUnavailable => "credential_store_unavailable",
        };
        f.write_str(text)
    }
}

/// The outcome of inspecting one target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetCheck {
    /// The configured target name.
    pub target: String,
    pub state: TargetState,
    /// The version read from the target's marker, or `None` when no installed
    /// version is known.
    ///
    /// `NotInstalled` always implies `None`. The converse does not hold and must
    /// not be assumed: an access or transport problem is resolved before the
    /// installed-version question (see `check_one`), so a failing target may
    /// carry `None` here without being "not installed".
    pub installed_version: Option<String>,
    /// `None` unless `state` is [`TargetState::UpdateAvailable`].
    pub offered_version: Option<String>,
    pub channel: String,
    /// Human-readable cause. Must never carry credential material.
    pub detail: Option<String>,
}

impl TargetCheck {
    /// Builds a check result and rejects combinations the contract forbids.
    ///
    /// The two invariants are cheaper to enforce at construction than to
    /// discover in rendered output: a "not installed" target that still names
    /// a version, or an offered version that did not come with an
    /// `UpdateAvailable` state, is always a caller bug.
    pub fn new(
        target: impl Into<String>,
        state: TargetState,
        installed_version: Option<String>,
        offered_version: Option<String>,
        channel: impl Into<String>,
        detail: Option<String>,
    ) -> Self {
        let installed_version = match state {
            TargetState::NotInstalled => None,
            _ => installed_version,
        };
        let offered_version = match state {
            TargetState::UpdateAvailable => offered_version,
            _ => None,
        };
        Self {
            target: target.into(),
            state,
            installed_version,
            offered_version,
            channel: channel.into(),
            detail,
        }
    }
}

/// The aggregate result of one check run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateRun {
    /// Schema version of the structured output. Starts at 1.
    pub spec_version: u32,
    /// One entry per configured target, in configuration order.
    pub targets: Vec<TargetCheck>,
}

impl UpdateRun {
    pub const SPEC_VERSION: u32 = 1;

    pub fn new(targets: Vec<TargetCheck>) -> Self {
        Self {
            spec_version: Self::SPEC_VERSION,
            targets,
        }
    }

    /// `0` when every target produced a usable answer, `2` when at least one
    /// failed. A configuration error never reaches this type; it is `1`.
    pub fn exit_status(&self) -> u8 {
        if self.targets.iter().any(|t| t.state.is_failure()) {
            2
        } else {
            0
        }
    }
}

/// The one wording the specification pins for an access problem.
///
/// Both the state's rendering and callers that add an actionable suffix build
/// on this constant, so the phrase cannot drift in one place and not the other.
pub const PHRASE: &str = "no access to update source";

/// Renders one target's outcome as a single human-readable line.
///
/// The phrase `no access to update source` is verbatim for [
/// `TargetState::Inaccessible`]: it is the one wording the specification pins,
/// and it must be identical whether the repository is absent or the identity
/// is unauthorized.
pub fn render_check(check: &TargetCheck) -> String {
    let name = &check.target;
    let channel = &check.channel;
    match check.state {
        TargetState::UpdateAvailable => {
            let installed = check.installed_version.as_deref().unwrap_or("?");
            let offered = check.offered_version.as_deref().unwrap_or("?");
            format!("{name}  update available: {installed} -> {offered} ({channel})")
        }
        TargetState::UpToDate => {
            let installed = check.installed_version.as_deref().unwrap_or("?");
            format!("{name}  up to date ({installed}, {channel})")
        }
        TargetState::NotInstalled => format!("{name}  not installed"),
        TargetState::Inaccessible => {
            // The pinned phrase leads; anything a caller added after it is the
            // actionable part (a provisioning step, a named missing entry) and
            // must be shown rather than swallowed.
            let mut line = format!("{name}  {PHRASE}");
            if let Some(detail) = &check.detail {
                let extra = detail
                    .strip_prefix(PHRASE)
                    .map(|rest| rest.trim_start_matches([':', ' ']))
                    .unwrap_or(detail.as_str());
                if !extra.is_empty() {
                    line.push_str(&format!("\n  {extra}"));
                }
            }
            line
        }
        TargetState::NetworkError => {
            // The adapter's own text already leads with "cannot reach update
            // source"; repeating it would replace the informative part of the
            // line with the same phrase twice.
            let detail = match check.detail.as_deref() {
                Some("cannot reach update source") => "connection failed",
                Some(raw) => raw
                    .strip_prefix("cannot reach update source")
                    .map(|rest| rest.trim_start_matches([':', ' ']))
                    .filter(|rest| !rest.is_empty())
                    .unwrap_or(raw),
                None => "connection timed out",
            };
            format!("{name}  cannot reach update source (network error: {detail})")
        }
        TargetState::InvalidArtifact => {
            let detail = check
                .detail
                .as_deref()
                .unwrap_or("no usable artifact for this platform");
            format!("{name}  release has no usable artifact for this platform ({detail})")
        }
        TargetState::CredentialStoreUnavailable => {
            format!("{name}  credential store unavailable on this system")
        }
    }
}

/// The structured document emitted by `--format json`.
///
/// This is deliberately a separate output type rather than `Serialize` on
/// [`UpdateRun`]: the contract requires `checked_at` and `exit_status` as
/// fields, while `UpdateRun` knows no clock and treats the exit status as a
/// method. Deriving `Serialize` onto the run would either omit both fields or
/// force the run to carry a timestamp it does not own.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JsonDocument {
    /// Schema version. Mirrors [`UpdateRun::SPEC_VERSION`].
    pub spec_version: u32,
    /// RFC3339 timestamp of the run.
    pub checked_at: String,
    /// The run's exit status, as a field so a producer and a consumer agree.
    pub exit_status: u8,
    /// One entry per selected target, in configuration order.
    pub targets: Vec<TargetCheck>,
}

impl JsonDocument {
    pub fn from_run(run: &UpdateRun, checked_at: impl Into<String>) -> Self {
        Self {
            spec_version: run.spec_version,
            checked_at: checked_at.into(),
            exit_status: run.exit_status(),
            targets: run.targets.clone(),
        }
    }
}

/// Serializes a run as the structured document.
///
/// `checked_at` is passed in rather than read from the clock so a test can pin
/// the whole document; callers that want the current time use
/// [`render_json_now`].
pub fn render_json(run: &UpdateRun, checked_at: &str) -> anyhow::Result<String> {
    let document = JsonDocument::from_run(run, checked_at);
    serde_json::to_string_pretty(&document).map_err(Into::into)
}

/// Serializes a run with the current UTC time as `checked_at`.
pub fn render_json_now(run: &UpdateRun) -> anyhow::Result<String> {
    use time::format_description::well_known::Rfc3339;

    let checked_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| anyhow::anyhow!("format checked_at: {e}"))?;
    render_json(run, &checked_at)
}

/// Renders a whole run, one line per target, in configuration order.
pub fn render_run(run: &UpdateRun) -> String {
    let mut out = String::new();
    for check in &run.targets {
        out.push_str(&render_check(check));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(state: TargetState, installed: Option<&str>, offered: Option<&str>) -> TargetCheck {
        TargetCheck::new(
            "guidance",
            state,
            installed.map(str::to_string),
            offered.map(str::to_string),
            "stable",
            None,
        )
    }

    /// A target that is not installed must not claim a version; a version
    /// there would make "not installed" indistinguishable from "up to date".
    #[test]
    fn not_installed_drops_any_supplied_version() {
        let c = check(TargetState::NotInstalled, Some("1.0.0"), None);
        assert_eq!(c.installed_version, None);
    }

    /// An offered version is meaningful only for an available update.
    #[test]
    fn only_update_available_keeps_the_offered_version() {
        let avail = check(TargetState::UpdateAvailable, Some("1.0.0"), Some("2.0.0"));
        assert_eq!(avail.offered_version.as_deref(), Some("2.0.0"));

        let up = check(TargetState::UpToDate, Some("1.0.0"), Some("2.0.0"));
        assert_eq!(up.offered_version, None);
    }

    /// The wire form is snake_case so automation can match on it.
    #[test]
    fn state_serializes_as_lowercase_snake_case() {
        for (state, expected) in [
            (TargetState::UpdateAvailable, "\"update_available\""),
            (TargetState::UpToDate, "\"up_to_date\""),
            (TargetState::NotInstalled, "\"not_installed\""),
            (TargetState::Inaccessible, "\"inaccessible\""),
            (TargetState::NetworkError, "\"network_error\""),
            (TargetState::InvalidArtifact, "\"invalid_artifact\""),
            (
                TargetState::CredentialStoreUnavailable,
                "\"credential_store_unavailable\"",
            ),
        ] {
            assert_eq!(serde_json::to_string(&state).unwrap(), expected);
        }
    }

    /// A run reports exactly as many checks as it was given, and a failure in
    /// one target does not hide the others.
    #[test]
    fn run_keeps_every_target_and_derives_the_failure_status() {
        let run = UpdateRun::new(vec![
            check(TargetState::UpToDate, Some("1.0.0"), None),
            check(TargetState::Inaccessible, Some("2.0.0"), None),
            check(TargetState::NotInstalled, None, None),
        ]);
        assert_eq!(run.targets.len(), 3);
        assert_eq!(run.exit_status(), 2);
    }

    #[test]
    fn a_clean_run_exits_zero() {
        let run = UpdateRun::new(vec![check(TargetState::UpToDate, Some("1.0.0"), None)]);
        assert_eq!(run.exit_status(), 0);
    }
}
