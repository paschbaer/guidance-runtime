# ADR-0009: Platform-architecture asset suffix table

- **Status:** Accepted (2026-11-12)
- **Context:** Release artifacts are published per platform and architecture
  (`windows-x64.zip`, `linux-aarch64.zip`, …). The original implementation
  derived a hardcoded `windows-x64.zip` pattern, which made arm64
  installations undeliverable: selection either failed or handed them a
  foreign archive. Hand-maintaining the table risks drift with the release
  workflow matrix, and letting a configured pattern widen the search would
  defeat the point.
- **Decision:** `crates/update/src/arch.rs` owns the closed table
  (`KNOWN_ARCH_SUFFIXES`) mapping each supported platform/architecture pair to
  its asset suffix; unknown platforms and architectures are rejected by name
  rather than silently falling back. The expected suffix is derived from the
  *running* architecture (`running_arch()`), and `selection::select` receives
  it separately from any configured `asset_pattern`: a pattern can narrow the
  search (e.g. pin a filename convention) but can never widen it beyond the
  running architecture. A newer release with no artifact matching the running
  pair is `invalid_artifact`, never `up_to_date` (ADR-0005).
- **Consequences:** Every supported architecture is deliverable and every
  unsupported one fails loudly. Adding an architecture requires exactly two
  coordinated edits (the table and the release workflow matrix) — accepted
  manual coupling, tracked as RW-007, with a committed manifest as the future
  fix.
- **Evidence:** `crates/update/src/arch.rs`, `crates/update/src/selection.rs`
  (`select_detailed`), `memory-bank/remaining-work-plan.md` (RW-007),
  `memory-bank/systemPatterns.md` (design pattern
  "Architecture-aware asset selection").
