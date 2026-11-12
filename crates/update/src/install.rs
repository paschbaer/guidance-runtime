//! Archive installation adapter. The Windows transaction is unchanged from the
//! pre-port implementation: stop service, back up, replace, restart, and
//! restore the backup on any failure.

use crate::ports::Installer;
use crate::UpdateConfig;
use anyhow::{bail, Result};
use std::{fs, path::Path};

/// Installs an update from a ZIP archive using `sc.exe` for service control.
pub struct ZipInstaller;

impl Installer for ZipInstaller {
    fn install(&self, config: &UpdateConfig, bytes: &[u8]) -> Result<()> {
        install_archive(config, bytes)
    }
}

#[allow(unused_variables)]
fn install_archive(config: &UpdateConfig, bytes: &[u8]) -> Result<()> {
    #[cfg(not(windows))]
    {
        bail!("installation is supported on Windows only")
    }
    #[cfg(windows)]
    {
        use std::io::Cursor;

        let staging = tempfile::tempdir()?;
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let Some(rel) = file.enclosed_name() else {
                bail!("unsafe archive path")
            };
            let out = staging.path().join(rel);
            if file.is_dir() {
                fs::create_dir_all(&out)?
            } else {
                if let Some(p) = out.parent() {
                    fs::create_dir_all(p)?
                }
                let mut target = fs::File::create(&out)?;
                std::io::copy(&mut file, &mut target)?;
            }
        }
        let source = find_bin_dir(staging.path())?;
        let backup = config.install_dir.with_extension("backup");
        run_sc("stop", &config.service_name)?;
        std::thread::sleep(std::time::Duration::from_secs(2));
        let operation = (|| -> Result<()> {
            if backup.exists() {
                fs::remove_dir_all(&backup)?
            }
            if config.install_dir.exists() {
                copy_dir(&config.install_dir, &backup)?
            }
            fs::create_dir_all(&config.install_dir)?;
            for name in [
                "guidance-service.exe",
                "guidance.exe",
                "guidance-tray.exe",
                "guidance-updater.exe",
            ] {
                let src = source.join(name);
                if src.exists() {
                    fs::copy(src, config.install_dir.join(name))?;
                }
            }
            Ok(())
        })();
        if let Err(e) = operation {
            if backup.exists() {
                let _ = copy_dir(&backup, &config.install_dir);
            }
            let _ = run_sc("start", &config.service_name);
            return Err(e);
        }
        if let Err(e) = run_sc("start", &config.service_name) {
            let _ = copy_dir(&backup, &config.install_dir);
            let _ = run_sc("start", &config.service_name);
            return Err(e);
        }
        Ok(())
    }
}

#[cfg(windows)]
fn run_sc(action: &str, service: &str) -> Result<()> {
    let s = std::process::Command::new("sc.exe")
        .args([action, service])
        .status()?;
    if !s.success() {
        bail!("sc.exe {action} failed: {s}")
    }
    Ok(())
}
#[cfg(windows)]
fn find_bin_dir(root: &Path) -> Result<std::path::PathBuf> {
    for p in walk(root)? {
        if p.join("guidance-service.exe").exists() {
            return Ok(p);
        }
    }
    bail!("archive does not contain guidance-service.exe")
}
#[cfg(windows)]
fn walk(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut out = vec![root.to_path_buf()];
    for e in fs::read_dir(root)? {
        let p = e?.path();
        if p.is_dir() {
            out.extend(walk(&p)?);
        }
    }
    Ok(out)
}
#[cfg(windows)]
fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &to)?
        } else {
            fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn config() -> UpdateConfig {
        UpdateConfig {
            repository: "owner/repo".into(),
            channel: crate::Channel::Stable,
            install_dir: PathBuf::from(r"C:\Program Files\Guidance Runtime"),
            service_name: "GuidanceRuntime".into(),
            asset_pattern: "windows-x64.zip".into(),
        }
    }

    /// A zip entry that escapes the staging directory must be rejected
    /// before anything is written to the install directory.
    #[cfg(windows)]
    #[test]
    fn unsafe_archive_path_is_rejected() {
        use std::io::Write;

        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("../evil.exe", options).unwrap();
            writer.write_all(b"payload").unwrap();
            writer.finish().unwrap();
        }
        let err = install_archive(&config(), buffer.get_ref()).unwrap_err();
        assert_eq!(err.to_string(), "unsafe archive path");
    }

    /// A payload that is not a zip fails cleanly instead of panicking.
    #[cfg(windows)]
    #[test]
    fn malformed_archive_is_rejected() {
        let err = install_archive(&config(), b"not a zip").unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn installation_is_refused_off_windows() {
        let err = install_archive(&config(), b"whatever").unwrap_err();
        assert!(err.to_string().contains("Windows only"));
    }
}
