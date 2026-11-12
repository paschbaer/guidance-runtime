# Product Context

> Derived from `README.md` and `docs/IMPLEMENTATION_STATUS.md`. Items marked **(assumption)** are
> inferred and need product confirmation.

## Problem Statement

Running several MCP servers on a workstation means starting, stopping, restarting, and watching
each process by hand, with no shared status view and no safe way to roll out updates. GRS centralizes
that supervision behind one service and one control surface.

## User Needs

- As a **workstation operator**, I need to start, stop, and restart my MCP workloads without
  hunting for processes, so that I can switch context quickly.
- As a **developer**, I need one place that shows whether each workload is running, so that I can
  tell "broken" from "not started".
- As a **maintainer**, I need updates to apply only when verified and to roll back on failure, so
  that a bad release cannot leave a machine without a working service.

## Core User Journeys

### Journey 1: Supervise workloads from the tray

1. The operator installs and starts the `GuidanceRuntime` service.
2. The tray shows the current status; "Refresh" re-reads it.
3. "Start All Enabled" / "Stop All" act on every enabled workload in `config/runtime.toml`.

**Expected outcome:** Status and bulk control with no terminal work.

### Journey 2: Check the same state from the CLI

1. `guidance status` / `guidance list` queries the runtime over IPC.
2. `guidance start <id>` / `guidance stop <id>` controls a single workload.
3. `guidance logs <id> --tail <n>` returns the most recent output.

**Expected outcome:** Scriptable control with the same semantics as the tray.

### Journey 3: Apply an update

1. `guidance update check` reports available versions for the configured channel.
2. `guidance update install --yes` hands off to `guidance-updater`.
3. The updater stops the service, backs up the install, replaces binaries, restarts, and restores
   the backup if anything fails.

**Expected outcome:** An updated installation, or the previous one intact.

## Domain Model and Terminology

- **Workload:** one supervised MCP server process, configured in `config/runtime.toml` with `id`,
  `command`, `args`, `enabled`, `endpoint`, and a restart policy.
- **Runtime:** the supervision component (`guidance-runtime`) holding workload state.
- **Channel:** the release line a client follows — `stable` or `beta`.
- **Asset:** a release artifact; the updater requires the Windows x64 ZIP plus its SHA-256 asset.
- **Backup / rollback:** the copy of the previous installation kept during an update.

Use these terms consistently in code, documentation, and interfaces.

## Business Rules

1. A release without a SHA-256 asset is refused; updates are never applied unverified.
2. A release whose existing tag or release name already exists is never overwritten.
3. Installation is Windows-only; other platforms fail with a clear error.
4. The service is stopped before binaries are replaced and restarted afterwards, with the backup
   restored if either step fails.
5. A commit with `[skip release]` in the head message produces no release.

## User Experience Principles

- Prefer clear and predictable behavior over hidden automation.
- Provide actionable error messages that name the failed operation.
- Preserve user data and existing configuration by default — an update rolls back rather than
  leaving a partial installation.

## Assumptions

- The primary audience is a single developer workstation, not a shared server: single-user
  control over a local service.
- Workload binaries are installed by the user; GRS supervises them but does not fetch them.

## Open Product Questions

- [ ] Who is the owner of the repository, and who are the intended primary users?
- [ ] Is the Linux/macOS support in the specification a committed roadmap item or aspirational?
- [ ] Which hardening items from `docs/IMPLEMENTATION_STATUS.md` are required before a first
      "production" release?

## References

- Requirements: `docs/SPECIFICATION-v1.1.md`
- Designs or prototypes: `README.md` (usage and release flow)
- Related decisions: `memory-bank/systemPatterns.md`, `memory-bank/remaining-work-plan.md`
