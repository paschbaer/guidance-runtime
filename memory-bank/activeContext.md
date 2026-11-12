# Active Context

> This file describes the current working state. Keep it concise and update it after substantial work.

## Current Focus

Executing Spec-Kit feature `001-multi-target-update-sources`. The first three phases (setup,
foundational, User Story 1) are implemented on `feature/multi-target-update-sources`: the updater
fans out over a configurable target list instead of addressing one hardcoded repository, and the
platform suffix table covers Windows, macOS and Linux.

## Current Objective

Continue with User Story 2 (authenticated private-source access, T016-T022). Until it lands, a
private target reports `no access to update source` unconditionally and must not be presented as
working private-source support.

## Current Work

- [x] Fix the release build for `-D warnings` (zip 2 API, winit `ApplicationHandler`, CLI binding).
- [x] Create and verify the `.guidance/` configuration (template `rust`, 23 files).
- [x] Register the repository in the Guidance workspace registry.
- [x] Resolve the SOLID review findings (RW-001 plus OCP/DIP/DRY) on `feature/solid-findings-fix`.
- [x] Decide `Cargo.lock` tracking — tracked since `b9f817d` (RW-002 Handled).
- [x] Make update asset selection architecture-aware so arm64 installations can update.
- [x] Draft the constitution v1.0.0 in `.specify/memory/constitution.md` (scaffold resolved).
- [x] Ratify the constitution v1.0.0 (2026-11-12); Sync Impact Report removed.
- [x] Implement Spec-Kit feature 001 phases 1-3 (T001-T015): multi-target configuration, outcome
      model, platform-aware suffix table, fan-out, exit statuses, CLI wiring.
- [ ] Continue feature 001 with US2 (T016-T022) — private-source access.
- [ ] Complete `projectBrief.md` — blocked on owner and primary-user input.
- [ ] Complete `productContext.md` — user journeys need product input.

## Recent Changes

- `crates/update/src/`: added `config.rs` (target list, legacy flat form, validation naming the
  offending field, file-shape preservation on save), `report.rs` (closed seven-state outcome model,
  `UpdateRun`, text rendering), `config_tests.rs`. `arch.rs` widened from Windows-only to a
  platform-and-architecture suffix table without changing the Windows values. `command.rs` gained
  `check_all` (per-target fan-out with failure isolation) and `check_update_with`. `Installer` now
  takes `&UpdateTarget` rather than the whole configuration.
- `crates/cli/src/main.rs`, `crates/updater/src/main.rs`: a legacy flat configuration keeps the
  historical output wording and exit status; an explicit `[[targets]]` file gets one line per
  target. `--target` is forwarded to the spawned updater and now scopes the exit status.
- `crates/tray/src/main.rs`: adapted to the target-based API.
- `config/updates.example.toml`: documents both the flat and the `[[targets]]` form.
- `docs/IMPLEMENTATION_STATUS.md`: records what the multi-target work delivers and, explicitly,
  what it does not yet deliver.
- `.specify/memory/constitution.md`: replaced the untouched Spec Kit scaffold with the first
  ratified principle set (v1.0.0): Fail-Closed Safety, Test Evidence Before Claims, Stable
  Protocol Surface, Minimal Review-Gated Change, Platform Boundaries Are Explicit; plus
  Technology Constraints, Development Workflow & Quality Gates, and Governance sections.
  Ratified 2026-11-12; first version, so the whole set counts as initial adoption.
- `crates/update/src/arch.rs`: architecture-to-suffix mapping (`x86_64` → `windows-x64.zip`,
  `aarch64` → `windows-arm64.zip`); unknown architectures error instead of falling back to x64.
- `crates/update/src/lib.rs`: `UpdateConfig.asset_pattern` is now `Option<String>`; an omitted key
  derives from the running architecture, an explicitly empty value is rejected.
