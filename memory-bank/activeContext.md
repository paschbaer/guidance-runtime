# Active Context

> This file describes the current working state. Keep it concise and update it after substantial work.

## Current Focus

Spec-Kit feature `001-multi-target-update-sources` is implemented (T001-T040), reviewed and
**merged into `develop`** (`5b064ee`). The updater fans out over a configurable target list, reads a
private source with a credential from the platform store, renders text or JSON, derives the installed
version from a per-target marker, and installs each target transactionally (Windows) without
disturbing another target.

## Current Objective

CI is green on every matrix leg (`windows-latest`, `ubuntu-latest`, `macos-latest`, commit
`28737dd`). The feature is closed out; the open repository work is the deferred items below.

## Current Work

- [x] Fix the release build for `-D warnings` (zip 2 API, winit `ApplicationHandler`, CLI binding).
- [x] Create and verify the `.guidance/` configuration (template `rust`, 23 files).
- [x] Register the repository in the Guidance workspace registry.
- [x] Resolve the SOLID review findings (RW-001 plus OCP/DIP/DRY) on `feature/solid-findings-fix`.
- [x] Decide `Cargo.lock` tracking — tracked since `b9f817d` (RW-002 Handled).
- [x] Make update asset selection architecture-aware so arm64 installations can update.
- [x] Ratify the constitution v1.0.0 (2026-11-12); Sync Impact Report removed.
- [x] Implement feature 001 phases 1-3 (T001-T015).
- [x] Implement phase 4 (T016-T022): CredentialStore, authenticated GitHub adapter, refusal
      classification, credential wiring.
- [x] Implement phases 5-7 (T023-T040): structured text and JSON output, `--format`, version
      marker, shared transactional install, per-target orchestration, macOS CI leg, docs, READMEs,
      least privilege, quickstart validation.
- [x] Merge feature 001 into `develop`; final review (`.guidance/state/final-review.json`) and
      session lessons recorded. Workspace tests: 164 on Windows, 159 on Linux.
- [x] Make the `guidance-update` fixtures platform-independent so the `ubuntu-latest` leg passes.
- [x] Complete `projectBrief.md` — owner and primary users recorded as "User" (2026-11-13).
- [x] Complete `productContext.md` — user needs and journeys documented; the remaining open items
      are roadmap questions, not documentation gaps.

## Recent Changes

- `installer/GuidanceRuntime.wxs` (new), `scripts/build-installer.ps1` (new): WiX v7 MSI installer
  mirroring `install-dev.ps1` — binaries, `GuidanceRuntime` service (auto start, config argument,
  failure actions via deferred `sc.exe failure` custom action), `NeverOverwrite`/`Permanent`
  example configs under `C:\ProgramData\Guidance Runtime\config`, logs folder; `x64`/`arm64` via
  `-arch`. Build verified by a `wix build` link run with placeholder binaries (WiX 7.0.1,
  `-acceptEula wix7`); no real machine install yet.
- `.github/workflows/release.yml`: the MSI is now built per Windows matrix leg and attached to the
  release alongside the ZIP and its SHA-256SUMS (`dist/*.msi` added to both publish steps).
- `crates/cli/src/main.rs`, `crates/updater/src/main.rs`: `Cli::try_parse()` — clap usage errors
  exit 1, help/version exit 0; exit 2 remains exclusively `run.exit_status()` (commit `cf573af`).
- `crates/cli/tests/exit_codes.rs`, `crates/updater/tests/exit_codes.rs` (new): 10 integration
  tests pinning the exit-code contract via `CARGO_BIN_EXE_*` spawns.
- `crates/update/src/install.rs`: `trait ServiceControl` + real `ScControl` adapter (sole
  `sc.exe` site, owns the 2 s settle delay); `install_on_windows`/`restore` take `&dyn
  ServiceControl`; four fake-driven `#[cfg(windows)]` transaction tests (commits `907088f`,
  `8f13b4b`). RW-005 and RW-004 now Handled in `remaining-work-plan.md`.
- `README.md` l.103, `README.de.md` l.103, `docs/IMPLEMENTATION_STATUS.md` l.30: usage-error exit
  status corrected to 1; exit 2 documented as reserved for target failures.
- `crates/update/src/tests.rs`, `crates/update/src/credentials.rs`: the first `ubuntu-latest` run
  failed 23 tests. Fixtures now derive the archive from
  `asset_suffix_for_arch(running_arch())` (`running_zip()`) instead of hard-coding
  `windows-*.zip`, and a `other_arch_zip()` helper supplies a provably foreign archive. The
  architecture-crossing tests keep explicit suffixes; the omitted-pattern test pins an explicit
  per-host table instead of mirroring `asset_suffix_for_arch`. The credential-command test now
  asserts that exactly one argument mentions the entry (the entry itself), which is the property
  that fails when an entry is joined into a shell string. CI run 5 (`28737dd`) is green on
  `windows-latest`, `ubuntu-latest` and `macos-latest`.
- `crates/update/src/report.rs`: dedicated `JsonDocument` (`spec_version`, RFC3339 `checked_at`,
  `exit_status`, `targets[]`) and `render_json`/`render_json_now`; `UpdateRun` remains timestamp-free
  and not `Serialize`. Human rendering keeps the verbatim access phrase and de-duplicates the
  network-error detail.
- `crates/update/src/marker.rs` (new): `install.json` with target, semver version and RFC3339
  timestamp; read state distinguishes Absent / Unreadable / Present; atomic temp+rename write.
- `crates/update/src/command.rs`: `installed_version` is the shared installed-version source;
  `check_one` selects against the marker version, not the running binary; the legacy single-target
  `check_with` also honours a marker when present. `install_all`/`InstallRun` orchestrate per-target
  installation.
