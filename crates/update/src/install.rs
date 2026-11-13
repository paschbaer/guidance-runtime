//! Archive installation adapter.
//!
//! One transactional body serves every target. The runtime target keeps the
//! historical sequence exactly — stop the service, back up, replace, restart,
//! restore the backup on any failure. A target without a service runs the same
//! transaction minus service control (FR-020a, FR-021).
//!
//! Replacement is copy-and-overwrite: the transaction never deletes the
//! installation directory, because uninstalling is not part of this feature and
//! a delete could remove operator-placed files.

use crate::ports::Installer;
use crate::UpdateTarget;
use anyhow::{bail, Result};
use semver::Version;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::{collections::VecDeque, fs};

/// The files a runtime archive is expected to carry.
#[cfg(windows)]
const RUNTIME_FILES: [&str; 4] = [
    "guidance-service.exe",
    "guidance.exe",
    "guidance-tray.exe",
    "guidance-updater.exe",
];

/// Installs an update from a ZIP archive.
pub struct ZipInstaller;

impl Installer for ZipInstaller {
    fn install(&self, target: &UpdateTarget, version: &Version, bytes: &[u8]) -> Result<()> {
        install_archive(target, version, bytes)
    }
}

#[allow(unused_variables)]
fn install_archive(target: &UpdateTarget, version: &Version, bytes: &[u8]) -> Result<()> {
    #[cfg(not(windows))]
    {
        bail!("installation is supported on Windows only")
    }
    #[cfg(windows)]
    {
        let control = ScControl::with_settle(std::time::Duration::from_secs(2));
        install_on_windows(target, version, bytes, &control)
    }
}

/// Service control for the install transaction. The real adapter shells out to
/// `sc.exe`; tests substitute a recording fake so no real service is touched.
#[cfg(windows)]
trait ServiceControl {
    fn stop(&self, name: &str) -> Result<()>;
    fn start(&self, name: &str) -> Result<()>;
}

/// The production adapter. It owns the settle delay: after `sc.exe stop` the
/// service manager needs a moment before the files are actually unlocked.
#[cfg(windows)]
struct ScControl {
    settle: std::time::Duration,
}

#[cfg(windows)]
impl ScControl {
    fn with_settle(settle: std::time::Duration) -> Self {
        Self { settle }
    }
}

#[cfg(windows)]
impl ServiceControl for ScControl {
    fn stop(&self, name: &str) -> Result<()> {
        run_sc("stop", name)?;
        std::thread::sleep(self.settle);
        Ok(())
    }

    fn start(&self, name: &str) -> Result<()> {
        run_sc("start", name)
    }
}

#[cfg(windows)]
fn install_on_windows(
    target: &UpdateTarget,
    version: &Version,
    bytes: &[u8],
    control: &dyn ServiceControl,
) -> Result<()> {
    let staging = tempfile::tempdir()?;
    extract(bytes, staging.path())?;

    // A target named `guidance-runtime` keeps the historical shape even when
    // `service_name` is omitted: the key was optional and previously implied
    // `GuidanceRuntime`, so its absence must not turn the runtime into a
    // service-less target with a recursive file set.
    let runtime_shaped = target.name == crate::LEGACY_TARGET_NAME;
    let service = service_name(target);

    // The archive determines its own content (HANDOVER §4.2): a `bin/`
    // directory wins, because that is the layout the runtime archive uses; a
    // runtime target without `bin/` falls back to the historical search for
    // `guidance-service.exe`; anything else replaces the archive root.
    let source = discover_source(staging.path(), runtime_shaped)?;
    let files = file_set(&source, runtime_shaped)?;
    if files.is_empty() {
        bail!("archive contains no installable files");
    }

    let backup = backup_path(&target.install_dir);

    if let Some(service) = service {
        control.stop(service)?;
    }

    // Paths this transaction creates (rather than overwrites) are recorded so
    // a rollback can remove exactly them: restoring the backup alone is
    // additive and would leave files from the rejected archive behind.
    let mut created: Vec<PathBuf> = Vec::new();
    let replacement = (|| -> Result<()> {
        if backup.exists() {
            fs::remove_dir_all(&backup)?;
        }
        if target.install_dir.exists() {
            copy_dir(&target.install_dir, &backup)?;
        }
        fs::create_dir_all(&target.install_dir)?;
        for (relative, source_path) in &files {
            let destination = target.install_dir.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            if !destination.exists() {
                created.push(destination.clone());
            }
            fs::copy(source_path, &destination)?;
        }
        Ok(())
    })();

    if let Err(e) = replacement {
        restore(&backup, &target.install_dir, service, &created, control);
        return Err(e);
    }

    if let Some(service) = service {
        if let Err(e) = control.start(service) {
            restore(
                &backup,
                &target.install_dir,
                Some(service),
                &created,
                control,
            );
            return Err(e);
        }
    }

    // The marker is the final step (FR-020b). A failure here restores the
    // previous installation so the marker never claims a version that is not
    // fully in place.
    if let Err(e) = crate::marker::write(&target.install_dir, &target.name, &version.to_string()) {
        restore(&backup, &target.install_dir, service, &created, control);
        return Err(e);
    }
    Ok(())
}

