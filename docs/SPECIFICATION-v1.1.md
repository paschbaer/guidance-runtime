# Guidance Runtime Service (GRS)
## Specification v1.1

### Status
Draft Architecture Specification

### Language
English

---

# 1. Executive Summary

The Guidance Runtime Service (GRS) is a local platform service responsible for discovering, deploying, starting, monitoring, updating, and managing MCP servers, agent runtimes, and auxiliary developer tools.

Rather than operating a single MCP server process, GRS acts as a runtime host and process orchestrator similar in spirit to Docker Desktop, Ollama, systemd, or Kubernetes on a local workstation.

The initial implementation targets Windows but must be designed to support Linux and macOS in later releases.

---

# 2. Goals

## Primary Goals

- Single entry point for all local MCP services.
- Automatic process lifecycle management.
- Health monitoring and restart policies.
- Plugin-based extensibility.
- Centralized configuration.
- Unified CLI experience.
- Optional Windows tray application for interactive status and lifecycle control.
- Authenticated local Named Pipe control API shared by the CLI and tray application.
- Future Forge runtime integration.

## Non-Goals

- Agent implementation logic.
- Knowledge storage engine.
- LLM inference engine.
- Workflow execution engine.

---

# 3. High-Level Architecture

```text
+-------------------+        +-------------------+
| guidance.exe      |        | guidance-tray.exe |
| CLI Client        |        | User Session UI   |
+-------------------+        +-------------------+
          |                            |
          +-------- Named Pipe --------+
                       |
                       v
+----------------------------------------+
| Guidance Runtime Service               |
+----------------------------------------+
| Named Pipe Control API                 |
| Service Host                           |
| Process Manager                        |
| Plugin Registry                        |
| Update Manager                         |
| Health Monitor                         |
+----------------------------------------+
                       |
          +------------+------------+
          |            |            |
          v            v            v
    Guidance MCP  Insight MCP  ClearThought MCP
```

---

# 4. Architectural Components

## 4.1 Service Host

Responsibilities:

- Windows Service implementation.
- Startup orchestration.
- Shutdown coordination.
- Service lifecycle management.

Suggested Technology:

- Rust
- windows-service crate

### Operations

- Start
- Stop
- Pause (future)
- Restart
- Query status

The Windows Service runs in Session 0 and must not create desktop UI elements. All interactive functionality is hosted by the separate `guidance-tray.exe` process in the signed-in user's session.

---

## 4.2 Process Manager

Responsibilities:

- Spawn child processes.
- Capture logs.
- Track process state.
- Apply restart policies.

Process States:

```text
Stopped
Starting
Running
Degraded
Restarting
Failed
```

Restart Policy:

```text
max_retries = 5
retry_interval = 10s
```

---

## 4.3 Plugin Registry

Responsible for discovering installed plugins.

Plugin Location:

```text
plugins/
```

Each plugin contains:

```text
plugin.toml
binary executable
assets/
```

Example:

```toml
name = "Insight"
version = "1.0.0"
type = "mcp"
command = "insight-mcp.exe"
```

---

## 4.4 Health Monitor

Responsibilities:

- Heartbeat monitoring.
- Process health tracking.
- Crash detection.
- Restart triggering.

Health Indicators:

- Process alive
- Port reachable
- MCP handshake available
- Memory limits
- CPU limits

---

## 4.5 Update Manager

Responsibilities:

- Version checks.
- Download packages.
- Install updates.
- Rollback.

Future Support:

- GitHub Releases
- Internal package repositories
- Azure Artifacts

---


## 4.6 Tray Application

`guidance-tray.exe` is an optional, unprivileged Windows user-session application. It provides convenient access to the runtime without changing the service's ability to operate without an interactive login.

### Responsibilities

- Display the aggregate runtime state in the Windows notification area.
- Display the state of each configured workload.
- Start, stop, and restart authorized workloads through the control API.
- Open or tail workload logs.
- Show actionable notifications for crashes, crash loops, failed health checks, and failed updates.
- Copy a redacted diagnostic summary to the clipboard.
- Launch the CLI or a future management UI for advanced operations.

### Tray States

