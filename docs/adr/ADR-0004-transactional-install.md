# ADR-0004: Transactional Windows install with full rollback

- **Status:** Accepted (2026-11-12; rollback hardening 2026-11-12/13)
- **Context:** Replacing the binaries of a running Windows installation is
  destructive: a half-applied update would leave the target unbootable, and a
  restore attempt against running executables silently does nothing (review
  finding `restore-while-running`). A purely additive rollback also left files
  the rejected archive had created behind (`additive-rollback`).
- **Decision:** Every install runs one transaction: stop the target's service
  (if it has one) → copy the installation to the backup sibling → replace
  files → start the service → write the version marker last. The backup
  sibling is derived from the *full* directory name (`App.Prod` →
  `App.Prod.backup`), falling back to an extension-style sibling only when the
  path has no file-name component, and configuration
  validation rejects any collision between an install directory and another
  target's backup directory (`backup-path-collision`). On any failure the
  rollback stops the service *before* restoring the backup, removes exactly
  the paths the transaction had created, then restarts the service. Restore
  errors are deliberately not escalated — the original install error is
  reported. The service-control calls sit behind the `ServiceControl` port
  (ADR-0002), which owns the 2 s settle delay after `sc.exe stop`.
- **Consequences:** A failed install cannot leave a broken or mixed
  installation, and one target's rollback can never delete another target's
  installation. File replacement is copy-and-overwrite: the install directory
  is never deleted, so operator-placed files survive (ADR-0008). The
  transaction is Windows-only and cfg-gated; targets without a service run the
  same transaction minus service control.
- **Evidence:** `crates/update/src/install.rs` (`install_on_windows`,
  `restore`, `backup_path`), `crates/update/src/config.rs` (collision
  validation), the `#[cfg(windows)]` transaction tests in `install.rs`
  (`a_failed_start_restores_the_old_state_and_stops_before_restoring`,
  `a_failed_stop_aborts_before_touching_the_installation`,
  `a_service_less_archive_replaces_the_root_and_writes_the_marker_last`), the
  dotted-name collision tests in `crates/update/src/config_tests.rs`,
  `specs/001-multi-target-update-sources/spec.md` (rollback clarification) and
  `research.md` (R5).
