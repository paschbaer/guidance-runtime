# Scheduler configuration

```json
{
  "max_parallel": 3,
  "worktree_root": "../forge-agent-worktrees",
  "branch_prefix": "agent/"
}
```

## Execution model

Execution is orchestrator-based: the current agent acts as orchestrator and
dispatches tasks to at most **3 parallel subagents** (`spawn_agent`), each of
which runs its task as a guidance workflow (see the `guidance-plain-workflow`
skill). `max_parallel` is a hard cap enforced by the orchestrator — keep it at
3 or lower.

The configuration is used for **planning only** (waves, worktree layout,
branch prefixes). There are no agent adapter commands: task execution goes
through subagents, not subprocesses.

Worktrees and branches are created by the orchestrator before any subagent is
spawned. Subagents receive ready-to-use worktree paths and never create
worktrees themselves.
