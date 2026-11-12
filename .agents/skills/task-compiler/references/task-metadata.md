# Spec Kit parallel task metadata contract

The compiler accepts two metadata schemes. The **native inline scheme** is the
default and matches how `specs/<id>/tasks.md` is written. The **fenced scheme**
is an alternate with a richer field set.

## Native inline scheme (primary)

Add one `taskmeta:` line immediately below each task line:

```markdown
- [ ] T004 [P] [US1] Create 18 stub crates + xtask stub

  - taskmeta: files=[crates/*/Cargo.toml, crates/*/src/lib.rs, xtask/src/main.rs] deps=[T001] exclusive=[root Cargo.toml] est=45m
```

Fields (space-separated `key=value` tokens):

| Field | Required | Meaning |
|---|---:|---|
| `files` | yes | Write scope — files/globs the task may modify. Use `[]` when none. |
| `deps` | yes | Task IDs that must complete first. Use `[]` when empty. Supports ranges: `T024–T029` expands to `T024..T029`. |
| `exclusive` | no | Logical resources that cannot be changed concurrently (e.g. `root Cargo.toml`, `migrations/`). A comma-free bracket body is one resource name (e.g. `exclusive=[xtask rules]`). |
| `est` | yes | Estimated minutes, `NNm` (e.g. `est=30m`). |
| `wave` | no | Execution wave, bare integer (`wave=1`). The guidance engine reads it from `tasks.md` (`- taskmeta:` line or a `## Wave N` heading) and uses it as the cut boundary for automatic batch releases (ADR 0029). Not read from `task-graph.json`. |

List values inside `[...]` are comma-separated; a bracketed body with no comma
is treated as a single item.

The compiler normalizes these to the canonical keys below before writing the
graph: `files` → `write_scope`, `deps` → `depends_on`,
`exclusive` → `exclusive_resources`, `est` → `estimated_minutes`. Missing
optional fields default to: `read_scope=[]`, `validation=[]`,
`agent_profile=rust-builder`, `max_attempts=2`, `non_goals=[]`.

## Fenced scheme (alternate)

Add a fenced `taskmeta` block immediately below each task line:

```markdown
- [ ] T004 [P] [US1] Implement GoalFirstStrategy in `crates/forge-clearthought/src/methods/goal_first.rs`

  ```taskmeta
  depends_on: [T001, T003]
  write_scope:
    - crates/forge-clearthought/src/methods/goal_first.rs
  read_scope:
    - crates/forge-decision/**
    - specs/004-clearthought/contracts/**
  exclusive_resources:
    - forge-clearthought-method-registry
  validation:
    - cargo test -p forge-clearthought goal_first
    - cargo clippy -p forge-clearthought --all-targets -- -D warnings
  agent_profile: rust-builder
  estimated_minutes: 30
  max_attempts: 2
  non_goals:
    - Do not change public decision contracts.
    - Do not add MCP integration.
  ```
```

The parser supports a deliberately small YAML subset: inline arrays and
indented string lists. Avoid nested mappings.