| Visual state | Meaning |
|---|---|
| Green | Runtime reachable and all enabled workloads are healthy. |
| Yellow | Runtime reachable, but at least one workload is starting, stopped unexpectedly, or degraded. |
| Red | Runtime reachable, but at least one enabled workload is failed or in a crash loop. |
| Gray | Runtime service is stopped, unavailable, or the control API cannot be reached. |
| Blue animation | A user-requested start, stop, restart, install, or update operation is in progress. |

The actual icon artwork must also use shape or overlay differences so that state is not communicated by color alone.

### Context Menu

The tray menu shall contain:

```text
Guidance Runtime: Running
--------------------------------
Guidance MCP       Running    >
Insight MCP        Running    >
ClearThought MCP   Stopped    >
--------------------------------
Start All Enabled
Stop All
Refresh
Open Logs
Copy Diagnostics
Start with Windows        [x]
Show Notifications        [x]
--------------------------------
Exit Tray
```

Each workload submenu may expose `Start`, `Stop`, `Restart`, `Open Logs`, and `Copy Endpoint`. Commands that the current user is not authorized to execute shall be disabled rather than silently failing.

`Exit Tray` terminates only `guidance-tray.exe`. It must not stop the runtime service or managed workloads.

### Startup and Single-Instance Behavior

- The tray application starts at interactive user login, not at machine boot.
- The preferred startup mechanism is a per-user Startup App registration created by the installer or enabled from the tray settings.
- Only one tray instance may run per Windows user session.
- Multiple signed-in users may each run a tray process, subject to control API authorization.
- The tray reconnects automatically when the service restarts.
- Reconnection uses bounded exponential backoff from 1 second up to 30 seconds.
- The tray must remain responsive while the service is unavailable.

### Notifications

Notifications should be emitted only for state transitions that require attention:

- Workload entered `Failed`.
- Restart budget was exhausted.
- Required workload became unavailable.
- Update failed or was rolled back.
- Runtime service became unavailable for more than 30 seconds.

Repeated notifications for the same unchanged condition must be suppressed. Informational process restarts should be grouped to avoid notification spam.

### Security Boundary

The tray application:

- Runs with the permissions of the signed-in user.
- Must not request elevation during normal status and lifecycle operations.
- Never opens service-owned configuration or state files directly.
- Never spawns or terminates managed workload processes directly.
- Sends all commands to the service through the Named Pipe control API.
- Never displays secret values returned from workload configuration.

---

## 4.7 Named Pipe Control API

The local control API is the single control-plane interface used by both `guidance.exe` and `guidance-tray.exe`.

### Endpoint

```text
\\.\pipe\guidance-runtime-v1
```

The pipe name is versioned to allow future protocol-major upgrades. The service owns the pipe server. CLI and tray processes act as clients.

### Transport Requirements

- Windows Named Pipes in byte mode.
- UTF-8 JSON Lines framing for v1.
- One JSON object per line.
- Maximum request or response frame size: 1 MiB.
- Maximum concurrent clients: 32 by default.
- Idle client timeout: 5 minutes, except active event or log subscriptions.
- Request identifiers must be unique within a client connection.
- Long-running operations return an operation ID immediately.
- Status, events, and logs must not contain secret values.

### Authentication and Authorization

The service obtains the caller's Windows identity from the named-pipe connection and authorizes each method.

Recommended roles:

| Role | Permissions |
|---|---|
| Reader | Runtime status, workload status, endpoints, and redacted logs. |
| Operator | Reader permissions plus start, stop, and restart. |
| Administrator | Operator permissions plus plugin installation, removal, updates, and runtime configuration changes. |

Recommended local groups:

```text
Guidance Runtime Readers
Guidance Runtime Operators
Guidance Runtime Administrators
```

The pipe Discretionary Access Control List must deny anonymous and network logons. Administrative methods must not trust a role claimed in the JSON payload; authorization is derived from the authenticated Windows token.

### Protocol Envelope

Request:

```json
{
  "protocol": "1.0",
  "id": "req-42",
  "method": "workload.restart",
  "params": {
    "workload_id": "insight"
  }
}
```

Successful response:

```json
{
  "protocol": "1.0",
  "id": "req-42",
  "result": {
    "accepted": true,
    "operation_id": "op-91"
  }
}
```

