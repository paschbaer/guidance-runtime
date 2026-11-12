---
name: plain-workflow-orchestration
description: Orchestrate a set of independent, non-Spec-Kit work items (review follow-ups, tracked findings, migrations) as chained or parallel plain guidance workflows in isolated git worktrees. Use when a handover prompt or findings list names several items with disjoint write scopes and you must run them with fresh subagents, waves, exclusive-resource serialization and evidence gates — without any Spec-Kit feature folder.
---

# Plain Workflow Orchestration (no Spec-Kit)

## Purpose

Drive N independent work items through **plain guidance workflows** (`guidance-plain-workflow`, Mode 1), chained (sequentially) or parallel (waves of up to 3 fresh subagents in worktrees), and integrate the results yourself. This is the generalized form of the parallel execution model: no `specs/<id>/tasks.md`, no `import_spec_kit_artifacts`, no `release_batch`/`start_task`/`verify_task` tools.

You are the **orchestrator**: you own the branch, the item manifest, the wave plan, worktrees, integration merges, gate runs and the memory-bank updates. You do **not** implement the items yourself.

Communicate with the user in **German**. All artifacts (code, commits, docs, prompts, the manifest) in **English**.

## When to use

- A handover prompt, a `remaining-work-plan.md` block, or a review report lists several tracked findings (`F-xx`, `RV-xx`, `SOLID-xx`) that are independent enough to parallelize.
- The work needs per-item guidance sessions (own understand/plan/implement/verify phases) but there is no Spec-Kit feature to import.
- Do NOT use when `specs/<id>/` with `spec.md`/`plan.md`/`tasks.md` exists — use `guidance-speckit-parallel-orchestration` there.

## Step 0 — Rules and State

1. Read `AGENTS.md`, `.specify/memory/constitution.md`, and `memory-bank/{activeContext,progress,remaining-work-plan,lessonsLearned}.md`.
2. Read every artifact the items reference (contracts, ADRs, specs, and the code files named in the findings).
3. Resolve the registered guidance workspace name from the guidance configuration (`.guidance/` config or `D:\tools\guidance\.guidance\guidance.json`). Never guess it. It serves two purposes: the name you use if you drive a session yourself, and the **prefix** for the derived temporary worktree workspace names handed to subagents (`<main-workspace>-<item-id-slug>`). Subagents never start a workflow under the main name.
4. Verify the base state: branch, HEAD, clean tree, toolchain (`cargo --version`), and run the baseline gates once so later failures can be attributed.
5. Create the work branch (`feature/<meaningful-name>`) if it does not exist.

## Step 1 — Build the item manifest

Write the manifest **before** dispatching. For every item record:

| Field | Content |
|---|---|
| `id` | stable id from the source (`013-RV-04`, `F-03`) |
| `title` | one line |
| `finding` | the finding quoted verbatim |
| `files` | exact touch points (path + symbol), verified against the current source |
| `outcome` | what must be true afterwards |
| `acceptance` | the tests/evidence that prove it |
| `writeScope` | glob list |
| `exclusive` | files no other item may touch concurrently |
| `estimate` | rough effort |

Do not invent touch points: `grep` for the symbol and read the file. An item whose scope you cannot state precisely is not ready for parallel dispatch.

Keep the manifest in the handover prompt or in a temporary file under `.guidance/state/`; do not add it to the spec tree.

## Step 2 — Derive waves (exclusive-resource serialization)

1. Two items conflict if their write scopes intersect or they claim the same `exclusive` file.
2. Build a conflict graph; assign every item to the lowest wave whose already-assigned items do not conflict with it.
3. Cap each wave at **3 items**; if a wave is wider, split by conflict distance.
4. Publish the wave table together with the conflicts that forced each placement — the user must be able to audit why an item waits.

Typical conflict shapes: one shared module file, one contract document, one pinned-surface test file, one manifest (`Cargo.toml`/`Cargo.lock`).

## Step 3 — Dispatch (one fresh subagent per item)

Create a worktree per parallel item:

```bash
git worktree add ../<repo>-worktrees/<item-id> -b fix/<item-id> <base>
```

Items whose exclusive resource is shared run alone — still as a subagent, but never concurrently with a conflicting item.

Every dispatch message must be self-contained and contain these sections (template: `references/dispatch-prompt-template.md`):

1. `## Execution mode` — verbatim block: the subagent registers and uses a **derived temporary workspace name** for its worktree (never the main workspace name), drives phases with plan-level submissions, reads `get_current_guidance` after each transition, and never imports Spec-Kit artifacts or calls any task tool — explicitly forbidden by name: `import_spec_kit_artifacts`, `release_batch`, `start_task`, `submit_task_implementation`, `prepare_task_review`, `verify_task`, `complete_task`. Reason (verified engine behavior): the spec-kit task machinery is session-scoped and `release_batch` re-points the session's `activeBatchId` on every call while nothing ever clears it — one misplaced call in a plain session creates an active batch record that makes later task-tool sessions of that workspace unreachable and can only be repaired by the `guidance-batch-record-repair` state surgery (operator server stop/start).
2. `## Task` — the item text from the manifest, including the quoted finding.
3. `## Context` — the files to read (rules, contract, ADR, affected sources).
4. `## Worktree` — path, base commit, write scope, and "modify ONLY files inside your write scope".
5. `## Validation` — the exact commands to run inside the worktree.
6. `## Report back` — the required final-message fields (a)–(g), including the temporary workspace name and the deregistration outcome.

