# Active Context

> This file describes the current working state. Keep it concise and update it after substantial work.

## Current Focus

Stabilizing the release build and establishing project context (Memory Bank) plus the Guidance
workflow configuration.

## Current Objective

Get `cargo clippy --workspace --all-targets -- -D warnings` green on both CI matrix OSes and
replace the Memory Bank template placeholders with repository-specific content.

## Current Work

- [x] Fix the release build for `-D warnings` (zip 2 API, winit `ApplicationHandler`, CLI binding).
- [x] Create and verify the `.guidance/` configuration (template `rust`, 23 files).
- [x] Register the repository in the Guidance workspace registry.
- [x] Resolve the SOLID review findings (RW-001 plus OCP/DIP/DRY) on `feature/solid-findings-fix`.
- [x] Decide `Cargo.lock` tracking — tracked since `b9f817d` (RW-002 Handled).
- [ ] Complete `projectBrief.md` — blocked on owner and primary-user input.
- [ ] Complete `productContext.md` — user journeys need product input.

## Recent Changes

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

- Guidance configuration uses template `rust`, transport `http`, GitNexus `optional` with
  `local-cli` index refresh, Insight downstream enabled, `gates = standard`.
- `.guidance/scripts/*.mjs` live directly under `.guidance/scripts/`, matching the paths
  referenced in `operations.json`.

## Open Questions

- [ ] Who is the project owner (person or team) for `projectBrief.md`?
- [ ] Who are the primary users — workstation operators, or a wider developer audience?

## Blockers

- `projectBrief.md` and `productContext.md` cannot be completed without product/ownership input.

## Next Actions

1. Answer the open questions above, then fill `projectBrief.md` and `productContext.md`.
2. Restart the Guidance server so the new `.guidance/` configuration is snapshotted.
3. Address RW-004/RW-005: add a service-control port so the backup/restore transaction becomes
   testable.
4. Consider RW-006 (handler test seam via a supervisor trait) when the next handler change lands.

## Last Updated

2026-11-12 (agent session: SOLID review fixes — IPC dispatch, Method enum, update ports, DRY)
