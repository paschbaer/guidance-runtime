# System Patterns

> Derived from the source tree. Items marked **(assumption)** are inferred and not confirmed.

## Architecture Overview

Guidance Runtime Service (GRS) is a **runtime host and process orchestrator** for local MCP
servers, comparable in spirit to Docker Desktop, Ollama, systemd, or a local Kubernetes
(see `docs/SPECIFICATION-v1.1.md` §1). Rather than being a single MCP server, it supervises a set
of configured *workloads*, exposes a small request/response protocol to local clients, and can
update itself from GitHub Releases.

## Component Map

- **`guidance-protocol`**
  - Responsibility: IPC request/response types (`Request`, `Response`) shared by all clients
  - Inputs and outputs: serialized JSON over the local transport
  - Dependencies: `serde`, `serde_json`
- **`guidance-ipc`**
  - Responsibility: client side of the protocol plus the method dispatch table that maps a
    method name to a runtime action
  - Methods: `runtime.status`, `workload.list`, `workload.start`, `workload.stop`,
    `workload.restart`, `workload.stop_all`, `workload.start_all`, `workload.logs`,
    `diagnostics.summary`; unknown methods fail with `method not found`
  - Dependencies: `guidance-protocol`, `guidance-runtime`, `anyhow`
- **`guidance-runtime`** (library, 277 LOC)
  - Responsibility: workload supervision core — list/start/stop/restart, start/stop enabled
    workloads, collect logs
  - Dependencies: `tokio`, `tracing`
- **`guidance-service`** (24 LOC binary)
  - Responsibility: host the runtime as the Windows service `GuidanceRuntime`
- **`guidance-cli`** (binary `guidance`)
  - Responsibility: operator-facing commands (`status`, `list`, `start`, `stop`, `restart`,
    `logs`, plus `update channel|check|install`)
  - Notable: `update install` copies `guidance-updater.exe` to a temp path and spawns it with
    `--wait-pid` so the running CLI can exit before the binaries are replaced
- **`guidance-tray`** (binary, Windows only)
  - Responsibility: tray menu (status, channel, check for updates, start all enabled, stop all,
    refresh, exit)
  - Notable: uses `winit` 0.30 `ApplicationHandler` + `run_app`
- **`guidance-update`** (library)
  - Responsibility: release lookup, asset selection (`windows-x64.zip`), SHA-256 verification,
    archive extraction, backup/replace/restart of the installation
- **`guidance-updater`** (binary)
  - Responsibility: apply an update after the invoking process exits (`wait_for_exit`)

## Architectural Principles

- Keep responsibilities explicit and components loosely coupled: protocol types, IPC dispatch,
  supervision logic, and each UI surface live in separate crates.
- Keep core logic (`guidance-runtime`) independent from user-interface integrations
  (`guidance-service`, `guidance-tray`, `guidance-cli`).
- Platform-specific behaviour is expressed with `cfg(windows)` / `cfg(not(windows))`, keeping a
  non-Windows stub that fails with a clear error instead of not compiling.
- Fail closed on safety-relevant paths (missing checksum → refuse update; unsafe archive path →
  abort extraction).

## Design Patterns

### Pattern: Method dispatch table

- **Use when:** adding a new IPC command
- **Implementation:** add an arm to the `match` on `q.method` in `crates/ipc/src/lib.rs` and a
  corresponding CLI/tray call site; do not restructure the surrounding dispatch
- **Avoid when:** the new behaviour is not a request/response interaction with the runtime

### Pattern: Transactional self-update

- **Use when:** replacing binaries of a running installation
- **Implementation:** stop service → copy current install to `<install>.backup` → extract staged
  archive → replace files → start service; on any failure restore the backup and restart
- **Avoid when:** the target does not require the service to be stopped

## Data Flow

1. A client (`guidance` CLI or tray) builds a `Request` with a method name and JSON params.
2. `guidance-ipc` dispatches the method against the runtime instance.
3. `guidance-runtime` performs the operation (Supervisor) and returns JSON.
4. The client renders the result (`summary_and_errors` style) or surfaces the error.

Update flow: `update check` → GitHub API release lookup → asset selection → download ZIP +
SHA-256 → verify → hand off to `guidance-updater` → install with backup/restore.

## Integration Boundaries

- **GitHub Releases**
  - Protocol or interface: HTTPS REST (`reqwest`, rustls)
  - Authentication: none for public repositories (assumption)
  - Failure behavior: the update is aborted, never partially applied
- **Windows SCM**
  - Protocol or interface: `sc.exe` invoked as a child process
  - Failure behavior: the update is rolled back and the service restarted

## Error Handling

- Fail early for missing mandatory prerequisites.
- Include the failed operation and an actionable correction in error messages.
- Do not hide partial failures; the updater restores the backup and reports the original error.
- Avoid destructive recovery steps unless explicitly requested.

## Security Patterns

- Never commit secrets; `config/runtime.toml` and `config/updates.toml` are local and untracked.
- Validate external input before use: archive paths go through `enclosed_name()` to prevent path
  traversal; the SHA-256 asset is mandatory.
- Keep dependencies centralized in `[workspace.dependencies]` so versions are constrained once.

## Testing Strategy

- Unit tests: present but sparse — the workspace currently has 2 tests in total
- Integration tests: none
- End-to-end tests: none; build, clippy, and tests run in CI on Windows and Ubuntu
- Manual checks: service install/uninstall via `scripts/install-dev.ps1` /
  `scripts/uninstall-dev.ps1`, tray interaction, `guidance update check`

## Architectural Decisions

Record durable decisions as ADRs and link them here.

- (unresolved) No ADR directory exists yet; `docs/SPECIFICATION-v1.1.md` is the current
  authoritative description of intent.

## Known Technical Risks

- **Sparse test coverage** — a bugfix can outlive the process it modifies (updater), with no
  automated protection. Mitigation: add focused tests around `guidance-update` first.
- **Thin module surfaces** — `crates/ipc/src/lib.rs` implements dispatch in a single dense
  expression, which is hard to review and violates SRP. See `remaining-work-plan.md` (RW-001).
- **Spec/implementation drift** — `docs/SPECIFICATION-v1.1.md` (779 lines) describes considerably
  more than the ~1.2k LOC implementation. Treat the spec as intent, the code as fact.
