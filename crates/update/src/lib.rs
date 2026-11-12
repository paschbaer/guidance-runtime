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
use std::{
    fs,
    path::{Path, PathBuf},
};

mod arch;
mod command;
mod github;
mod install;
mod params;
mod ports;
mod selection;
#[cfg(test)]
mod tests;

pub use arch::{asset_suffix_for_arch, running_arch, KNOWN_ARCH_SUFFIXES};
pub use command::{check_update, current_version, set_or_get_channel, UpdateOutcome};
pub use ports::{BoxFuture, Downloader, Installer, Releases};

pub use selection::{is_allowed, parse_checksum, select};

pub const DEFAULT_CONFIG: &str = r"C:\ProgramData\Guidance Runtime\config\updates.toml";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub repository: String,
    #[serde(default = "stable")]
    pub channel: Channel,
    #[serde(default = "install_dir")]
    pub install_dir: PathBuf,
    #[serde(default = "service_name")]
    pub service_name: String,
    /// Optional asset suffix override. When absent the suffix is derived from
    /// the running architecture, so an installation never asks for another
    /// architecture's archive by default.
    #[serde(default)]
    pub asset_pattern: Option<String>,
}
fn stable() -> Channel {
    Channel::Stable
}
fn install_dir() -> PathBuf {
    PathBuf::from(r"C:\Program Files\Guidance Runtime")
}
fn service_name() -> String {
    "GuidanceRuntime".into()
}
impl UpdateConfig {
    /// The asset suffix to look for: the explicit override when configured,
    /// otherwise the suffix of the architecture this binary runs on.
    ///
    /// An explicitly configured but empty value is rejected rather than
    /// treated as "unset": an empty suffix matches every archive, which is
    /// exactly the architecture-blind behaviour this type avoids.
    pub fn resolved_asset_pattern(&self) -> Result<String> {
        let Some(raw) = self.asset_pattern.as_deref() else {
            return Ok(arch::asset_suffix_for_arch(arch::running_arch())?.to_string());
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

    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        Ok(toml::from_str(&text)?)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?
        }
        fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
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
pub async fn check(config: &UpdateConfig, current: &Version) -> Result<Option<AvailableUpdate>> {
    let source = github::GitHubReleases::new();
    check_with(&source, config, current).await
}

/// Channel-aware release lookup against an arbitrary [`Releases`] port.
pub async fn check_with(
    source: &dyn Releases,
    config: &UpdateConfig,
    current: &Version,
) -> Result<Option<AvailableUpdate>> {
    let releases = source.list(&config.repository).await?;
    let pattern = config.resolved_asset_pattern()?;
    let expected = arch::asset_suffix_for_arch(arch::running_arch())?;
    Ok(selection::select(
        config.channel,
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
pub fn install_archive(config: &UpdateConfig, bytes: &[u8]) -> Result<()> {
    install::ZipInstaller.install(config, bytes)
}
