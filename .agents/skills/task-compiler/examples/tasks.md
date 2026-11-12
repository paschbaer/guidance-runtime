# Tasks: Native ClearThought

- [ ] T001 Create decision contracts in `crates/forge-decision/src/candidate.rs`

  ```taskmeta
  depends_on: []
  write_scope:
    - crates/forge-decision/src/candidate.rs
  validation:
    - cargo test -p forge-decision
  agent_profile: rust-builder
  estimated_minutes: 30
  max_attempts: 2
  non_goals:
    - Do not implement reflection strategies.
  ```

- [ ] T002 [P] [US1] Implement GoalFirstStrategy in `crates/forge-clearthought/src/methods/goal_first.rs`

  ```taskmeta
  depends_on: [T001]
  write_scope:
    - crates/forge-clearthought/src/methods/goal_first.rs
  read_scope:
    - crates/forge-decision/**
  exclusive_resources: []
  validation:
    - cargo test -p forge-clearthought goal_first
  agent_profile: rust-builder
  estimated_minutes: 30
  max_attempts: 2
  non_goals:
    - Do not change decision contracts.
    - Do not add MCP integration.
  ```