- `crates/update/src/install.rs`: one transactional body for every target. Archive discovery is
  `bin/`, then the historical service directory, then the root; replacement is copy-and-overwrite
  and never deletes the install directory; the backup sibling is `"<name>.backup"` (full name, not
  `with_extension`); rollback stops the service before restoring and removes files the rejected
  archive created; the marker is written last.
- `crates/update/src/config.rs`: rejects a rollback directory that collides with any target's
  install directory; `UpdateTarget` denies unknown fields; `select_target` narrows before a run.
- `crates/update/src/credentials.rs`: `provision_hint` renders the platform-correct provisioning
  instruction, so the CLI hint and the lookup cannot disagree (`guidance:update:<entry>` on Windows).
- `crates/cli/src/main.rs`, `crates/updater/src/main.rs`: `--format text|json`; `--target` narrows
  the run; install hands off to the updater, which reports per-target results and exits 0 or 2.
- `crates/tray/src/main.rs`: a pure `check_summary`; an all-failing run reports `Update problem`,
  never `Up to date`.
- `.github/workflows/ci.yml`: `macos-latest` added; `release.yml` remains Windows-only.
- `config/updates.example.toml`, `docs/IMPLEMENTATION_STATUS.md`, `README.md`, `README.de.md`,
  `specs/001-multi-target-update-sources/quickstart.md`: updated to describe the delivered code,
  the least-privilege provisioning rule, and the executed/not-executed validation split.

## Recent Decisions

- The marker is authoritative and is the selection baseline. Using the running CLI version would
  offer a downgrade to a target installed at a different version.
- The transitional legacy rule (no marker + `guidance-service.exe` ⇒ installed at the running
  version) applies only to `guidance-runtime` and only when the marker is genuinely absent. A
  corrupt or mismatched marker is not an installation without a marker.
- The legacy flat form still reuses its historical check, so `--format` and marker reading never
  change its historical exit status; a pre-existing installation has no marker, so SC-001 holds.
- Backup paths are derived from the full directory name; `with_extension("backup")` would make
  `App.Prod` and `App.backup` collide and delete another target's installation.
- Rollback must stop the service before restoring (overwriting a running executable fails) and must
  remove paths the rejected archive created; restoring the backup alone is additive.
- An all-failing check or install run exits 2. This defect has appeared twice in the feature and is
  now pinned by tests in both paths.
- A not-installed target is never installed; during `install` it is reported and skipped with 0,
  while a `check` of the same target may report an access problem with 2. The commands answer
  different questions and the docs state the difference.
- Command-line usage errors exit 1 (nothing was run), not clap's default 2; exit 2 is reserved for
  "at least one target failed". The rejected alternative (exit 64, EX_USAGE) would need a contract
  change for no automation benefit.
- `sc.exe` is invoked from exactly one place: the `ScControl` adapter, which also owns the 2 s
  settle delay after stop. Tests drive the transaction with a recording fake, so service-state
  rollback branches are covered without a real service.
- Windows installation remains the only implemented install platform; macOS/Linux checking is
  supported and the CI leg exists, but non-Windows release legs are deferred until artifacts exist.
- The `guidance` product is obtained from a separate, **release-only private** repository,
  `paschbaer/guidance-dist` (target name `guidance`, credential entry `guidance-update`).
  `guidance-runtime` itself stays on the public source repository `paschbaer/guidance-runtime`.
  Mirroring artifacts into the private repository is the `guidance` product's own pipeline step
  (FR-027–FR-029), not a job in this repository's `release.yml`.
- These decisions are now formally recorded as ADRs in `docs/adr/` (ADR-0001…ADR-0009, created
  2026-11-13); the sections above remain the working summaries.
- The Project Owner (currently "User") approves constitution-only amendments. Recorded as PATCH
  1.0.1 because it names the decision-maker without changing a principle.

## Open Questions

- [x] Who is the project owner (person or team) for `projectBrief.md`? — User (2026-11-13).
- [x] Who are the primary users — workstation operators, or a wider developer audience? — User.
- [x] Which role approves amendments to the constitution? — the Project Owner, currently held by
  "User" (recorded 2026-11-13, constitution PATCH 1.0.1).

## Blockers

- `productContext.md` still has open roadmap questions: whether Linux/macOS support is committed,
  and which hardening items gate a first "production" release. Those need product input, not code.

## Next Actions

1. ~~Wire or remove the global `--json` flag~~ — done 2026-11-13: removed (Option A, commit
   `fa490cd` on `feature/json-flag-resolution`); IPC output is always pretty; a usage-error test
   pins the removal. The stale exit-code bullet in `IMPLEMENTATION_STATUS.md` was fixed too.
2. Restart the Guidance server so the final `.guidance/` state is snapshotted.
3. Re-bless `.guidance/state/final-review.json` (`headCommit`) after the next commit that changes
   code, before re-running the completion gate.
4. ~~Confirm CI is green on all three legs for `4b83ea0`~~ — confirmed 2026-11-13: `ci` green on
   windows/ubuntu/macos for `4b83ea0` and `448a32c`; `Runtime CI` and `Runtime Release` green.
5. Merge and push `feature/json-flag-resolution` (operator approval pending).

## Last Updated

2026-11-13 (agent sessions: WiX v7 MSI installer added and link-validated; cross-platform fixtures
and the measured Linux leg; owner and primary users recorded as "User"; CLI usage-error exit codes
aligned with `update-cli.md` §2; ServiceControl port (RW-005/RW-004 Handled); governance approver
named, constitution PATCH 1.0.1)