Error response:

```json
{
  "protocol": "1.0",
  "id": "req-42",
  "error": {
    "code": "permission_denied",
    "message": "Operator permission is required."
  }
}
```

### Required Methods

| Method | Minimum role | Purpose |
|---|---|---|
| `runtime.status` | Reader | Return service health, version, and aggregate state. |
| `workload.list` | Reader | Return all workloads and their current states. |
| `workload.get` | Reader | Return details for one workload. |
| `workload.start` | Operator | Start an enabled workload. |
| `workload.stop` | Operator | Stop a workload. |
| `workload.restart` | Operator | Perform a controlled restart. |
| `workload.start_all` | Operator | Start all enabled workloads in dependency order. |
| `workload.stop_all` | Operator | Stop all workloads in reverse dependency order. |
| `workload.logs` | Reader | Read a bounded, redacted log tail. |
| `events.subscribe` | Reader | Subscribe to runtime and workload state changes. |
| `diagnostics.summary` | Reader | Return a redacted diagnostic summary. |
| `plugin.install` | Administrator | Install and validate a plugin package. |
| `plugin.remove` | Administrator | Remove an inactive plugin. |
| `plugin.update` | Administrator | Update a plugin and report operation progress. |

### Event Subscription

The tray should use `events.subscribe` rather than aggressive polling. The service emits events such as:

```json
{
  "protocol": "1.0",
  "event": "workload.state_changed",
  "sequence": 481,
  "data": {
    "workload_id": "insight",
    "previous_state": "Restarting",
    "state": "Running",
    "timestamp": "2026-11-12T07:00:00Z"
  }
}
```

After reconnecting, the tray requests a complete snapshot with `runtime.status` and `workload.list` before resubscribing. Sequence numbers are connection-scoped in v1 and are used to detect dropped events.

### Error Codes

Minimum error codes:

```text
invalid_request
unsupported_protocol
method_not_found
permission_denied
runtime_unavailable
workload_not_found
invalid_state
operation_in_progress
validation_failed
timeout
internal_error
```

### Control Flow

```text
User
  |
  v
Guidance Tray or CLI
  |
  | JSON request over authenticated Named Pipe
  v
Named Pipe Control API
  |
  +--> Authorization
  |
  +--> Runtime command handler
  |
  +--> Process Manager / Registry / Update Manager
  |
  v
JSON response and asynchronous events
```

---

# 5. Deployment Layout

```text
C:\Program Files\Guidance\
│
├─ guidance.exe
├─ guidance-service.exe
├─ guidance-tray.exe
├─ config\
├─ plugins\
├─ logs\
├─ data\
├─ cache\
└─ assets\tray\
```

---

# 6. CLI Specification

## Install

```bash
guidance install
```

Creates and registers the runtime service.

## Uninstall

```bash
guidance uninstall
```

## Start Runtime

```bash
guidance up
```

## Stop Runtime

```bash
guidance down
```

## Status

```bash
guidance status
```

Example:

```text
Guidance MCP       Running
Insight MCP        Running
ClearThought MCP   Stopped
```

## Logs

```bash
guidance logs
```

```bash
guidance logs insight
```

## Restart

```bash
guidance restart insight
```

## Plugin Discovery

```bash
guidance plugins list
```

## Plugin Installation

```bash
guidance plugins install insight
```

## Plugin Removal

```bash
guidance plugins remove insight
```

---

# 7. Configuration System

Main service configuration:

```toml
runtime_name = "Guidance"
log_level = "info"

[mcp.guidance]
enabled = true
command = "guidance-mcp.exe"

[mcp.insight]
enabled = true
command = "insight-mcp.exe"

[control_api]
pipe_name = "guidance-runtime-v1"
max_clients = 32
max_frame_size_bytes = 1048576
idle_timeout_seconds = 300
reader_group = "Guidance Runtime Readers"
operator_group = "Guidance Runtime Operators"
administrator_group = "Guidance Runtime Administrators"
```

Per-user tray configuration:

```toml
start_with_windows = true
show_notifications = true
notify_service_unavailable_after_seconds = 30
reconnect_initial_delay_seconds = 1
reconnect_max_delay_seconds = 30
```

