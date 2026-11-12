//! Pure release-selection logic: channel filtering, version selection and
//! checksum parsing. No I/O lives here, so every rule is unit-testable.

use crate::{AvailableUpdate, Channel, Release};
use semver::Version;

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

/// Picks the highest version above `current` that is allowed on the channel
/// and ships a matching asset.
pub fn select(
    channel: Channel,
    current: &Version,
    repo_asset_pattern: &str,
    releases: Vec<Release>,
) -> Option<AvailableUpdate> {
    let mut candidates: Vec<AvailableUpdate> = releases
        .into_iter()
        .filter(|r| !r.draft)
        .filter_map(|release| {
            let raw = release.tag_name.trim_start_matches('v');
            let version = Version::parse(raw).ok()?;
            if !is_allowed(channel, &release, &version) || version <= *current {
                return None;
            }
            let asset = release
                .assets
                .iter()
                .find(|a| a.name.contains(repo_asset_pattern) && a.name.ends_with(".zip"))?;
            let checksum_url = release
                .assets
                .iter()
                .find(|a| a.name.to_ascii_lowercase().contains("sha256"))
                .map(|a| a.browser_download_url.clone());
            Some(AvailableUpdate {
                current: current.clone(),
                version,
                tag: release.tag_name,
                channel,
                asset_name: asset.name.clone(),
                download_url: asset.browser_download_url.clone(),
                checksum_url,
            })
        })
        .collect();
    candidates.sort_by(|a, b| b.version.cmp(&a.version));
    candidates.into_iter().next()
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
