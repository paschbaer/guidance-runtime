# Dispatch prompt template (plain guidance workflow, one item)

Copy this skeleton for every `spawn_agent` call. Replace every `<...>` placeholder, including `<main-workspace>` and `<item-id-slug>` used to build the derived temporary workspace name. Keep the `## Execution mode` block verbatim — it is the pre-spawn integrity marker.

---

You are the implementation agent for exactly ONE work item (`<item-id>`) of the <run name>. Work only inside your worktree.

## Execution mode
Execute this fix as a guidance workflow:
start_workflow { workspace: <temp workspace name>, request: <item text below> }
Follow the guidance-plain-workflow skill (Mode 1): drive phases with
plan-level submissions (understanding → plan → implementation →
verification → completion), read get_current_guidance after every
transition, respect phase gates — a red gate is a finding, not a
bypass. Do NOT call import_spec_kit_artifacts or any spec-kit task
tool; your workflow is a plain session owned by you.

Workspace identity: a git worktree resolves to a different root than the main
repository, and the main workspace name MUST NOT be used for your session.
Register a DERIVED TEMPORARY workspace name instead:
- name: `<main-workspace>-<item-id-slug>` (lowercase, hyphens only, e.g.
  `forge-013-rv-04`; must match `^[a-z][a-z0-9-]{0,63}$`),
- root: the ABSOLUTE worktree path,
- projectName: the project name of the main workspace.
Call `registry_register { name, root, projectName }` once, run the whole
workflow under that temporary name, and DEREGISTER it as the very last action
(see `## Report back`).

## Task
<item-id> — <title>

<finding quoted verbatim>

Required outcome:
- <outcome 1>
- <outcome 2>

Constraints:
- <non-goals, e.g. no new dependencies, no behaviour change outside the item>
- Regression tests are mandatory; security items additionally need the negative test.

## Context
Read before planning (repo root `<repo path>`; your worktree is a full copy):
- `AGENTS.md` (hard rules, SOLID, Findings Lifecycle) and `.specify/memory/constitution.md`
- `<contract/ADR/spec files>`
- `<affected source files, with symbol names>`

## Worktree
Worktree: `<absolute worktree path>` (branch `<fix/item-id>`, based on `<base commit>`). cd there first. Never touch the main repository directly.
Write scope: `<glob list>` — modify ONLY files inside this scope. No other crates, no new dependencies.

## Validation
Run inside the worktree:
- `cargo test -p <crate>`
- `cargo fmt --check`
- `cargo clippy -p <crate> --all-targets -- -D warnings`
- <additional item-specific checks>

No npm/node commands. If the linker fails with LNK1318 or "no space on device", delete `rust/target/debug/incremental` and retry once.

Commit inside the worktree with a Conventional Commit message (English) referencing `<item-id>`. Do NOT push, do NOT merge, do NOT delete the worktree.

## Report back
Before your final message, DEREGISTER the temporary workspace — this is your last tool action, after `complete_workflow` succeeded:

    registry_register { name: "<temp workspace name>", root: "<absolute worktree path>", remove: true }

The input schema requires both `name` and `root` with `minLength: 1` and validates them BEFORE the remove branch runs; `root` is ignored while removing, but send the real absolute worktree path anyway. Never remove the main workspace, never omit `root`, never invent a different name than the one you registered.

Your FINAL message must contain:
(a) the guidance workflow session id and the phase reached (or the exact start_workflow error),
(b) files created/changed (paths),
(c) commands executed with exit codes and the key output lines,
(d) deviations from the task text (if none, say "none"),
(e) confirmation that the write scope was respected,
(f) the commit hash(es) created in the worktree,
(g) the temporary workspace name you registered and confirmation that `registry_register { remove: true }` succeeded (or the exact error, in which case the orchestrator must clean it up).

---

## Re-dispatch after a failed attempt

Append this section and quote the failure verbatim:

    ## Corrections
    The previous attempt failed: <verbatim error / missing evidence>.
    Address exactly this in the new attempt; keep the rest of the task unchanged.

## Reviewer prompt skeleton

```
You are an INDEPENDENT REVIEWER for <run name>. You did not build this code.
Review the integrated branch state (read-only) against the items below and
report findings in your final message.

Repository state: <repo path>, branch <branch>, HEAD <hash>, clean tree.
Read AGENTS.md (Code Review Protocol) and .specify/memory/constitution.md.

Items under review: <id → title → expected outcome, one line each>
Changed files: <list>

Check: task conformance, evidence quality (run the tests yourself),
security/fail-closed behaviour, write-scope compliance, SOLID boundaries,
contract conformance, no weakened assertions.

Output per finding: id, severity (high|medium|low|info), fixRequired (true only
for real defects), evidence row (file/symbol, how verified, pre-existing vs.
introduced). End with the count of open HIGH/CRITICAL findings.
Do NOT modify any file.
```
