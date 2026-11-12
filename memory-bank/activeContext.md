# Active Context

> This file describes the current working state. Keep it concise and update it after substantial work.

## Current Focus

Spec-Kit feature `001-multi-target-update-sources` is implemented through T040 on
`feature/multi-target-update-sources`. The updater fans out over a configurable target list, reads a
private source with a credential from the platform store, renders text or JSON, derives the installed
version from a per-target marker, and installs each target transactionally (Windows) without
disturbing another target.

## Current Objective

Close out feature 001: commit the work, record the independent final review and session lessons, and
validate the completion gate honestly. The remaining repository work is the deferred items below,
not this feature.

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
      least privilege, quickstart validation. Workspace tests 166 (147 in `guidance-update`).
- [ ] Complete `projectBrief.md` — blocked on owner and primary-user input.
- [ ] Complete `productContext.md` — user journeys need product input.

## Recent Changes

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
- Windows installation remains the only implemented install platform; macOS/Linux checking is
  supported and the CI leg exists, but non-Windows release legs are deferred until artifacts exist.

## Open Questions

- [ ] Who is the project owner (person or team) for `projectBrief.md`?
- [ ] Who are the primary users — workstation operators, or a wider developer audience?
- [ ] Which role approves amendments to the constitution (the ratification date is fixed at
  2026-11-12)?

## Blockers

- `projectBrief.md` and `productContext.md` cannot be completed without product/ownership input.

## Next Actions

1. Commit the phase 5-7 work as one feature commit; write the final-review evidence after the commit.
2. Answer the open questions above, then fill `projectBrief.md` and `productContext.md`.
3. Decide the CLI-wide exit-code taxonomy (clap usage errors currently exit 2, colliding with
   "at least one target failed").
4. Wire or remove the global `--json` flag on `guidance`; update subcommands use `--format json`.
5. Add a service-control port so the Windows service-state half of target isolation becomes testable
   (RW-005).
6. Restart the Guidance server so the final `.guidance/` state is snapshotted.

## Last Updated

2026-11-12 (agent session: feature 001 phases 5-7 implemented and validated)
