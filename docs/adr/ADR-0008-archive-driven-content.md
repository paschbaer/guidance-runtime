# ADR-0008: Archive-driven install content

- **Status:** Accepted (2026-11-12)
- **Context:** The updater installs ZIP artifacts whose layouts differ: the
  runtime archive carries a `bin/` directory plus a service binary; foreign
  products ship arbitrary roots. The updater cannot know a product's layout,
  so the archive itself must decide what is installed — while the runtime
  target must keep its historical shape even when optional configuration keys
  are absent (`service-name-regression`: keying the file set on
  `service_name.is_some()` silently widened the runtime replacement to a
  recursive copy).
- **Decision:** The archive determines its own content, discovered in a fixed
  order: a `bin/` directory wins (for a runtime target, `bin/` must actually
  contain `guidance-service.exe`, so an unrelated `bin/` cannot hide a nested
  layout); otherwise a runtime target falls back to the historical search for
  `guidance-service.exe`; anything else replaces the archive root. A runtime
  target keeps its historical four-binary file set (`guidance-service.exe`,
  `guidance.exe`, `guidance-tray.exe`, `guidance-updater.exe`) even when
  `service_name` is omitted, because the key was optional and previously
  implied `GuidanceRuntime`. Replacement is copy-and-overwrite only: the
  installation directory is never deleted, so operator-placed files survive;
  extraction rejects archive paths that escape the staging directory.
- **Consequences:** The updater installs any conforming archive without
  product-specific code. The cost is that archive layout *is* the contract:
  a product shipping an unexpected layout changes what is installed, which is
  why the runtime fallback ordering and the four-binary set are pinned by
  tests.
- **Evidence:** `crates/update/src/install.rs` (`discover_source`, `file_set`,
  `RUNTIME_FILES`, `extract`), the `#[cfg(windows)]` tests
  (`a_bin_directory_limits_replacement_to_its_contents`,
  `the_runtime_file_set_stays_four_binaries_without_a_service_name`,
  `unsafe_archive_path_is_rejected`),
  `specs/001-multi-target-update-sources/` (spec clarifications and
  `data-model.md`).
