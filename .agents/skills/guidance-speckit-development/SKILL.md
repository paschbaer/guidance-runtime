---
name: guidance-speckit-development
description: Runs the full Spec-Kit SDD flow on the artifact-bound spec-kit-development workflow variant with fail-closed artifact gates per phase. Use when the user says "spec-kit-development", "SDD flow", "run speckit specify/plan/converge", or wants a feature developed from scratch through the speckit command sequence.
---

# Guidance Workflow — Artifact-Bound spec-kit-development Variant (Mode 3)

## When to use
The feature should go through the complete SDD cycle (specify → clarify →
plan → checklist → tasks → analyze → implement → converge) with the workflow
enforcing artifact existence at each phase exit.

## Workspace resolution

- `start_workflow` takes the workspace's REGISTERED NAME (kebab-case, e.g. `dcfamily`) — never a display name, a host path, or free text from the request.
- Discover the registered name BEFORE calling: check prior run records, the repository's `.guidance/` configuration, or ask the operator. Do not guess.
- If the workspace is not yet registered: register it with
  `registry_register { name: "<kebab-case>", root: "<absolute repository path>" }`
  — Windows paths are supported (e.g. `D:\repos\cabp`); there is no
  Docker/pool path indirection. On failure, ask the operator; do not guess.

## Procedure
1. Start WITH the variant id:
   `start_workflow { workspace: "<registered-name>", request: "<feature>",
   workflowId: "spec-kit-development" }`
   The definition resolves from `.guidance/workflows/spec-kit-development.json`;
   a missing file fails closed with `workflow_not_found` (check the file
   exists in THIS workspace).
2. Per phase, run the bound commands; the phase exit gate blocks until the
   artifact exists AND imports cleanly:
   - understand: `/speckit-specify` + `/speckit-clarify` → gate: spec.md
   - plan: `/speckit-plan` → gate: plan.md
   - checklist: `/speckit-checklist` → gate: checklists/**
   - tasks: `/speckit-tasks` → gate: tasks.md
   - review_and_adjust_plan: `/speckit-analyze` → review submission
   - implement: `/speckit-implement` → per-batch review
   - verify: converge → see below
3. Attended clarify: open questions from `/speckit-clarify` MUST go through
   `report_blocker { requiresUserDecision: true }`; the session pauses until
   `resume_workflow` carries the answers. Never answer for the user.
4. Strict batch cadence in implement: submissions carry the current batch;
   review decides `implementation_changes_required` (loop) /
   `batch_approved_more_pending` (next batch) / `submission_valid`.
   Parallel execution in implement (when tasks.md carries parallelization
   metadata): compile the batch with the `task-compiler` skill, then execute
   each wave via the `agent-scheduler` skill — the current agent is the
   orchestrator and spawns AT MOST 3 parallel subagents, each running one
   task as a guidance workflow in its own worktree. The orchestrator session
   keeps making ALL guidance submissions (batch evidence, review,
   `verify_task`, `complete_task`); the workflow gates run unchanged.
5. Converge loop in verify: a byte-identical tasks.md after
   `speckit.converge` completes; a new `## Phase N: Convergence` section
   loops back to implement — call `refresh_spec_kit_artifacts` FIRST.
6. Resume-friendly: bound phases whose exit artifact already exists are
   skipped (`artifacts_present`) — a crashed session restarts where it left off.
