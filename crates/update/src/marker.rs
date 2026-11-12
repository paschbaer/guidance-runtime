//! The per-target version marker.
//!
//! Each target's install directory holds `install.json` stating which product
//! and version are installed there. The marker is the authoritative answer to
//! "what is installed here": the target name is part of the record, so two
//! targets on the same machine cannot read each other's version (FR-009c).
//!
//! The read result distinguishes an *absent* marker from an *unreadable* or
//! target-mismatched one. Both resolve to "not installed", but only genuine
//! absence may fall back to the transitional legacy rule — a corrupt marker is
//! not an installation without a marker.
//!
//! The marker is written only after a successful install (see `install.rs`), so
//! a failed install cannot claim the new version.

use anyhow::{Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

/// Name of the marker file inside a target's install directory.
pub const MARKER_FILE: &str = "install.json";

/// The on-disk marker record.
///
/// Unknown fields are tolerated so a later minor schema addition (for example a
/// build identifier) does not make every existing marker unreadable; the format
/// was chosen for precisely that property. A missing required field is still
/// unreadable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    /// The target that owns this installation.
    pub target: String,
    /// The installed version, as published by that target.
    pub version: String,
    /// RFC3339 timestamp of the successful installation.
    pub installed_at: String,
}

/// What a marker read found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerRead {
    /// No marker file exists.
    Absent,
    /// A marker file exists but cannot be understood, or names an invalid
    /// target/version.
    Unreadable,
    /// A usable marker.
    Present(Marker),
}

impl MarkerRead {
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

/// Reads and validates the marker.
pub fn read_state(install_dir: &Path) -> MarkerRead {
    let path = install_dir.join(MARKER_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return MarkerRead::Absent,
        // A directory, a permission problem or an I/O error is not "absent":
        // the marker exists in some form and must not trigger the legacy
        // fallback.
        Err(_) => return MarkerRead::Unreadable,
    };
    match serde_json::from_str::<Marker>(&text) {
        Ok(marker) if marker_is_valid(&marker) => MarkerRead::Present(marker),
        _ => MarkerRead::Unreadable,
    }
}

fn marker_is_valid(marker: &Marker) -> bool {
    !marker.target.trim().is_empty()
        && !marker.installed_at.trim().is_empty()
        && Version::parse(&marker.version).is_ok()
}

/// Reads the marker, if one exists and can be understood.
pub fn read(install_dir: &Path) -> Option<Marker> {
    match read_state(install_dir) {
        MarkerRead::Present(marker) => Some(marker),
        MarkerRead::Absent | MarkerRead::Unreadable => None,
    }
}

/// The installed version for `target`, or `None` when there is no marker, it is
/// unreadable, or it belongs to a different target (FR-009b, FR-009c).
pub fn read_version(install_dir: &Path, target: &str) -> Option<String> {
    match read_state(install_dir) {
        MarkerRead::Present(marker) if marker.target == target => Some(marker.version),
        _ => None,
    }
}

/// Writes the marker for a successful installation.
///
/// The write goes through a temporary file so a crash cannot leave a truncated
/// marker: readers see either the previous marker or the new one, never half of
/// one. Callers must invoke this only after the replacement succeeded; writing
/// it earlier would let a crash-after-replacement failure claim the new version.
pub fn write(install_dir: &Path, target: &str, version: &str) -> Result<()> {
    use time::format_description::well_known::Rfc3339;

    if target.trim().is_empty() || Version::parse(version).is_err() {
        anyhow::bail!(
            "refusing to write an install marker with an empty target or non-semver version"
        );
    }
    fs::create_dir_all(install_dir)
        .with_context(|| format!("create install directory {}", install_dir.display()))?;
    let installed_at = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| anyhow::anyhow!("format installed_at: {e}"))?;
    let marker = Marker {
        target: target.to_string(),
        version: version.to_string(),
        installed_at,
    };
    let text = serde_json::to_string_pretty(&marker).context("serialize install marker")?;

    let final_path = install_dir.join(MARKER_FILE);
    let temp_path = install_dir.join(format!("{MARKER_FILE}.tmp"));
    fs::write(&temp_path, text).with_context(|| format!("write {}", temp_path.display()))?;
    match fs::rename(&temp_path, &final_path) {
        Ok(()) => Ok(()),
        Err(first_error) => {
            // Windows refuses to replace an existing file with `rename` on
            // some versions. Remove the old marker and retry; the window is
            // small and the marker is written last, so a crash here leaves no
            // marker rather than a false version claim.
            if let Err(remove_error) = fs::remove_file(&final_path) {
                let _ = fs::remove_file(&temp_path);
                return Err(remove_error).with_context(|| {
                    format!(
                        "replace {} (rename failed with: {first_error})",
                        final_path.display()
                    )
                });
            }
            fs::rename(&temp_path, &final_path)
                .with_context(|| format!("rename into {}", final_path.display()))
        }
    }
}

/// Whether an installation from before the marker existed is present.
///
/// Transitional rule, required so the existing runtime installation does not
/// become "not installed" merely because it predates this marker. It applies
/// only to `guidance-runtime`, the one target whose installed version the
/// running binary actually knows; claiming that version for a foreign product
/// would be a false reading (FR-007).
///
/// The trigger is the service executable, matching the documented legacy shape
/// (`no marker but guidance-service.exe present`). A directory holding only a
/// stale tray or updater binary is not an installation.
///
/// **Removal trigger:** delete this function (and its call in `check_one`) once
/// every supported install path writes a marker. Today that means once no
/// installation older than the marker-bearing release remains in use; the
/// installer writes `install.json` since this feature landed.
pub fn legacy_runtime_present(install_dir: &Path) -> bool {
    service_executable_path(install_dir).is_file()
}

fn service_executable_path(install_dir: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        install_dir.join("guidance-service.exe")
    }
    #[cfg(not(windows))]
    {
        install_dir.join("guidance-service")
    }
}

/// The marker path for a target, for diagnostics and tests.
pub fn marker_path(install_dir: &Path) -> PathBuf {
    install_dir.join(MARKER_FILE)
}
