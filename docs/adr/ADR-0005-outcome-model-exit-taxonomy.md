# ADR-0005: Seven-state outcome model with a 0/1/2 exit taxonomy

- **Status:** Accepted (2026-11-12; usage-error exit code corrected 2026-11-13)
- **Context:** Operators script `guidance update check`/`install`, so the
  result must be machine-consumable: every target ends in a *named* state, and
  the process exit status must distinguish "clean", "configuration problem",
  and "at least one target failed". Two defect classes motivated the closed
  model: failures being collapsed into a clean-looking result (the tray
  originally reported an all-failing run as "Up to date"), and a defined state
  (`invalid_artifact`) being unreachable because selection silently dropped a
  newer release without a usable artifact as "up to date".
- **Decision:** Every checked target resolves to exactly one of seven states —
  `update_available`, `up_to_date`, `not_installed`, `inaccessible`,
  `network_error`, `invalid_artifact`, `credential_store_unavailable` — held in
  an `UpdateRun` whose invariants are enforced at construction. Exit status is
  `0` clean, `1` usage/configuration error (nothing was run — clap usage
  errors included, via `try_parse()`), `2` at least one target failed
  (`inaccessible`, `network_error`, `invalid_artifact`,
  `credential_store_unavailable`). Exit `2` is reserved for target failures and
  is never used for usage errors (contract `update-cli.md` §2, pinned by
  integration tests in both binaries). `--target` narrows the run itself, not
  just the printing, and per-target failure isolation means one failing target
  cannot hide another's result.
- **Consequences:** Scripts can branch on the exit code without parsing output;
  a new state requires a model change rather than ad-hoc reporting. The cost is
  that every new failure mode must be classified into an existing state or the
  model must grow — deliberate friction against silent fail-open.
- **Evidence:** `crates/update/src/report.rs`, `crates/update/src/selection.rs`,
  `specs/001-multi-target-update-sources/contracts/update-cli.md` §2,
  `crates/cli/tests/exit_codes.rs`, `crates/updater/tests/exit_codes.rs`,
  `crates/tray/src/main.rs` (failing-run summary) and its tests.
