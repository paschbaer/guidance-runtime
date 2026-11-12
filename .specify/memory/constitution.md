# Guidance Runtime Service Constitution

## Core Principles

### I. Fail-Closed Safety

Safety-relevant paths MUST fail closed: when a required precondition cannot be verified, the
operation MUST be refused rather than attempted.

- An update MUST NOT be applied unless the release provides a matching SHA-256 asset; a release
  without it is refused, never "applied with a warning".
- Archive entries MUST be constrained by `enclosed_name()` before extraction; an unsafe path
  aborts the extraction.
- A failed update transaction MUST restore the `<install>.backup` state and surface the original
  error; partial application MUST NOT be reported as success.
- Errors MUST name the failed operation and an actionable correction.

Rationale: the service replaces its own binaries and supervises other processes on an operator's
workstation. A partially applied or unverified update is worse than no update.

### II. Test Evidence Before Claims (NON-NEGOTIABLE)

No change may be described as working, fixed, or complete without executed evidence.

- Every behaviour change MUST carry a test, or an explicit statement of why no test seam exists
  and what manual check was performed instead.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` are the authoritative gates; they MUST be reported as run or as not run.
- CI results for BOTH matrix operating systems are the only authoritative signal for
  `cfg`-gated code. A green local build, or an Ubuntu-only green, is not success.
- Cross-compiled targets are verified structurally (for example `scripts/verify-arch.ps1` reading
  the PE machine field), not by executing them on the wrong host.
- Focused tests MUST be run first, and pre-existing failures labelled separately so unrelated
  regressions are not attributed to the current change.

Rationale: `lessonsLearned.md` records three separate defects that a single-OS build and an
unexecuted assumption each hid until CI or an operator hit them.

### III. Stable Protocol Surface

The IPC contract is a versioned public surface between the runtime service, the CLI, the tray, and
external automation.

- IPC method names MUST be declared exactly once in the typed `guidance_protocol::Method` enum;
  raw method strings are forbidden at call sites.
- Wire names MUST remain pinned by test; renaming or removing a wire name is a breaking change
  that requires an explicit versioning decision and a migration note.
- Dispatch MUST remain exhaustive over the method enum, and unknown methods MUST fail with a
  typed `not_found` error.

Rationale: clients are built and released independently of the service. A silently renamed wire
name breaks tray and CLI installs that the service cannot see.

### IV. Minimal, Review-Gated Change

Changes stay inside the requested scope, and every review finding is tracked to closure.

- Unrequested changes MUST NOT be made; unrequested findings are reported as observation plus
  options, and corrective action waits for explicit confirmation.
- Architecture follows SOLID boundaries with dependencies inverted through ports (`Releases`,
  `Downloader`, `Installer`) wired at the composition root; new code MUST respect that direction.
- Reviews MUST be performed with a fresh context window, and a HIGH or CRITICAL finding MUST NOT
  be dismissed without reproducible evidence.
- Every unresolved finding MUST be persisted in `memory-bank/activeContext.md` and
  `memory-bank/remaining-work-plan.md` with an explicit trigger point before scope closes.
- Bug fixes MUST document reproduction, root cause, impact, the compared fix strategies, and the
  verification plan.

Rationale: parallel agent work and a self-updating service both amplify the cost of scope creep
and silently dropped findings.

### V. Platform Boundaries Are Explicit

Platform-specific behaviour MUST be expressed at compile time and documented at the boundary.

- Windows-only paths MUST be `cfg(windows)`-gated, with a non-Windows arm that still compiles and
  returns a clear error instead of failing to build.
- Imports used only inside a `cfg`-gated block MUST be placed inside that block, so the other
  platform cannot see an unused import.
- New architectures require an entry in `KNOWN_ARCH_SUFFIXES` and a matching release workflow
  matrix leg; a hardcoded default is a defect.
- Platform behaviour claims MUST reference the CI leg that proves them.

Rationale: the workspace builds on Linux for a subset of crates while the service, tray, and
install paths are Windows-only; a boundary expressed by convention alone has already produced
release-blocking defects.

## Technology Constraints

**Scope.** A Rust (edition 2021) Cargo workspace with eight member crates. Windows is the
implementation target for service, tray, and update installation; Linux and macOS are planned for
later releases only.

**Dependencies.** Versions MUST be declared centrally in `[workspace.dependencies]` and
constrained once. A `Cargo.lock` MUST be committed; a dependency major-version bump MUST be treated
as an API contract change and adapted against the resolved signature, not by intuition.

**Configuration and secrets.** Runtime configuration lives in local, untracked
`config/runtime.toml` and `config/updates.toml`, created from the tracked `*.example.toml` files.
Secrets MUST NOT appear in the repository, in TOML files, or in documentation; environment
variables are documented by name only.

**Distribution.** Releases are GitHub Releases produced only from `main` (stable) and `develop`
(beta). Every packaged executable MUST be architecture-verified before upload. Artifacts MUST NOT
be published by bypassing the release workflow.

**Documentation scope.** `docs/SPECIFICATION-v1.1.md` records intent, not current state; the code
is the fact, and `docs/IMPLEMENTATION_STATUS.md` is the authoritative statement of what is still
open for production.

## Development Workflow & Quality Gates

**Branching.** Work happens on `feature/<meaningful-name>`; `main` and `develop` MUST NOT receive
direct commits. Work merges `feature/*` → `develop` (rebase) → PR → `main` (squash). Feature
branches are deleted after merge.

**Pre-submit gates.** Before a review request or push: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Documentation
(`README.md`, `README.de.md`) MUST be updated in the same change when behaviour or configuration
changes.

**Review.** Reviews run through the review-agent protocol with a recorded snapshot (branch, HEAD,
diff status, files read, tests executed). An approval MUST state the explicit count of unresolved
HIGH/CRITICAL findings, which MUST be zero.

**Knowledge continuity.** Durable knowledge lives in `memory-bank/` and MUST be read before
substantial work and updated after it: `activeContext.md` and `progress.md` always,
`systemPatterns.md` on architectural change, `lessonsLearned.md` on recurring or non-obvious
failures. Memory-bank files are the source of truth for project state.

**Language.** All artifacts — code, identifiers, comments, documentation, commit messages,
configuration, and generated files — MUST be written in English. Conversation language with the
operator is not governed here.

**Commits.** Messages follow conventional commits and stay in the imperative mood. `[skip release]`
suppresses a release for exactly one commit and MUST NOT be used to hide an intended release.

## Governance

This constitution supersedes conflicting project practice. Where `AGENTS.md`, CI configuration, and
this document disagree, this document governs, and the conflict MUST be resolved by amending one of
them — not by convention drift.

**Amendments.** Any principle change MUST be proposed with its rationale, reviewed as a
constitution-only change, and merged with the Sync Impact Report removed. Amendments MUST NOT also
carry unrelated implementation work.

**Versioning.** Semantic versioning: MAJOR for an incompatible removal or redefinition of a
principle, MINOR for a new principle or materially expanded guidance, PATCH for clarifications and
wording. The version line below MUST match the amendment record.

**Compliance review.** Every review MUST verify compliance with Principles I–V and record any
deviation with justification. Complexity or deviation that cannot be justified MUST be rejected.
Open production-hardening work is tracked in `docs/IMPLEMENTATION_STATUS.md`; this constitution
governs how that work is done, not whether it is finished.

**Version**: 1.0.0 | **Ratified**: 2026-11-12 | **Last Amended**: 2026-11-12

<!-- Version: 1.0.0 | Ratified: 2026-11-12 | Last Amended: 2026-11-12 -->