---

# 8. Plugin Model

## Plugin Types

### MCP Plugin

Hosts external MCP servers.

### Tool Plugin

Hosts utility tools.

### Agent Plugin

Hosts full autonomous agents.

### Runtime Plugin

Provides infrastructure services.

---

# 9. Security Model

Principles:

- Least privilege.
- Explicit plugin registration.
- Signed release packages.
- Secure update channels.
- Authenticated Named Pipe clients using Windows identities.
- Per-method role checks for Reader, Operator, and Administrator operations.
- The service remains the only owner of managed workload processes.
- No secrets in status responses, events, diagnostics, or tray notifications.

Future Features:

- Plugin signature validation.
- Policy enforcement.
- RBAC.

---

# 10. Logging

Format:

```json
{
  "timestamp":"2026-11-11T12:00:00Z",
  "service":"insight",
  "level":"info",
  "message":"started"
}
```

Storage:

```text
logs/
```

Rotation:

- Daily rotation
- Compression
- Retention policies

---

# 11. Observability

Metrics:

- Active processes
- Restart counts
- Uptime
- CPU utilization
- Memory utilization

Future:

- OpenTelemetry
- Prometheus exporter

---

# 12. Future Forge Integration

Long-term architecture:

```text
Forge Runtime
│
├─ Runtime Kernel
├─ Process Manager
├─ Update Manager
├─ Registry
└─ Scheduling Engine

Guidance
├─ Guidance MCP
├─ Insight MCP
└─ ClearThought MCP
```

GRS should be architected so that its internals can later be migrated into Forge with minimal changes.

---

# 13. Technology Stack

| Area | Technology |
|--------|-----------|
| Language | Rust |
| CLI | clap |
| Service Runtime | windows-service |
| Async Runtime | tokio |
| Configuration | serde + toml |
| Logging | tracing |
| Updates | self_update |
| Packaging | WiX Toolset |
| Local IPC | Windows Named Pipes + JSON Lines |
| Tray UI | Rust Windows desktop/tray library selected during implementation |

---

# 14. MVP Scope

Included:

- Runtime service
- CLI
- Plugin registry
- Process manager
- Health monitoring
- Logging
- Configuration support
- Authenticated Named Pipe control API
- Optional Windows tray application
- Tray status, workload lifecycle commands, log access, diagnostics, and notifications

Excluded:

- Full management UI or settings dashboard
- Web dashboard
- Marketplace
- Remote cluster management
- Distributed execution

---

# 15. Success Criteria

The MVP is considered successful if it can:

1. Run as a Windows service.
2. Discover plugins automatically.
3. Start and stop MCP servers.
4. Restart failed processes.
5. Expose runtime status.
6. Provide centralized logs.
7. Install future MCP servers without recompilation.
8. Run the service without an interactive user session.
9. Show aggregate and per-workload state through the optional tray application.
10. Execute authorized tray and CLI commands through the same Named Pipe control API.
11. Reject unauthorized control requests based on the caller's Windows identity.
12. Reconnect the tray automatically after service restart without interrupting the service or workloads.
13. Exit the tray without stopping the runtime service.

---

# 16. Tray and Control API Acceptance Tests

The tray and control API extension is accepted when:

1. `guidance-tray.exe` displays the correct gray state while the service is unavailable.
2. The tray reconnects after a service restart and refreshes its full state snapshot.
3. A Reader can inspect status and logs but cannot start, stop, restart, install, or update.
4. An Operator can start, stop, and restart workloads but cannot install plugins.
5. An Administrator can perform all defined control operations.
6. An unauthorized local process cannot connect to or invoke methods on the Named Pipe.
7. `Exit Tray` closes only the tray process.
8. The service and managed workloads continue running after user logoff.
9. Workload state changes appear in the tray through event subscriptions without polling faster than once per 30 seconds as a fallback.
10. Crash-loop notifications are deduplicated and contain no secret values.
11. A lost event sequence triggers a complete status refresh.
12. Oversized or malformed JSON frames are rejected without terminating the service.
13. Multiple signed-in users can run independent tray clients while service-side authorization remains correct.

---

End of Specification
