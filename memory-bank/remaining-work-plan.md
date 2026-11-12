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

- **Finding:** `crates/ipc/src/lib.rs` implements the whole method dispatch as one dense
  expression, with multiple match arms compressed into a single line. It is hard to review and
  hard to extend safely.
- **Trigger point:** The next change that adds or modifies an IPC method.
- **Action:** Required — extract one function per method (or a dispatch table) before adding
  another arm. Pre-existing violation of the SOLID SRP obligation in `AGENTS.md`.
- **Origin:** Session 2026-11-12 (structural observation while documenting `systemPatterns.md`).
- **Status:** Open

### RW-002 — `Cargo.lock` is untracked

- **Finding:** `Cargo.lock` is present locally (102 KB) but listed in `.gitignore` and not
  tracked. For a workspace that ships binaries (service, CLI, tray, updater) this forfeits
  reproducible builds; dependency versions can drift between releases.
- **Trigger point:** Before the next stable release, or as soon as a dependency-resolution
  surprise appears in CI.
- **Action:** Required — decide explicitly: either track `Cargo.lock` (recommended for binary
  crates) or document why the release workflow resolves the lockfile itself.
- **Origin:** Session 2026-11-12 (commit sequence that added and then removed the file).
- **Status:** Open

### RW-004 — Minimal test coverage with untested transaction logic

- **Finding:** The workspace contains 2 tests in total, while `guidance-update` implements a
  multi-step, destructive transaction (stop service, back up, extract, replace, restart, restore
  on failure) that is exercised on every update.
- **Trigger point:** The next change to `guidance-update` or `guidance-updater`, at the latest
  before the next stable release.
- **Action:** Required — add focused tests for SHA-256 verification, archive path traversal
  rejection, and the backup/restore path.
- **Origin:** Session 2026-11-12 (`systemPatterns.md` → Known Technical Risks).
- **Status:** Open

## Handled Follow-Ups

### RW-003 — Temporary debug steps in CI

- **Finding:** `.github/workflows/ci.yml` contained two steps that dumped `ci.yml` and
  `release.yml` (`Show ci.yml`, `Dump workflow files`).
- **Action:** Handled — the steps were removed and replaced by `Swatinem/rust-cache@v2` plus a
  `cargo build --workspace --release` step in commit `d644d65`.
- **Origin:** Session 2026-11-12 (committed as `22105f7`, removed in `d644d65`).
- **Status:** Handled

## Accepted Observations

None yet.
