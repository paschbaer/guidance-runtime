# Remaining Work Plan

Durable record of unresolved review findings and deferred work, as required by the Findings
Lifecycle rules in `AGENTS.md`. Reviewer reports and chat summaries do not count as
documentation.

## Entry Format

For every tracked follow-up, include:

- **ID:** Short stable identifier
- **Finding:** What was found or deferred
- **Trigger point:** The concrete future scope, stage, or condition under which it must be handled
- **Action:** Required action, or accepted observation with rationale
- **Origin:** Review, session, or decision that produced it; link where useful
- **Status:** Open | Handled | Reclassified

## Rules

- When a future scope begins, check open follow-ups for matching trigger points; handle them or
  re-schedule them with a new trigger point.
- A follow-up may only be removed or moved to `Handled` when it is fixed with regression coverage
  or explicitly reclassified with evidence (e.g., verified false positive or subsumed by another
  change).
- Keep entries concise and current; remove obsolete information instead of letting contradictions
  accumulate.
- Never store credentials, tokens, personal data, or secrets here.

## Open Follow-Ups

### RW-001 — Dense IPC dispatch violates SRP

- **Finding:** `crates/ipc/src/lib.rs` implemented the whole method dispatch as one dense
  expression, with multiple match arms compressed into a single line. It was hard to review and
  hard to extend safely.
- **Action:** Handled — commit `feature/solid-findings-fix`: dispatch split into `dispatch.rs`
  (one named handler per method over an exhaustive `match` on `guidance_protocol::Method`),
  typed `DispatchError` with an exhaustive wire-code mapping in `error.rs`, parameter parsing in
  `params.rs`. Covered by 13 unit tests. The `.contains("not found")` heuristic is replaced by an
  explicit typed classification.
- **Origin:** Session 2026-11-12.
- **Status:** Handled

### RW-002 — `Cargo.lock` is untracked

- **Finding:** `Cargo.lock` was present locally but listed in `.gitignore` and untracked, which
  forfeits reproducible builds for a workspace that ships four binaries.
- **Action:** Handled — commit `b9f817d` tracks `Cargo.lock`. Evidence verified 2026-11-12:
  `git ls-files Cargo.lock` returns one entry and `.gitignore` no longer lists it.
- **Origin:** Session 2026-11-12 (commit sequence that added and then removed the file).
- **Status:** Handled

### RW-004 — Minimal test coverage with untested transaction logic

- **Finding:** The workspace contained 2 tests in total, while `guidance-update` implements a
  multi-step, destructive transaction (stop service, back up, extract, replace, restart, restore
  on failure) that is exercised on every update.
- **Action:** Partially handled — `guidance-update` now exposes ports (`Releases`, `Downloader`,
  `Installer`) and covers channel/version selection, checksum parsing, checksum verification and
  verification rejection with 23 tests driven through fakes. The backup/restore transaction itself
  is still untested (see open follow-up RW-005).
- **Origin:** Session 2026-11-12 (`systemPatterns.md` → Known Technical Risks).
- **Status:** Open

### RW-005 — Windows backup/restore transaction remains untested

- **Finding:** The destructive install transaction in `crates/update/src/install.rs` (stop service
  → back up → replace → restart, restore on failure) has no test seam for `sc.exe` or the
  filesystem transaction. Archive path-traversal rejection IS covered (in-memory ZIP with a
  `../evil.exe` entry, Windows-gated); the backup/restore branches are not.
- **Trigger point:** The next change to the install transaction, or the first Windows CI job that
  can install a service.
- **Action:** Required — extract the service-control calls behind a small port and drive the
  backup/restore branches with a fake in a temp directory.
- **Origin:** Session 2026-11-12 (plan review finding F2 / remaining concern 2).
- **Status:** Open

### RW-006 — IPC handlers have no direct test seam

- **Finding:** `Runtime::load` requires a config file with real command paths and `start()` spawns
  a child process, so the extracted IPC handlers cannot be exercised in isolation. Only the pure
  parts (error-code mapping, parameter parsing, tail clamping) are unit-tested.
- **Trigger point:** The next change to handler behaviour, or when a supervisor trait is
  introduced.
- **Action:** Accepted observation until a `Supervisor` port exists; then add handler-level tests.
- **Origin:** Session 2026-11-12 (plan review finding F2).
- **Status:** Open

### RW-007 — Architecture suffix table is hand-maintained

- **Finding:** `KNOWN_ARCH_SUFFIXES` in `crates/update/src/arch.rs` must stay in sync with the
  release workflow matrix by hand. A new architecture is invisible to the guard until it is added
  to both places.
- **Trigger point:** Whenever the release workflow gains or renames a target.
- **Action:** Required at that point — add the suffix and a test; consider deriving the list from
  a committed manifest that the workflow also reads.
- **Origin:** Session 2026-11-12 (independent review of the arch-aware selection).
- **Status:** Open

### RW-009 — Amendment approver role is undesignated

- **Finding:** Constitution v1.0.0 was ratified on 2026-11-12, but the repository still names
  neither a project owner nor the role that approves amendments to the constitution.
- **Trigger point:** The first proposed amendment to `.specify/memory/constitution.md`.
- **Action:** Required — designate the amendment approver (and the project owner for
  `memory-bank/projectBrief.md`) and record both in the Governance section.
- **Origin:** Session 2026-11-12 (`speckit-constitution`, constitution v1.0.0).
- **Status:** Open

## Handled Follow-Ups

### RW-003 — Temporary debug steps in CI

- **Finding:** `.github/workflows/ci.yml` contained two steps that dumped `ci.yml` and
  `release.yml` (`Show ci.yml`, `Dump workflow files`).
- **Action:** Handled — the steps were removed and replaced by `Swatinem/rust-cache@v2` plus a
  `cargo build --workspace --release` step in commit `d644d65`.
- **Origin:** Session 2026-11-12 (committed as `22105f7`, removed in `d644d65`).
- **Status:** Handled

### RW-008 — Non-Windows `update check` reports a Windows-looking suffix

- **Finding:** On Linux the derived pattern was still `windows-*.zip`, so `guidance update check`
  succeeded with a meaningless result before `install` refuses. Not a correctness risk, but a
  confusing diagnostic.
- **Action:** Handled — the premise no longer holds. `UpdateTarget::resolved_asset_pattern` derives
  the suffix from `asset_suffix_for_arch(running_arch())`, and `arch.rs` maps Windows, macOS and
  Linux for x86_64 and aarch64. `an_omitted_pattern_derives_from_the_running_architecture` pins the
  expected per-host value in an explicit table, and the `ubuntu-latest` leg is green on commit
  `28737dd`, so a Linux host resolves `linux-x86_64.zip`. The finding also hid the real defect: the
  `guidance-update` fixtures themselves hard-coded `windows-*.zip` and are now derived.
- **Origin:** Session 2026-11-12 (independent review of the arch-aware selection); re-checked and
  closed 2026-11-13 while fixing the cross-platform fixtures.
- **Status:** Handled

## Accepted Observations

None yet.
