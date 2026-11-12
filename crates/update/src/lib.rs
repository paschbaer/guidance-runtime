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
mod credentials;
mod github;
mod install;
pub mod marker;
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
    check_all, check_from_outcome, check_update, check_update_with, current_version, install_all,
    set_or_get_channel, uses_legacy_output, InstallEntry, InstallRun, UpdateOutcome,
};
pub use config::{ConfigFile, UpdateConfig, UpdateTarget, Visibility, LEGACY_TARGET_NAME};
pub use credentials::{
    command_for, provision_hint, CredentialCommand, PlatformCredentialStore, SERVICE,
};
pub use github::GitHubReleases;
pub use github::HttpDownloader;
pub use github::{classify_status, SourceError};
pub use ports::{
    Access, AuthenticatedAsset, BoxFuture, CredentialStore, CredentialStoreError, Downloader,
    Installer, Releases, Secret,
};
pub use report::{
    render_check, render_json, render_json_now, render_run, JsonDocument, TargetCheck, TargetState,
    UpdateRun,
};

pub use selection::{is_allowed, parse_checksum, select};

/// Installs the available updates with the default HTTP downloader and archive
/// installer. The caller selects the targets by narrowing the configuration
/// first.
pub async fn install_all_default(
    source: &dyn Releases,
    store: &dyn CredentialStore,
    config: &UpdateConfig,
    current: &Version,
) -> InstallRun {
    let downloader = github::HttpDownloader::new();
    command::install_all(
        source,
        &downloader,
        &install::ZipInstaller,
        store,
        config,
        current,
    )
    .await
}

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
    /// The numeric asset id, needed to build the authenticated API address a
    /// private source requires. `browser_download_url` is served without
    /// authentication and 404s for a private repository, so it is deliberately
    /// absent: there is no field a caller could mistakenly use instead.
    pub id: u64,
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

/// Builds the address an asset must be fetched from.
///
/// The authenticated API endpoint, never `browser_download_url`: the latter
/// requires no credential, which is precisely why it is unusable for a private
/// source — it answers 404 to an authorized caller.
pub fn asset_endpoint(repository: &str, asset_id: u64) -> String {
    format!("https://api.github.com/repos/{repository}/releases/assets/{asset_id}")
}
/// Looks up the newest update offered on the configured channel.
pub async fn check(
    target: &UpdateTarget,
    token: Option<&Secret>,
    current: &Version,
) -> Result<Option<AvailableUpdate>> {
    let source = github::GitHubReleases::new();
    check_with(&source, target, token, current).await
}

/// The detailed selection for a target, used by the check and install paths so
/// an undeliverable release is visible instead of collapsing to "no update".
pub(crate) async fn select_for_target(
    source: &dyn Releases,
    target: &UpdateTarget,
    token: Option<&Secret>,
    current: &Version,
) -> Result<crate::selection::Selection> {
    let releases = source
        .list(ports::Access {
            target: &target.name,
            repository: &target.repository,
            token,
        })
        .await?;
    let pattern = target.resolved_asset_pattern()?;
    let expected = arch::asset_suffix_for_arch(arch::running_arch())?;
    // A marker is the truer installed version when one exists. A pre-existing
    // installation has no marker, so this falls back to the running version and
    // SC-001's behaviour is unchanged for every machine that predates the
    // feature; a marker can only exist after this updater installed something.
    let baseline = marker::read_version(&target.install_dir, &target.name)
        .and_then(|version| Version::parse(&version).ok())
        .unwrap_or_else(|| current.clone());
    Ok(selection::select_detailed(
        &target.repository,
        target.channel,
        &baseline,
        &pattern,
        expected,
        releases,
    ))
}

/// Channel-aware release lookup against an arbitrary [`Releases`] port.
///
/// The historical single-target path keeps its permissive view: a release with
/// a usable archive but no checksum asset is still offered here, and the
/// download step refuses it. The multi-target check reports that case as
/// `invalid_artifact` because it has a state for it.
pub async fn check_with(
    source: &dyn Releases,
    target: &UpdateTarget,
    token: Option<&Secret>,
    current: &Version,
) -> Result<Option<AvailableUpdate>> {
    match select_for_target(source, target, token, current).await? {
        selection::Selection::Available(update) => Ok(Some(update)),
        selection::Selection::None | selection::Selection::NoArtifact => Ok(None),
    }
}

/// Downloads the update asset and verifies it against the release checksum.
pub async fn download_and_verify(
    update: &AvailableUpdate,
    token: Option<&Secret>,
) -> Result<Vec<u8>> {
    let downloader = github::HttpDownloader::new();
    download_and_verify_with(&downloader, update, token).await
}

/// Checksum-verified download against an arbitrary [`Downloader`] port.
///
/// The archive and its checksum both travel as an
/// [`ports::AuthenticatedAsset`], so they cannot diverge into one authenticated
/// request and one public one.
pub async fn download_and_verify_with(
    downloader: &dyn Downloader,
    update: &AvailableUpdate,
    token: Option<&Secret>,
) -> Result<Vec<u8>> {
    let archive = ports::AuthenticatedAsset {
        url: update.download_url.clone(),
        token: token.cloned(),
    };
    let bytes = downloader.get(&archive).await?;
    let Some(url) = &update.checksum_url else {
        bail!("release has no SHA-256 asset; refusing update")
    };
    let checksum = ports::AuthenticatedAsset {
        url: url.clone(),
        token: token.cloned(),
    };
    let text = String::from_utf8_lossy(&downloader.get(&checksum).await?).into_owned();
    let expected =
        selection::parse_checksum(&text, &update.asset_name).context("empty checksum file")?;
    let actual = params::sha256_hex(&bytes);
    if actual != expected {
        bail!("SHA-256 mismatch: expected {expected}, got {actual}")
    }
    Ok(bytes)
}

/// Installs a downloaded archive with the default installer.
pub fn install_archive(target: &UpdateTarget, version: &Version, bytes: &[u8]) -> Result<()> {
    install::ZipInstaller.install(target, version, bytes)
}
