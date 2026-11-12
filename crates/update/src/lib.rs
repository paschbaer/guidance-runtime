//! Update support: release lookup, download, checksum verification and the
//! transactional installation of a new build.
//!
//! The I/O-heavy parts live behind the ports in [`ports`]; concrete adapters
//! are in [`github`] and [`install`]. The public free functions below are the
//! composition root: they wire the default adapters so existing call sites
//! keep working unchanged.

use anyhow::{bail, Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};

mod arch;
mod command;
mod config;
#[cfg(test)]
mod config_tests;
mod github;
mod install;
mod params;
mod ports;
mod report;
mod selection;
#[cfg(test)]
mod tests;

pub use arch::{
    asset_suffix_for, asset_suffix_for_arch, running_arch, Platform, KNOWN_ARCH_SUFFIXES,
};
pub use command::{
    check_all, check_update, check_update_with, current_version, set_or_get_channel,
    uses_legacy_output, UpdateOutcome,
};
pub use config::{ConfigFile, UpdateConfig, UpdateTarget, Visibility, LEGACY_TARGET_NAME};
pub use github::GitHubReleases;
pub use ports::{BoxFuture, Downloader, Installer, Releases};
pub use report::{render_check, render_run, TargetCheck, TargetState, UpdateRun};

pub use selection::{is_allowed, parse_checksum, select};

pub const DEFAULT_CONFIG: &str = r"C:\ProgramData\Guidance Runtime\config\updates.toml";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
}
impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Stable => "stable",
                Self::Beta => "beta",
            }
        )
    }
}
impl std::str::FromStr for Channel {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "stable" => Ok(Self::Stable),
            "beta" => Ok(Self::Beta),
            _ => bail!("channel must be stable or beta"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub prerelease: bool,
    pub draft: bool,
    pub assets: Vec<Asset>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AvailableUpdate {
    pub current: Version,
    pub version: Version,
    pub tag: String,
    pub channel: Channel,
    pub asset_name: String,
    pub download_url: String,
    pub checksum_url: Option<String>,
}
/// Looks up the newest update offered on the configured channel.
pub async fn check(target: &UpdateTarget, current: &Version) -> Result<Option<AvailableUpdate>> {
    let source = github::GitHubReleases::new();
    check_with(&source, target, current).await
}

/// Channel-aware release lookup against an arbitrary [`Releases`] port.
pub async fn check_with(
    source: &dyn Releases,
    target: &UpdateTarget,
    current: &Version,
) -> Result<Option<AvailableUpdate>> {
    let releases = source.list(&target.repository).await?;
    let pattern = target.resolved_asset_pattern()?;
    let expected = arch::asset_suffix_for_arch(arch::running_arch())?;
    Ok(selection::select(
        target.channel,
        current,
        &pattern,
        expected,
        releases,
    ))
}

/// Downloads the update asset and verifies it against the release checksum.
pub async fn download_and_verify(update: &AvailableUpdate) -> Result<Vec<u8>> {
    let downloader = github::HttpDownloader::new();
    download_and_verify_with(&downloader, update).await
}

/// Checksum-verified download against an arbitrary [`Downloader`] port.
pub async fn download_and_verify_with(
    downloader: &dyn Downloader,
    update: &AvailableUpdate,
) -> Result<Vec<u8>> {
    let bytes = downloader.get(&update.download_url).await?;
    let Some(url) = &update.checksum_url else {
        bail!("release has no SHA-256 asset; refusing update")
    };
    let text = String::from_utf8_lossy(&downloader.get(url).await?).into_owned();
    let expected =
        selection::parse_checksum(&text, &update.asset_name).context("empty checksum file")?;
    let actual = params::sha256_hex(&bytes);
    if actual != expected {
        bail!("SHA-256 mismatch: expected {expected}, got {actual}")
    }
    Ok(bytes)
}

/// Installs a downloaded archive with the default installer.
pub fn install_archive(target: &UpdateTarget, bytes: &[u8]) -> Result<()> {
    install::ZipInstaller.install(target, bytes)
}
