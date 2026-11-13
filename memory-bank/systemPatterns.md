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
  - Responsibility: IPC request/response types (`Request`, `Response`) and the typed `Method`
    enum shared by all clients
  - Inputs and outputs: serialized JSON over the local transport
  - Dependencies: `serde`, `serde_json`
- **`guidance-ipc`**
  - Responsibility: client side of the protocol plus the dispatch that maps a method onto a
    runtime action
  - Modules: `lib.rs` (transport: Unix socket / named pipe, line reader), `dispatch.rs` (one
    named handler per method, exhaustive `match` on `Method`), `error.rs` (`DispatchError` with the
    exhaustive wire-code mapping), `params.rs` (`workload_id`, `tail` parsing)
  - Methods: `runtime.status`, `workload.list`, `workload.start`, `workload.stop`,
    `workload.restart`, `workload.stop_all`, `workload.start_all`, `workload.logs`,
    `diagnostics.summary`; unknown methods fail with `method not found: <name>` (`not_found`)
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
  - Responsibility: release lookup, architecture-aware asset selection, SHA-256 verification,
    archive extraction, backup/replace/restart of the installation
  - Structure: `lib.rs` (public API + composition root), `arch.rs` (architecture-to-suffix
    mapping), `ports.rs` (`Releases`, `Downloader`, `Installer` with boxed futures), `github.rs`
    (HTTP adapters), `install.rs` (ZIP/service transaction), `selection.rs` (pure
    channel/version/checksum rules plus architecture-safe asset matching), `command.rs` (shared
    CLI/updater command bodies), `params.rs` (`sha256_hex`), `tests.rs` (port-driven tests)
  - Dependencies: `reqwest`, `sha2`, `zip`, `semver`, `toml`
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

### Pattern: Typed method enum + exhaustive dispatch

- **Use when:** adding or changing an IPC method
- **Implementation:** add a variant to `guidance_protocol::Method` (wire string in `as_str` and
  `FromStr`, pinned by `wire_names_are_pinned`), then a handler in `crates/ipc/src/dispatch.rs`.
  The compiler enforces both matches; no client may use a raw method string.
- **Avoid when:** the behaviour is not a request/response interaction with the runtime

### Pattern: Ports for the update pipeline

- **Use when:** changing how releases are fetched, downloaded or installed
- **Implementation:** implement `Releases`, `Downloader` or `Installer` in a new adapter and wire
  it in `lib.rs` (the composition root). Tests use the same traits with fakes.
- **Avoid when:** the change is pure selection logic — put that in `selection.rs` instead

### Pattern: Architecture-aware asset selection

- **Use when:** the release process publishes per-architecture artifacts
- **Implementation:** `arch::asset_suffix_for_arch` maps `std::env::consts::ARCH` onto the asset
  suffix; `selection::select` receives the expected suffix separately from the configured pattern,
  so a pattern can narrow the search but never widen it beyond the running architecture. New
  architectures need an entry in `KNOWN_ARCH_SUFFIXES` and the release workflow matrix.
- **Avoid when:** only one architecture is ever published — but keep the mapping, since the
  hardcoded default was the bug that made arm64 undeliverable

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

- Unit tests: 164 across the workspace on Windows (159 on Linux and macOS, where the
  `#[cfg(windows)]` install tests do not compile) — IPC error mapping and parameter parsing (13),
  protocol method names (3), update architecture mapping/selection/checksum/verification/install
  guard (147), runtime defaults (1)
- Integration tests: none
- End-to-end tests: none; build, clippy, and tests run in CI on Windows, Ubuntu and macOS
- Manual checks: service install/uninstall via `scripts/install-dev.ps1` /
  `scripts/uninstall-dev.ps1`, tray interaction, `guidance update check`

## Architectural Decisions

Recorded as ADRs in `docs/adr/` (index: `docs/adr/README.md`):

- ADR-0001 typed `Method` enum with exhaustive IPC dispatch
- ADR-0002 ports for the update pipeline (Releases, Downloader, Installer,
  CredentialStore, ServiceControl)
- ADR-0003 `install.json` marker as the authoritative installed-version source
- ADR-0004 transactional Windows install with full rollback
- ADR-0005 seven-state outcome model with a 0/1/2 exit taxonomy
- ADR-0006 normalized multi-target configuration, legacy flat form preserved
- ADR-0007 platform credential store, least-privilege, indistinguishable access failures
- ADR-0008 archive-driven install content
- ADR-0009 platform-architecture asset suffix table

## Known Technical Risks

- **Untested destructive transaction** — the Windows backup/restore path in
  `crates/update/src/install.rs` still has no test seam (`sc.exe` calls). Tracked as RW-005.
- **IPC handlers lack a test seam** — `Runtime` requires a real config and spawns processes, so
  handler behaviour is only covered indirectly. Tracked as RW-006.
- **Spec/implementation drift** — `docs/SPECIFICATION-v1.1.md` (779 lines) describes considerably
  more than the ~2k LOC implementation. Treat the spec as intent, the code as fact.