/// The service this target controls, if any.
///
/// A target named `guidance-runtime` defaults to `GuidanceRuntime` because the
/// configuration key was always optional for it. A foreign target controls a
/// service only when it names one.
#[cfg(windows)]
fn service_name(target: &UpdateTarget) -> Option<&str> {
    if target.service_name.is_some() || target.name == crate::LEGACY_TARGET_NAME {
        Some(target.service_name.as_deref().unwrap_or("GuidanceRuntime"))
    } else {
        None
    }
}

#[cfg(windows)]
fn extract(bytes: &[u8], staging: &Path) -> Result<()> {
    use std::io::Cursor;

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let Some(rel) = file.enclosed_name() else {
            bail!("unsafe archive path")
        };
        let out = staging.join(rel);
        if file.is_dir() {
            fs::create_dir_all(&out)?
        } else {
            if let Some(p) = out.parent() {
                fs::create_dir_all(p)?
            }
            let mut sink = fs::File::create(&out)?;
            std::io::copy(&mut file, &mut sink)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn discover_source(staging: &Path, has_service: bool) -> Result<PathBuf> {
    if let Some(bin) = find_dir_named(staging, "bin")? {
        // A runtime archive must actually carry its service binary in `bin/`;
        // an unrelated `bin/` directory must not hide a nested service layout.
        if !has_service || bin.join("guidance-service.exe").is_file() {
            return Ok(bin);
        }
    }
    if has_service {
        return find_service_dir(staging);
    }
    Ok(staging.to_path_buf())
}

/// The shallowest directory named `name` anywhere in the extracted tree.
#[cfg(windows)]
fn find_dir_named(root: &Path, name: &str) -> Result<Option<PathBuf>> {
    let mut queue = VecDeque::from([root.to_path_buf()]);
    while let Some(directory) = queue.pop_front() {
        if directory != root && directory.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Ok(Some(directory));
        }
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                queue.push_back(path);
            }
        }
    }
    Ok(None)
}

/// The historical runtime layout: the directory holding the service binary.
#[cfg(windows)]
fn find_service_dir(root: &Path) -> Result<PathBuf> {
    for p in walk(root)? {
        if p.join("guidance-service.exe").exists() {
            return Ok(p);
        }
    }
    bail!("archive does not contain guidance-service.exe")
}

/// The files to copy, as `(relative destination, source)`.
#[cfg(windows)]
fn file_set(source: &Path, runtime_shaped: bool) -> Result<Vec<(PathBuf, PathBuf)>> {
    if runtime_shaped {
        // Preserve the historical behaviour: only the known runtime binaries
        // are replaced, never anything else in the install directory. This is
        // keyed on the target identity, not on the optional service_name key.
        return Ok(RUNTIME_FILES
            .iter()
            .map(PathBuf::from)
            .filter(|relative| source.join(relative).is_file())
            .map(|relative| {
                let absolute = source.join(&relative);
                (relative, absolute)
            })
            .collect());
    }
    let mut files = Vec::new();
    collect_relative_files(source, source, &mut files)?;
    Ok(files)
}