Pre-spawn integrity check: the outgoing message must contain both `Execute this fix as a guidance workflow` (or `Execute this task as a guidance workflow`) and `start_workflow { workspace:`. If either is missing, rebuild the message; do not spawn.

### Workspace identity and cleanup (mandatory)

A worktree is a different root, so the main workspace name cannot be reused — and it must not be, because concurrent subagents would otherwise share one state directory and one run record.

- **Derived temporary name:** `<main-workspace>-<item-id-slug>` — lowercase, hyphens only, must match `^[a-z][a-z0-9-]{0,63}$` (main workspace `forge`, item `013-RV-04` → `forge-013-rv-04`). Compute it in the dispatch and put it in the message; do not let the subagent invent one.
- **Registration:** `registry_register { name: "<temp>", root: "<absolute worktree path>", projectName: "<main project name>" }` — once, before `start_workflow`.
- **Deregistration is the subagent's LAST tool action**, after `complete_workflow` succeeded: `registry_register { name: "<temp>", root: "<absolute worktree path>", remove: true }`.
- **Schema trap:** `registry_register` requires `name` and `root` with `minLength: 1` and validates them before the remove branch runs; `root` is ignored while removing, but send the real absolute worktree path anyway. Never omit `root`, never pass the main workspace name, never pass a name you did not register.
- **Orchestrator fallback:** if the report shows a deregistration error (or the subagent died before it), deregister yourself with the same call and note it in the run report. A leftover temporary workspace is a finding, not a detail.

## Step 4 — Integrate and gate (per item)

1. Cross-check the report: workflow session id present, phase reached `completed`, changed files inside the write scope, commands and exit codes listed, commit hash given, temporary workspace deregistered. Missing session id or changed files ⇒ **failed dispatch** (see Failure Handling).
2. Merge `fix/<item-id>` into the work branch with `--no-ff`.
3. Run the gates yourself (see §Gates). Never accept a subagent's claim without your own run of the affected crate tests plus fmt/clippy.
4. Remove the worktree (`git worktree remove --force <path>`) and delete the merged branch (`git branch -d`, only after `git merge-base --is-ancestor` confirms containment).

## Step 5 — Independent review (per wave or per run)

- Spawn a **fresh** reviewer subagent with no builder context and give it only: the item manifest, the changed-file list, the relevant contract/ADR excerpts, and the validation commands. It runs the commands itself.
- Findings come back in the reviewer's final message (outside Spec-Kit there is no review-token API). Record them in the run report.
- Any finding above `low`/`info` that is a real defect must be fixed by another fresh subagent with the finding quoted verbatim, then re-reviewed on the delta.
- Never let a builder review its own work.

## Step 6 — Findings Lifecycle

- For every closed item: set `Status: Handled <date> — <commit>` in `memory-bank/remaining-work-plan.md` and update the mirrored line in `memory-bank/activeContext.md`.
- New findings discovered during the run: add them to **both** files with a trigger point before you close the run.
- Accepted deviations: `Status: Open (accepted …, rationale)` or `tracked` with the trigger that must revisit them.
- Update `progress.md`; add a `lessonsLearned.md` entry only for a recurring trap.

## Gates (after every integration)

```bash
cd rust
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast      # 0 fails; label pre-existing env failures separately
cargo test -p forge-api --test api_snapshot   # when a public surface changed
cd ..
uv run --no-project --with pytest --python 3.12 -- pytest python/tests
```

No npm/node build commands. `LNK1318` / "no space on device" ⇒ remove `rust/target/debug/incremental` and retry once. Wire changes ⇒ update the pinned ACP schema assertions and regenerate the API snapshot (`FORGE_API_SNAPSHOT_UPDATE=1 cargo test -p forge-api --test api_snapshot`, then re-run it plain twice) plus a compatibility note/ADR.

## Failure Handling

- **Failed dispatch** (no session id, no evidence, write scope violated, red gate): re-dispatch once with the same prompt plus a `## Corrections` section quoting what failed. Second failure ⇒ `report_blocker { category, description, requiresUserDecision: true }`.
- **Guidance unusable** (workspace unresolvable, server down): report a blocker; do not fall back to ungated execution.
- **Scope grows during an item** (the fix needs a file outside its write scope): widen the scope in the manifest and re-derive the waves, or split the item — never let two agents edit the same file concurrently.
- **Contract or behaviour change discovered mid-item**: stop the item, record a plan-change/ADR question, and escalate if it changes the deliverable.

## Finish

1. Final report to the user (German): per item → commit → evidence → residual risk; the wave table actually executed; the review outcome; open follow-ups; any leftover temporary workspace and how it was cleaned up.
2. Complete the memory-bank updates (Step 6).
3. Merge request to the operator (`--no-ff` into `develop`). No push, no merge without explicit approval.
4. Remove every worktree and merged `fix/*` branch you created; leave worktrees/branches you did not create untouched and report them.

## Definition of Done

- Every item is fixed with a regression test, or explicitly re-scoped with documentation and a tracked follow-up.
- The work branch is green on the full gate set.
- `remaining-work-plan.md` and `activeContext.md` reflect the true status of every item.
- No worktree, temporary branch or temporary guidance workspace from this run is left behind.
