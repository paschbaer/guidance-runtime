---
name: task-compiler
description: Compile a GitHub Spec Kit tasks.md file containing parallelization metadata into a validated task DAG, execution waves, conflict report, and machine-readable task manifests. Use when planning parallel coding-agent work, validating task dependencies or write scopes, or converting Spec Kit tasks into scheduler input.
---

# Task Compiler

## Purpose

Turn `tasks.md` into deterministic scheduler input. This skill does not implement code and does not start agents.

## Trigger conditions

Use this skill when the user asks to:

- compile or validate Spec Kit tasks;
- derive a task dependency graph or parallel execution waves;
- detect conflicting file ownership or write scopes;
- prepare work packages for multiple coding agents;
- check whether `[P]` task markings are actually safe.

## Required input

A path to a Spec Kit `tasks.md` file. The compiler accepts two metadata
schemes, both documented in `references/task-metadata.md`:

- **Native inline scheme** (default, used by `specs/<id>/tasks.md`): a single
  `- taskmeta:` line per task with `files=[]`, `deps=[]`, `exclusive=[]` and
  `est=NNm`. This is the primary scheme.
- **Wave hint (write-back).** When the compiled task graph yields execution
  waves, write the wave into the artifact as `wave=N` in the SAME `- taskmeta:`
  line (e.g. `- taskmeta: files=[src/a.rs] deps=[T001] wave=1 est=30m`) or
  group tasks under a `## Wave N` heading. The guidance engine reads the wave
  from `tasks.md` only (ADR 0029) — `task-graph.json` and `execution-waves.md`
  are not imported, so a wave that is not written back cannot shape batch
  partitioning.
- **Fenced `taskmeta` block** (alternate): a fenced ````taskmeta` code block with
  `depends_on`, `write_scope`, `validation`, `agent_profile`, etc.

## Workflow

1. Read `tasks.md` and the feature's `plan.md` and `spec.md` when available.
2. Run:

   ```bash
   python scripts/compile_tasks.py path/to/tasks.md --out-dir path/to/generated
   ```

3. Inspect `validation-report.md`. Do not hand invalid output to a scheduler.
4. If validation succeeds, use:
   - `task-graph.json` for machine processing;
   - `execution-waves.md` for human review;
   - `manifests/*.json` as self-contained work packages.
5. Report concrete errors with task IDs and the smallest correction.

## Validation rules

The compiler must fail when:

- a task has no `taskmeta` (neither inline nor fenced);
- a dependency references an unknown task;
- the dependency graph contains a cycle;
- two tasks in the same execution wave have overlapping write scopes;
- a task is marked parallel but has no declared write scope;
- an exclusive resource is claimed concurrently;
- a task depends on itself.

The compiler warns when:

- estimated duration exceeds 60 minutes;
- maximum attempts exceeds 2;
- a write scope is repository-wide;
- `[P]` disagrees with dependency or scope analysis.

## Output contract

`task-graph.json` contains:

- feature identifier;
- task records;
- dependency edges;
- calculated execution wave;
- write-scope conflicts;
- exclusive-resource conflicts;
- validation status.

Do not infer that two tasks are safe merely because Spec Kit marks both `[P]`. Parallel safety is derived from dependencies, write scopes, and exclusive resources.
