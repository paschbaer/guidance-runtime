//! Shared command bodies for update operations.
//!
//! Both the operator CLI (`guidance update …`) and the standalone updater
//! binary use these functions. Only rendering differs between the two; the
//! *install* paths stay separate by design: the CLI spawns `guidance-updater`
//! with `--wait-pid` so it can exit before its own binaries are replaced,
//! while the updater performs the actual install.

use crate::{AvailableUpdate, Channel, UpdateConfig};
use anyhow::{Context, Result};
use semver::Version;
use std::path::Path;

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

/// Sets the channel on disk when `channel` is given, otherwise reports the
/// currently configured one.
pub fn set_or_get_channel(config_path: &Path, channel: Option<Channel>) -> Result<String> {
    let mut cfg = UpdateConfig::load(config_path)?;
    if let Some(c) = channel {
        cfg.channel = c;
        cfg.save(config_path)?;
        Ok(format!("Update channel set to {c}"))
    } else {
        Ok(cfg.channel.to_string())
    }
}

/// Checks for an update and returns a renderable outcome.
pub async fn check_update(config: &UpdateConfig, current: &Version) -> Result<UpdateOutcome> {
    match crate::check(config, current).await? {
        Some(update) => Ok(UpdateOutcome::Available(update)),
        None => Ok(UpdateOutcome::UpToDate(config.channel)),
    }
}

/// Parses the running crate version, used when no explicit version is given.
pub fn current_version() -> Result<Version> {
    Version::parse(env!("CARGO_PKG_VERSION")).context("parse crate version")
}
