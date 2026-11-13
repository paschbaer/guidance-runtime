# ADR-0006: Normalized multi-target configuration with a preserved legacy flat form

- **Status:** Accepted (2026-11-12)
- **Context:** The updater started as a single-target tool with one flat TOML
  file and grew multi-target support (`[[targets]]`). Raw parsed TOML cannot
  express the invariants the multi-target form needs (unique target names,
  unique install directories, rollback-directory collisions, required
  `install_dir`, valid semver-adjacent fields), and silently defaulting fields
  had already caused a defect (`install-dir-default`: an array entry without
  `install_dir` defaulted to the runtime's own directory).
- **Decision:** Configuration is parsed in two stages: a raw `ConfigFile`
  (serde) and a normalized `UpdateConfig` whose construction enforces every
  invariant with an error that names the offending target and field. The
  historical flat form remains valid and keeps its historical behaviour —
  same output wording, same exit status on a source failure (SC-001), only one
  target. `select_target` narrows a run before it executes, so an excluded
  target's sources and credential stores are never contacted.
- **Consequences:** All validation lives in one place and every violation is
  actionable; new invariants are additive. The legacy form is permanent
  compatibility surface — it constrains output and exit-status changes forever
  and must not silently gain new semantics.
- **Evidence:** `crates/update/src/config.rs` (`ConfigFile`, `UpdateConfig`,
  `select_target`, collision validation) and the `config_tests.rs` suite
  (duplicate names, duplicate install dirs, required `install_dir`, legacy
  default), `specs/001-multi-target-update-sources/contracts/update-cli.md`,
  `memory-bank/activeContext.md` (Recent Decisions on the legacy form).
