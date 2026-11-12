# Progress

## Overall Status

**Phase:** Repository Ready (partially complete)

**Summary:** The workspace builds cleanly under CI's strict clippy gate and the Guidance
configuration exists and is verified. Project-brief and product-context documentation still
depend on input from the maintainers.

## Completed

- [x] Create the Memory Bank directory structure.
- [x] Fix the release build so `cargo clippy --workspace --all-targets -- -D warnings` passes:
      `zip` 2 API adaptation, winit `ApplicationHandler` migration, CLI install binding.
- [x] Establish a green CI baseline on Windows (fmt, clippy, test).
- [x] Generate and verify the `.guidance/` configuration (template `rust`, 23 files,
      `setup_guidance_verify` reports `verified: true`).
- [x] Register the repository in the Guidance workspace registry as `guidance-runtime`.
- [x] Document the actual toolchain and commands in `techContext.md`.
- [x] Document the architecture in `systemPatterns.md`.
- [x] Fix the SOLID review findings: IPC dispatch extraction (RW-001), typed `Method` enum,
      update ports behind `Releases`/`Downloader`/`Installer`, shared update commands.
- [x] Raise workspace test coverage from 2 to 40 tests around the update pipeline and IPC mapping.
- [x] Track `Cargo.lock` (RW-002).

## In Progress

- [ ] Complete `projectBrief.md` (owner and primary users still open).
- [ ] Document user needs and domain rules in `productContext.md`.

## Planned

- [ ] Define the first milestone and acceptance criteria.
- [ ] Add a service-control port so the backup/restore transaction becomes testable (RW-005).
- [ ] Record initial architectural decisions as ADRs.
- [ ] Validate the development environment end to end (install/uninstall via `scripts/*.ps1`).

## Known Issues

- `projectBrief.md` and `productContext.md` still contain template placeholders.
- The Windows backup/restore transaction is still untested (RW-005).
- IPC handlers have no direct test seam until a supervisor port exists (RW-006).

## Milestones

### Milestone 1: Repository Ready

**Status:** In progress

**Exit criteria:**

- [ ] Project brief is complete.
- [x] Build, test, lint, and run commands are documented and verified.
- [x] Architecture and important boundaries are documented.
- [x] Repository instructions reference the Memory Bank.
- [ ] No unresolved critical setup issues remain.

## Change Log

### Initial Setup

- Initialized the Memory Bank.

### 2026-11-12 — Build and Guidance setup

- Repaired the workspace build for `-D warnings` (three defects, two of them Windows-only).
- Created and verified the `.guidance/` configuration; registered the workspace.
- Replaced the template placeholders in `techContext.md`, `systemPatterns.md`,
  `activeContext.md`, `progress.md`, `remaining-work-plan.md`, and `lessonsLearned.md`.

### 2026-11-12 — SOLID review fixes

- `guidance-protocol`: typed `Method` enum replaces scattered method string literals (OCP).
- `guidance-ipc`: dispatch split into named handlers with a typed `DispatchError` (SRP).
- `guidance-update`: ports (`Releases`, `Downloader`, `Installer`) with HTTP/ZIP adapters (DIP);
  pure selection logic and checksum parsing are unit-tested; Beta channel rule is explicit.
- CLI and updater share `set_or_get_channel`/`check_update` (DRY); install paths stay separate.
- `Cargo.lock` tracked; workspace test count 2 → 40.
