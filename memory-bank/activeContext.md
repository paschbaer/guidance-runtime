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
- [ ] Complete `projectBrief.md` — blocked on owner and primary-user input.
- [ ] Complete `productContext.md` — user journeys need product input.
- [ ] Decide whether `Cargo.lock` should stay untracked (see RW-002).

## Recent Changes

- `crates/update/src/lib.rs`: adapted archive extraction to `zip` 2.6.1
  (`enclosed_name()` returns `PathBuf`; added a missing statement terminator).
- `crates/tray/src/main.rs`: migrated from deprecated `EventLoop::run` to
  `ApplicationHandler::run_app`.
- `crates/cli/src/main.rs`: fixed the `UpdateCommand::Install` binding that broke the Windows build.
- `.guidance/`: full generated configuration, verified with `setup_guidance_verify` (23/23).
- `.github/workflows/ci.yml`: temporarily adds two debug steps that dump `ci.yml` and
  `release.yml` (see RW-003).

## Recent Decisions

- Guidance configuration uses template `rust`, transport `http`, GitNexus `optional` with
  `local-cli` index refresh, Insight downstream enabled, `gates = standard`.
- `.guidance/scripts/*.mjs` live directly under `.guidance/scripts/`, matching the paths
  referenced in `operations.json`.

## Open Questions

- [ ] Who is the project owner (person or team) for `projectBrief.md`?
- [ ] Who are the primary users — workstation operators, or a wider developer audience?
- [ ] Is the untracked `Cargo.lock` intentional for this binary workspace?

## Blockers

- `projectBrief.md` and `productContext.md` cannot be completed without product/ownership input.

## Next Actions

1. Answer the open questions above, then fill `projectBrief.md` and `productContext.md`.
2. Remove or keep the debug steps in `.github/workflows/ci.yml` (RW-003).
3. Restart the Guidance server so the new `.guidance/` configuration is snapshotted.
4. Consider the first focused tests for `guidance-update` (see `systemPatterns.md` risks).

## Last Updated

2026-11-12 (agent session: clippy fixes, Guidance setup, Memory Bank initialization)