#[cfg(windows)]
fn collect_relative_files(
    root: &Path,
    directory: &Path,
    out: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_relative_files(root, &path, out)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|e| anyhow::anyhow!("build relative path: {e}"))?
                .to_path_buf();
            out.push((relative, path));
        }
    }
    Ok(())
}

/// The sibling directory used as a rollback snapshot.
///
/// Derived from the full file name rather than `with_extension`, which would
/// replace everything after the last dot: `App.Prod` and `App.backup` would
/// both map to `App.backup`, and one target's rollback would delete another
/// target's installation.
pub(crate) fn backup_path(install_dir: &Path) -> PathBuf {
    match install_dir.file_name().and_then(|name| name.to_str()) {
        Some(name) => install_dir.with_file_name(format!("{name}.backup")),
        None => install_dir.with_extension("backup"),
    }
}

/// Restores the backup, removes only the paths this transaction created, and
/// restarts the service, if there is one.
///
/// The service is stopped before the restore: overwriting a running executable
/// fails, and a restore that silently does nothing would report failure while
/// the new binaries stay live.
#[cfg(windows)]
fn restore(
    backup: &Path,
    install_dir: &Path,
    service: Option<&str>,
    created: &[PathBuf],
    control: &dyn ServiceControl,
) {
    if let Some(service) = service {
        let _ = control.stop(service);
    }
    if backup.exists() {
        let _ = copy_dir(backup, install_dir);
    }
    for path in created.iter().rev() {
        let _ = fs::remove_file(path);
        if let Some(mut directory) = path.parent() {
            while directory != install_dir {
                if fs::remove_dir(directory).is_err() {
                    break;
                }
                match directory.parent() {
                    Some(parent) => directory = parent,
                    None => break,
                }
            }
        }
    }
    if let Some(service) = service {
        let _ = control.start(service);
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
fn walk(root: &Path) -> Result<Vec<PathBuf>> {
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

    fn config() -> UpdateTarget {
        UpdateTarget {
            name: "guidance-runtime".into(),
            repository: "owner/repo".into(),
            visibility: crate::Visibility::Public,
            credential_entry: None,
            channel: crate::Channel::Stable,
            install_dir: PathBuf::from(r"C:\Program Files\Guidance Runtime"),
            service_name: Some("GuidanceRuntime".into()),
            asset_pattern: Some("windows-x64.zip".into()),
        }
    }

    fn version() -> Version {
        Version::parse("2.0.0").unwrap()
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
        let err = install_archive(&config(), &version(), buffer.get_ref()).unwrap_err();
        assert_eq!(err.to_string(), "unsafe archive path");
    }

    /// A payload that is not a zip fails cleanly instead of panicking.
    #[cfg(windows)]
    #[test]
    fn malformed_archive_is_rejected() {
        let err = install_archive(&config(), &version(), b"not a zip").unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[cfg(not(windows))]
    #[test]
    fn installation_is_refused_off_windows() {
        let err = install_archive(&config(), &version(), b"whatever").unwrap_err();
        assert!(err.to_string().contains("Windows only"));
    }

    #[cfg(windows)]
    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;

        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            for (name, payload) in entries {
                writer.start_file(*name, options).unwrap();
                writer.write_all(payload).unwrap();
            }
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    #[cfg(windows)]
    fn service_less_target(install_dir: &std::path::Path) -> UpdateTarget {
        let mut target = config();
        target.name = "guidance".into();
        target.install_dir = install_dir.to_path_buf();
        target.service_name = None;
        target
    }

    /// FR-020a: a service-less target installs transactionally without service
    /// control, replaces the archive root when there is no `bin/`, overwrites
    /// files, never deletes an operator file, and records the new version last.
    #[cfg(windows)]
    #[test]
    fn a_service_less_archive_replaces_the_root_and_writes_the_marker_last() {
        let root = tempfile::tempdir().unwrap();
        let install_dir = root.path().join("install");
        std::fs::create_dir_all(&install_dir).unwrap();
        std::fs::write(install_dir.join("guidance.exe"), b"old").unwrap();
        std::fs::write(install_dir.join("operator-placed.txt"), b"keep me").unwrap();

        let target = service_less_target(&install_dir);
        let bytes = zip_bytes(&[("guidance.exe", b"new"), ("assets/data.txt", b"payload")]);

        install_archive(&target, &version(), &bytes).unwrap();

        assert_eq!(
            std::fs::read(install_dir.join("guidance.exe")).unwrap(),
            b"new"
        );
        assert_eq!(
            std::fs::read(install_dir.join("assets/data.txt")).unwrap(),
            b"payload"
        );
        assert_eq!(
            std::fs::read(install_dir.join("operator-placed.txt")).unwrap(),
            b"keep me",
            "the transaction must not delete operator-placed files"
        );
        assert_eq!(
            crate::marker::read_version(&install_dir, "guidance").as_deref(),
            Some("2.0.0"),
            "the marker records the version only after the replacement succeeded"
        );
    }

    /// The archive determines its content: a `bin/` directory limits the
    /// replacement to its contents, so sibling documentation is not installed.
    #[cfg(windows)]
    #[test]
    fn a_bin_directory_limits_replacement_to_its_contents() {
        let root = tempfile::tempdir().unwrap();
        let install_dir = root.path().join("install");
        std::fs::create_dir_all(&install_dir).unwrap();
        std::fs::write(install_dir.join("keep.txt"), b"keep").unwrap();

        let target = service_less_target(&install_dir);
        let bytes = zip_bytes(&[
            ("bin/guidance.exe", b"binary"),
            ("docs/readme.txt", b"not installed"),
        ]);

        install_archive(&target, &version(), &bytes).unwrap();

        assert_eq!(
            std::fs::read(install_dir.join("guidance.exe")).unwrap(),
            b"binary"
        );
        assert!(
            !install_dir.join("docs/readme.txt").exists(),
            "only bin/ contents may be installed"
        );
        assert!(install_dir.join("keep.txt").exists());
    }

    /// A service-less target must not run `sc.exe`: the command would fail for
    /// a service this machine does not have, so a successful install is the
    /// evidence that service control was skipped.
    #[cfg(windows)]
    #[test]
    fn a_service_less_target_does_not_run_service_control() {
        let root = tempfile::tempdir().unwrap();
        let install_dir = root.path().join("install");
        let target = service_less_target(&install_dir);
        assert_eq!(target.service_name, None);

        let bytes = zip_bytes(&[("guidance.exe", b"binary")]);
        install_archive(&target, &version(), &bytes).unwrap();

        assert_eq!(
            crate::marker::read_version(&install_dir, "guidance").as_deref(),
            Some("2.0.0")
        );
    }

    /// The runtime target keeps the historical four-binary replacement even
    /// when `service_name` is omitted, because that key was optional and
    /// previously implied `GuidanceRuntime`.
    #[cfg(windows)]
    #[test]
    fn the_runtime_file_set_stays_four_binaries_without_a_service_name() {
        let source = tempfile::tempdir().unwrap();
        for name in RUNTIME_FILES {
            std::fs::write(source.path().join(name), b"bin").unwrap();
        }
        std::fs::write(source.path().join("helper.dll"), b"not a runtime file").unwrap();

        let mut target = config();
        target.service_name = None;
        assert_eq!(service_name(&target), Some("GuidanceRuntime"));

        let files = file_set(source.path(), target.name == crate::LEGACY_TARGET_NAME).unwrap();
        let mut names: Vec<&str> = files
            .iter()
            .map(|(relative, _)| relative.to_str().unwrap())
            .collect();
        names.sort_unstable();
        let mut expected = RUNTIME_FILES.to_vec();
        expected.sort_unstable();
        assert_eq!(
            names, expected,
            "only the four runtime binaries may be replaced"
        );

        let foreign = service_less_target(source.path());
        assert_eq!(service_name(&foreign), None);
    }

    /// A failure at the final step must restore the previous installation and
    /// remove the files the rejected archive created; a purely additive
    /// rollback would leave new artefacts behind.
    #[cfg(windows)]
    #[test]
    fn a_failed_install_removes_files_it_created_and_restores_the_old_state() {
        let root = tempfile::tempdir().unwrap();
        let install_dir = root.path().join("install");
        std::fs::create_dir_all(&install_dir).unwrap();
        std::fs::write(install_dir.join("old.txt"), b"old").unwrap();
        crate::marker::write(&install_dir, "guidance", "1.0.0").unwrap();
        // Block the marker's temporary file so the last step fails after the
        // replacement already happened.
        std::fs::create_dir_all(install_dir.join("install.json.tmp")).unwrap();

        let target = service_less_target(&install_dir);
        let bytes = zip_bytes(&[("new.txt", b"new")]);

        install_archive(&target, &version(), &bytes).unwrap_err();

        assert!(
            !install_dir.join("new.txt").exists(),
            "a rolled-back install must not leave files from the rejected archive"
        );
        assert_eq!(std::fs::read(install_dir.join("old.txt")).unwrap(), b"old");
        assert_eq!(
            crate::marker::read_version(&install_dir, "guidance").as_deref(),
            Some("1.0.0"),
            "a failed install must not claim the new version"
        );
    }

    /// A recording service-control fake: every successful call is appended to
    /// `calls`; an attempt matching `fail_on` fails once (the failure is not
    /// recorded) and later attempts succeed. It has
    /// no settle delay, so service-driven tests stay fast.
    #[cfg(windows)]
    struct RecordingControl {
        calls: std::cell::RefCell<Vec<&'static str>>,
        fail_on: Option<&'static str>,
        failed: std::cell::Cell<bool>,
    }

    #[cfg(windows)]
    impl RecordingControl {
        fn recording() -> Self {
            Self {
                calls: std::cell::RefCell::new(Vec::new()),
                fail_on: None,
                failed: std::cell::Cell::new(false),
            }
        }

        fn failing_on(action: &'static str) -> Self {
            Self {
                calls: std::cell::RefCell::new(Vec::new()),
                fail_on: Some(action),
                failed: std::cell::Cell::new(false),
            }
        }

        /// Fails the first attempt of `self.fail_on`, then recovers.
        fn attempt(&self, action: &'static str) -> Result<()> {
            if self.fail_on == Some(action) && !self.failed.get() {
                self.failed.set(true);
                bail!("fake {action} failure")
            }
            self.calls.borrow_mut().push(action);
            Ok(())
        }
    }

    #[cfg(windows)]
    impl ServiceControl for RecordingControl {
        fn stop(&self, _name: &str) -> Result<()> {
            self.attempt("stop")
        }

        fn start(&self, _name: &str) -> Result<()> {
            self.attempt("start")
        }
    }

    #[cfg(windows)]
    fn served_target(install_dir: &std::path::Path) -> UpdateTarget {
        let mut target = config();
        target.install_dir = install_dir.to_path_buf();
        target
    }

    /// The service-bearing happy path: stop before replacement, start after,
    /// marker written last — and nothing else.
    #[cfg(windows)]
    #[test]
    fn a_served_target_stops_replaces_and_starts() {
        let root = tempfile::tempdir().unwrap();
        let install_dir = root.path().join("install");
        std::fs::create_dir_all(&install_dir).unwrap();
        std::fs::write(install_dir.join("guidance.exe"), b"old").unwrap();

        let control = RecordingControl::recording();
        let target = served_target(&install_dir);
        let bytes = zip_bytes(&[
            ("bin/guidance.exe", b"new"),
            ("bin/guidance-service.exe", b"svc"),
            ("bin/created/new-file.txt", b"created"),
        ]);

        install_on_windows(&target, &version(), &bytes, &control).unwrap();

        assert_eq!(*control.calls.borrow(), vec!["stop", "start"]);
        assert_eq!(
            std::fs::read(install_dir.join("guidance.exe")).unwrap(),
            b"new"
        );
        assert_eq!(
            crate::marker::read_version(&install_dir, "guidance-runtime").as_deref(),
            Some("2.0.0")
        );
    }

    /// A start failure after the replacement must roll back completely: old
    /// files restored, files the archive created removed, marker untouched,
    /// and the service stopped again before the restore.
    #[cfg(windows)]
    #[test]
    fn a_failed_start_restores_the_old_state_and_stops_before_restoring() {
        let root = tempfile::tempdir().unwrap();
        let install_dir = root.path().join("install");
        std::fs::create_dir_all(&install_dir).unwrap();
        std::fs::write(install_dir.join("old.txt"), b"old").unwrap();
        crate::marker::write(&install_dir, "guidance-runtime", "1.0.0").unwrap();

        let control = RecordingControl::failing_on("start");
        let target = served_target(&install_dir);
        let bytes = zip_bytes(&[
            ("bin/guidance-service.exe", b"svc"),
            ("bin/old.txt", b"new"),
            ("bin/created.txt", b"created"),
        ]);

        install_on_windows(&target, &version(), &bytes, &control).unwrap_err();

        assert_eq!(
            *control.calls.borrow(),
            vec!["stop", "stop", "start"],
            "install stop, rollback stop, rollback start — in that order"
        );
        assert_eq!(std::fs::read(install_dir.join("old.txt")).unwrap(), b"old");
        assert!(
            !install_dir.join("created.txt").exists(),
            "files the rejected archive created must be removed"
        );
        assert_eq!(
            crate::marker::read_version(&install_dir, "guidance-runtime").as_deref(),
            Some("1.0.0"),
            "the start never succeeded, so the marker must stay at the old version"
        );
    }

    /// A stop failure aborts before the transaction touches anything: the
    /// install directory stays byte-identical, no backup appears, and no
    /// start is attempted.
    #[cfg(windows)]
    #[test]
    fn a_failed_stop_aborts_before_touching_the_installation() {
        let root = tempfile::tempdir().unwrap();
        let install_dir = root.path().join("install");
        std::fs::create_dir_all(&install_dir).unwrap();
        std::fs::write(install_dir.join("old.txt"), b"old").unwrap();

        let control = RecordingControl::failing_on("stop");
        let target = served_target(&install_dir);
        let bytes = zip_bytes(&[("old.txt", b"new")]);

        install_on_windows(&target, &version(), &bytes, &control).unwrap_err();

        assert_eq!(*control.calls.borrow(), Vec::<&str>::new());
        assert_eq!(std::fs::read(install_dir.join("old.txt")).unwrap(), b"old");
        assert_eq!(
            std::fs::read_dir(&install_dir).unwrap().count(),
            1,
            "the install directory must be byte-identical"
        );
        assert!(!backup_path(&install_dir).exists());
    }

    /// The 2 s settle delay belongs to the real adapter, not to the
    /// transaction: a fake-driven install must complete well under it.
    #[cfg(windows)]
    #[test]
    fn a_fake_driven_install_does_not_incur_the_settle_delay() {
        let root = tempfile::tempdir().unwrap();
        let install_dir = root.path().join("install");
        std::fs::create_dir_all(&install_dir).unwrap();

        let control = RecordingControl::recording();
        let target = served_target(&install_dir);
        let bytes = zip_bytes(&[
            ("bin/guidance.exe", b"new"),
            ("bin/guidance-service.exe", b"svc"),
        ]);

        let start = std::time::Instant::now();
        install_on_windows(&target, &version(), &bytes, &control).unwrap();
        assert!(
            start.elapsed() < std::time::Duration::from_secs(1),
            "the test fake must not sleep for the production settle delay"
        );
    }
}
