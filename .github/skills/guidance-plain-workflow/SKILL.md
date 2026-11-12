---
name: guidance-plain-workflow
description: Starts a Guidance MCP workflow in the default plain plan-driven mode (standard-development) and drives it via plan-level submissions. Use when the user asks to "start a workflow" for a normal change with NO Spec-Kit feature folder involved. Do not use when specs/<id>/tasks.md artifacts exist or the user mentions spec-kit.
---

# Guidance Workflow — Plain Plan-Driven Mode (Mode 1)

## When to use
The user wants a guided development workflow for a task, and there is no
Spec-Kit feature (no `specs/<id>/` with spec.md/plan.md/tasks.md) to import.

## Workspace resolution

- `start_workflow` takes the workspace's REGISTERED NAME (kebab-case, e.g. `dcfamily`) — never a display name, a host path, or free text from the request.
- Discover the registered name BEFORE calling: check prior run records, the repository's `.guidance/` configuration, or ask the operator. Do not guess.
- If the workspace is not yet registered: register it with
  `registry_register { name: "<kebab-case>", root: "<absolute repository path>" }`
  — Windows paths are supported (e.g. `D:\repos\cabp`); there is no
  Docker/pool path indirection. On failure, ask the operator; do not guess.

## Procedure
1. Start the session WITHOUT `workflowId` (this selects the default
   `standard-development` workflow):
   `start_workflow { workspace: "<registered-name>", request: "<task>" }`
   The workspace must be the registered NAME, never a host path.
2. Drive the phases with plan-level submissions only:
   - `understand` → `submit_understanding` (summary, assumptions, acceptance criteria)
   - `plan` → `submit_plan` (stable task IDs, dependencies, affected files)
   - `implement` → `submit_implementation` (implementedTasks, changedFiles)
   - review loops via `submit_implementation_review` on findings
   - `verify` → `submit_verification`
   - `complete` → `complete_workflow` (runs completion operations/gates)
3. After EVERY transition, read guidance with `get_current_guidance`.

## Hard rules
- Do NOT call `import_spec_kit_artifacts` or any task tool — that switches
  behavior to mode 2 semantics and is not this mode.
- Never set a `workflowId` — plain mode is the default.
- Respect phase gates (lint/test/build `beforeExit` operations); a red gate
  is a finding, not a bypass.
