# ADR-0003: `install.json` marker as the authoritative installed-version source

- **Status:** Accepted (2026-11-12)
- **Context:** The updater needs to know which version is installed. Using the
  running CLI version was wrong in both directions: a target installed at a
  different version could be offered a downgrade, and an update could be
  skipped because the *running* binary happened to be new. An earlier
  implementation even displayed the marker while still selecting against the
  running version — a defect caught in review (`marker-display-only`).
- **Decision:** Each target records `install.json` in its installation
  directory (target name, semver version, RFC3339 timestamp), written atomically
  (temp + rename) and only after a fully successful install, so a crash can
  never claim a version that is not in place. Reading distinguishes
  `Absent` / `Unreadable` / `Present`: only genuine absence triggers the
  transitional rule (guidance-runtime only: no marker plus
  `guidance-service.exe` means installed at the running version), because a
  corrupt or mismatched marker is not an installation without a marker. The
  marker version is the selection baseline in every check path, including the
  legacy single-target form.
- **Consequences:** Selection and display share one source of truth; downgrade
  offers and stale-running-version decisions are impossible. The transitional
  rule is permanent legacy support scoped to one target name and must be kept
  narrow.
- **Evidence:** `crates/update/src/marker.rs`, `crates/update/src/command.rs`, the marker
  tests in `crates/update/src/tests.rs` (display/selection baseline with differing versions,
  unreadable-marker states, forward-compatible extra field),
  `memory-bank/activeContext.md` (Recent Decisions).