- `crates/update/src/selection.rs`: asset matching is suffix-based and architecture-guarded; the
  configured pattern can narrow the search but never widen it to another architecture.
- `crates/protocol/src/lib.rs`: added the typed `Method` enum (`as_str`, `FromStr`, `ALL`) as the
  single source of IPC method names, pinned by tests.
- `crates/ipc/src/`: split into `lib.rs` (transport), `dispatch.rs` (one handler per method),
  `error.rs` (typed `DispatchError` with exhaustive wire codes) and `params.rs`.
- `crates/update/src/`: split into `lib.rs` (composition root + public API), `ports.rs`,
  `github.rs`, `install.rs`, `selection.rs`, `command.rs`, `params.rs`, `tests.rs`. The update
  pipeline now runs against the `Releases`/`Downloader`/`Installer` ports; the Windows install
  body is unchanged.
- `crates/cli/src/main.rs`, `crates/updater/src/main.rs`: use the shared
  `set_or_get_channel`/`check_update` command bodies and the `Method` enum.
- `crates/tray/src/main.rs`: uses the `Method` enum.
- `crates/update/src/lib.rs`: adapted archive extraction to `zip` 2.6.1
  (`enclosed_name()` returns `PathBuf`; added a missing statement terminator).
- `crates/tray/src/main.rs`: migrated from deprecated `EventLoop::run` to
  `ApplicationHandler::run_app`.
- `crates/cli/src/main.rs`: fixed the `UpdateCommand::Install` binding that broke the Windows build.
- `.guidance/`: full generated configuration, verified with `setup_guidance_verify` (23/23).
- `.github/workflows/ci.yml`: the temporary debug steps were removed (RW-003).

## Recent Decisions

- The update configuration is modelled as a raw `ConfigFile` (all fields optional) plus a
  normalized `UpdateConfig`, deliberately not as a serde untagged enum: an untagged enum reports
  "data did not match any variant", and the specification requires the offending field to be named.
- `UpdateConfig` records the file shape it was loaded with, because a channel change must not
  reshape a legacy flat file into the array form. An independent review caught this as an FR-003
  violation before it reached the branch.
- The legacy path is a separate render branch, not a special case inside per-target rendering,
  because SC-001 requires byte-identical output for a file the operator never changed.
- A foreign target reports `not installed` rather than borrowing the runtime's own version. That is
  honest but means only the runtime target reports a real installed version until the per-target
  marker lands.
- The constitution encodes existing practice (`AGENTS.md`, CI workflows, `docs/SECURITY.md`) rather
  than introducing new rules; each principle names the failure that motivated it.
- Guidance configuration uses template `rust`, transport `http`, GitNexus `optional` with
  `local-cli` index refresh, Insight downstream enabled, `gates = standard`.
- `.guidance/scripts/*.mjs` live directly under `.guidance/scripts/`, matching the paths
  referenced in `operations.json`.

## Open Questions

- [ ] Who is the project owner (person or team) for `projectBrief.md`?
- [ ] Who are the primary users — workstation operators, or a wider developer audience?
- [ ] Which role approves amendments to the constitution (the ratification date is fixed at
  2026-11-12)?

## Blockers

- `projectBrief.md` and `productContext.md` cannot be completed without product/ownership input.

## Next Actions

1. Commit the ratified constitution as a constitution-only change, separately from the feature work
   that shares this working tree (a reviewer flagged the mix against the Governance section).
2. Continue feature 001 with US2 (T016-T022). When routing private targets through the HTTP
   adapter, fix the adapter's error text: it currently carries the full repository URL, which would
   disclose the private path and violate FR-014.
3. Answer the open questions above, then fill `projectBrief.md` and `productContext.md`.
4. Restart the Guidance server so the new `.guidance/` configuration is snapshotted.
5. Address RW-004/RW-005: add a service-control port so the backup/restore transaction becomes
   testable.

## Last Updated

2026-11-12 (agent session: feature 001 phases 1-3 implemented)
