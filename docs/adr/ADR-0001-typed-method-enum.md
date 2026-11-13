# ADR-0001: Typed `Method` enum with exhaustive IPC dispatch

- **Status:** Accepted (2026-11-12)
- **Context:** IPC requests originally carried the method as a raw string that
  `guidance-ipc` matched on. Adding or renaming a method meant touching string
  literals in every client, and a typo produced a runtime `method not found`
  instead of a compile error. The dispatch itself was also written as one dense
  expression with several arms compressed onto a single line (review finding
  RW-001), which was hard to review and to extend safely.
- **Decision:** `guidance-protocol` owns a typed `Method` enum whose wire names
  are pinned by a test (`wire_names_are_pinned` in the protocol crate's
  `FromStr`/`as_str`). `guidance-ipc` dispatches through `dispatch.rs` with one
  named handler per method over an *exhaustive* `match` on `Method`, backed by a
  typed `DispatchError` with an exhaustive wire-code mapping (`error.rs`) and
  parameter parsing isolated in `params.rs`. No client may send a raw method
  string.
- **Consequences:** The compiler enforces both sides: a new variant breaks every
  `match` until a handler exists, and a renamed wire name breaks the pinning
  test. Unknown methods still fail at runtime with
  `method not found: <name>` (`not_found`) for old clients. The cost is a
  protocol-crate change for every new method — accepted, because methods are
  few and stable.
- **Evidence:** `crates/protocol/src/lib.rs`, `crates/ipc/src/dispatch.rs`,
  `crates/ipc/src/error.rs`, `memory-bank/remaining-work-plan.md` (RW-001),
  `memory-bank/systemPatterns.md` (design pattern "Typed method enum").
