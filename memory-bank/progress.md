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
- [x] Raise workspace test coverage from 2 to 55 tests around the update pipeline and IPC mapping.
- [x] Track `Cargo.lock` (RW-002).
- [x] Draft the project constitution v1.0.0 from the resolved scaffold (five core principles plus
      technology, workflow, and governance sections).
- [x] Ratify the constitution v1.0.0 on 2026-11-12 and remove the temporary Sync Impact Report.
- [x] Implement Spec-Kit feature 001 phases 1-3: multi-target update configuration, seven-state
      outcome model, platform-aware archive suffix table, per-target fan-out with failure
      isolation, exit statuses, and CLI wiring. Workspace tests 41 → 91.

## In Progress

- [ ] Feature 001 User Stories 2-4 (T016-T040): private-source access, structured output,
      per-target installation, macOS CI leg.
- [ ] Complete `projectBrief.md` (owner and primary users still open).
- [ ] Document user needs and domain rules in `productContext.md`.

## Planned

- [ ] Define the first milestone and acceptance criteria.
- [ ] Add a service-control port so the backup/restore transaction becomes testable (RW-005).
- [ ] Record initial architectural decisions as ADRs.
- [ ] Validate the development environment end to end (install/uninstall via `scripts/*.ps1`).

## Known Issues

- The amendment approver role is still undesignated (RW-009).
- `projectBrief.md` and `productContext.md` still contain template placeholders.
- The Windows backup/restore transaction is still untested (RW-005).
- IPC handlers have no direct test seam until a supervisor port exists (RW-006).
- A private update target always reports `no access to update source`; private-source access is
  unimplemented (feature 001 US2).
- Only the runtime target reports an installed version; other targets report `not installed`
  because no per-target version marker exists yet.
- Every source-list failure reports as a network error, so an authorization problem cannot be
  distinguished from a transport problem for a public target.
- The HTTP adapter's error text contains the full repository URL, which would disclose a private
  target's path once private targets are routed through it.

## Milestones

### Milestone 1: Repository Ready

**Status:** In progress

**Exit criteria:**

- [ ] Project brief is complete.
- [x] Constitution is ratified (v1.0.0, 2026-11-12).
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
- `Cargo.lock` tracked; workspace test count 2 → 55.

### 2026-11-12 — Architecture-aware updates

- Asset selection derives its suffix from the running architecture; an omitted `asset_pattern`
  no longer defaults to x64. Unknown architectures and empty overrides fail loudly.
- The configured pattern can narrow the search but never select another architecture's archive.

### 2026-11-12 — Constitution v1.0.0

- Reproduced the resolved `constitution-template` (core layer, no overrides/presets/extensions).
- Replaced the Spec Kit scaffold in `.specify/memory/constitution.md` with five core principles
  grounded in `AGENTS.md`, CI gates, the IPC contract, and `docs/SECURITY.md`, plus Technology
  Constraints, Development Workflow & Quality Gates, and Governance sections.
- Ratified 2026-11-12; the temporary Sync Impact Report was removed. The amendment approver role
  remains open (RW-009).

### 2026-11-12 — Multi-target update sources (phases 1-3)

- `guidance-update` gained a target list: a raw `ConfigFile` plus a normalized `UpdateConfig`, so
  both the historical flat form and a `[[targets]]` array parse and every validation error names
  the offending field.
- Added a closed seven-state outcome model and `UpdateRun`, with both invariants enforced at
  construction.
- `arch.rs` widened from Windows-only to a platform-and-architecture suffix table; the Windows
  values are unchanged and pinned by a regression test.
- `check_all` fans out over the configured targets and isolates each failure; exit status is 0, 1
  or 2 per the feature contract.
- A legacy flat configuration keeps its historical output wording, exit status and file shape; an
  explicit `[[targets]]` file gets one line per target.
- Three independent reviews were run through the reviewer-token handoff. Four real defects were
  fixed as a result, including an FR-003 violation where a channel change reshaped a legacy file
  and an SC-001 gap where the legacy test proved a path the CLI never took.
- Workspace tests 41 → 91.
