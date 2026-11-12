//! Pure release-selection logic: channel filtering, version selection and
//! checksum parsing. No I/O lives here, so every rule is unit-testable.

use crate::arch::KNOWN_ARCH_SUFFIXES;
use crate::{AvailableUpdate, Channel, Release};
use semver::Version;

/// Decides whether an asset name belongs to the expected architecture.
///
/// `expected` is the architecture suffix this installation requires (for
/// example `windows-x64.zip`); `pattern` is the name fragment the operator
/// searches for. The pattern narrows the search and can never widen it, so no
/// configuration can pull in another architecture's archive. A pattern looser
/// than the expected suffix (for example `zip`) narrows nothing; it is
/// accepted but has no effect.
///
/// Defence in depth, not a proof: the name must end with `expected` AND its
/// known suffix (if any) must equal `expected`. Architectures outside
/// `KNOWN_ARCH_SUFFIXES` are not covered — the table is hand-maintained and
/// must grow with the release workflow.
fn belongs_to_expected_arch(name: &str, pattern: &str, expected: &str) -> bool {
    if !name.ends_with(pattern) || !name.ends_with(expected) {
        return false;
    }
    let name_suffix = KNOWN_ARCH_SUFFIXES
        .iter()
        .find(|known| name.ends_with(*known));
    matches!(name_suffix, Some(suffix) if *suffix == expected)
}

/// Decides whether a release is offered on the given channel.
///
/// Rules:
/// - Stable accepts published, non-prerelease versions only.
/// - Beta additionally accepts prereleases whose identifier starts with
///   `beta.` — this is what separates release candidates from other
///   prerelease labels such as `alpha.` or `rc.`.
pub fn is_allowed(channel: Channel, release: &Release, version: &Version) -> bool {
    let has_beta_pre = !version.pre.is_empty() && version.pre.as_str().starts_with("beta.");
    let is_plain_release = !release.prerelease && version.pre.is_empty();
    match channel {
        Channel::Stable => is_plain_release,
        Channel::Beta => is_plain_release || has_beta_pre,
    }
}

/// The outcome of choosing a release, including the reasons that are not an
/// update.
///
/// [`select`] collapses this to `Option` for callers that only need an update;
/// the check and install paths use the detailed form so a release that exists
/// but cannot be delivered is reported as `invalid_artifact` rather than as
/// "up to date".
#[derive(Debug, Clone, PartialEq)]
pub enum Selection {
    /// A newer release with a matching artifact (and its checksum asset when
    /// present) is available.
    Available(AvailableUpdate),
    /// No newer release is offered on the channel.
    None,
    /// A newer release exists, but none carries an asset for this platform and
    /// architecture.
    NoArtifact,
}

/// Picks the highest version above `current` that is allowed on the channel
/// and ships an asset for `expected_arch`, distinguishing "nothing newer" from
/// "newer release without a usable artifact".
pub fn select_detailed(
    repository: &str,
    channel: Channel,
    current: &Version,
    asset_pattern: &str,
    expected_arch: &str,
    releases: Vec<Release>,
) -> Selection {
    let mut saw_candidate = false;
    let mut best: Option<(Version, AvailableUpdate)> = None;

    for release in releases {
        if release.draft {
            continue;
        }
        let raw = release.tag_name.trim_start_matches('v');
        let Ok(version) = Version::parse(raw) else {
            continue;
        };
        if !is_allowed(channel, &release, &version) || version <= *current {
            continue;
        }
        saw_candidate = true;
        let Some(asset) = release
            .assets
            .iter()
            .find(|a| belongs_to_expected_arch(&a.name, asset_pattern, expected_arch))
        else {
            continue;
        };
        let checksum_url = release
            .assets
            .iter()
            .find(|a| a.name.to_ascii_lowercase().contains("sha256"))
            .map(|a| crate::asset_endpoint(repository, a.id));
        let candidate = AvailableUpdate {
            current: current.clone(),
            version: version.clone(),
            tag: release.tag_name,
            channel,
            asset_name: asset.name.clone(),
            download_url: crate::asset_endpoint(repository, asset.id),
            checksum_url,
        };
        if best
            .as_ref()
            .is_none_or(|(best_version, _)| version > *best_version)
        {
            best = Some((version, candidate));
        }
    }

    match best {
        Some((_, update)) => Selection::Available(update),
        None if saw_candidate => Selection::NoArtifact,
        None => Selection::None,
    }
}

/// Picks the highest available update, or `None` when there is nothing to
/// install. This is the permissive legacy view; the check and install paths use
/// [`select_detailed`] so an undeliverable release is visible.
pub fn select(
    repository: &str,
    channel: Channel,
    current: &Version,
    asset_pattern: &str,
    expected_arch: &str,
    releases: Vec<Release>,
) -> Option<AvailableUpdate> {
    match select_detailed(
        repository,
        channel,
        current,
        asset_pattern,
        expected_arch,
        releases,
    ) {
        Selection::Available(update) => Some(update),
        Selection::None | Selection::NoArtifact => None,
    }
}

/// Extracts the lowercase hex digest for `asset_name` from a checksum file.
/// Prefers the line naming the asset and falls back to the first line, which
/// covers the single-asset `shasum` output format.
pub fn parse_checksum(text: &str, asset_name: &str) -> Option<String> {
    text.lines()
        .find(|line| line.contains(asset_name))
        .or_else(|| text.lines().next())
        .and_then(|line| line.split_whitespace().next())
        .map(str::to_ascii_lowercase)
}
