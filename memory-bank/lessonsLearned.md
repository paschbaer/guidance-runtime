# Lessons Learned

This document captures reusable, evidence-based knowledge from implementation work, incidents,
experiments, reviews, and successful practices.

Do not use it as a chronological activity log. Only record lessons that are likely to influence
future decisions or prevent repeated mistakes.

## How to Add a Lesson

For every lesson, include:

- **Date:** When the lesson was validated
- **Context:** What was being attempted
- **Observation:** What happened
- **Root cause:** Why it happened, if known
- **Lesson:** The reusable insight
- **Action:** What should be done differently
- **Evidence:** Relevant issue, pull request, log, test, ADR, or source file
- **Status:** Proposed, validated, or superseded

## Validated Lessons

### Platform-gated code hides compile errors from the local build

- **Date:** 2026-11-12
- **Context:** Making `cargo clippy --workspace --all-targets -- -D warnings` pass on Windows and
  Ubuntu.
- **Observation:** Two defects (`Path::to_path_buf` type mismatch from the `zip` 2 API change and
  a missing statement terminator) plus one wrong binding in `crates/cli/src/main.rs` appeared only
  in code paths that the other OS never compiles.
- **Root cause:** `#[cfg(windows)]` / `#[cfg(not(windows))]` arms are invisible to the compiler on
  the other platform, so a local build on one OS cannot prove the other OS compiles.
- **Lesson:** CI on both matrix OSes is the only authoritative signal for
  `cfg`-split code; a green local build proves roughly half of it.
- **Action:** Run `cargo clippy --workspace --all-targets -- -D warnings` (not `cargo build`)
  before pushing, and read the CI result for *both* OSes — an Ubuntu-only green is not success.
- **Evidence:** `crates/update/src/lib.rs`, `crates/cli/src/main.rs`,
  `.github/workflows/ci.yml`
- **Recurrence 2026-11-12 (SOLID refactor):** `crates/update/src/install.rs` imported
  `std::{fs, path::Path}` unconditionally while every use sat inside `#[cfg(windows)]` arms. The
  Windows clippy run stayed green; Ubuntu failed with `unused_imports`. Fixed by gating the import.
  The stronger action is now: when adding or moving code behind any `cfg(...)` gate, import inside
  the gated block instead of at module level, so the other platform cannot see an unused import.
- **Status:** Validated (twice)

### Generated configuration must be written byte-exactly

- **Date:** 2026-11-12
- **Context:** Persisting the files emitted by `setup_guidance_generate` and passing
  `setup_guidance_verify`.
- **Observation:** Verification failed twice: (1) the file tool wrote CRLF where the manifest
  counted LF, and (2) the manifest paths for scripts were `.guidance/`-prefixed in an unexpected
  way, so writing them "relative" created a nested `.guidance/.guidance/scripts/`.
- **Root cause:** The manifest records exact UTF-8 byte sizes including their line endings, and
  its paths are not uniformly relative to `.guidance/`.
- **Lesson:** Treat a generation manifest as a checksum contract: reconcile path *and* exact byte
  size per file before reporting completion, and never assume a uniform path base.
- **Action:** After writing generated files, run the verify tool immediately; if sizes mismatch,
  compare actual line endings (`\n` vs `\r\n`) instead of rewriting content.
- **Evidence:** `setup_guidance_verify` output; `.guidance/` tree
- **Status:** Validated

### A dependency major-version gap is an API contract change

- **Date:** 2026-11-12
- **Context:** `zip = "2"` in the workspace while the extraction code was written against the 0.6
  API.
- **Observation:** `enclosed_name()` had changed from `Option<&Path>` to `Option<PathBuf>`, so
  `map(Path::to_path_buf)` no longer type-checked.
- **Root cause:** The code compiled against an older major version than the lockfile/workspace
  resolves; the mismatch stayed latent until the crate was built.
- **Lesson:** When a crate fails to compile after a dependency bump, read the actual function
  signature in the resolved version instead of patching the call site by intuition.
- **Action:** Check `cargo tree -p <crate>` and `cargo info <crate>` to confirm the resolved
  version before adapting code.
- **Evidence:** `crates/update/src/lib.rs`, `Cargo.toml`
- **Status:** Validated

## Proposed Lessons

None proposed.

## Superseded Lessons

None superseded.

## Entry Template

Copy this skeleton when adding a lesson (the `TODO` markers below are intentional — they
only ever appear inside this template):

```markdown
### Short Descriptive Title

- **Date:** YYYY-MM-DD
- **Context:** TODO
- **Observation:** TODO
- **Root cause:** TODO
- **Lesson:** TODO
- **Action:** TODO
- **Evidence:** TODO
- **Status:** Proposed | Validated | Superseded
```
