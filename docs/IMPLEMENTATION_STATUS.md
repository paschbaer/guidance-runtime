# Implementation Status

Implemented: Rust workspace; service/CLI/tray binaries; TOML workload discovery; Windows Named Pipe and Unix development socket; JSON Lines control protocol; process start/stop/restart/status/logs; bounded restart count; tray refresh/start-all/stop-all/exit; build/install scripts and tests.

Production hardening still required: Windows Service Control Dispatcher integration; named-pipe DACL and caller-token role enforcement; Job Objects; graceful termination; durable desired state; health probes and dependency graph; event subscriptions and notifications; signed package update/rollback; Credential Manager; MSI and Authenticode; Windows CI validation.

The source is a functional implementation baseline, not a certified production release.

## Update channels and updater

The update checker supports `stable` and `beta`, uses GitHub Releases, applies SemVer ordering, requires a SHA-256 release asset, and refuses unverified archives. Installation uses a temporary copy of `guidance-updater.exe`, waits for the invoking CLI to exit, stops the service, creates an installation backup, replaces binaries, restarts the service, and restores the backup if replacement or restart fails.

### Multi-target sources (in progress)

A configuration may list several targets, each with its own source, channel and
installation directory, and a check run reports one result per target without
one target's failure suppressing the others. Only the runtime target reports an
installed version; every other target currently reports `not installed`.

Delivered so far: the target list and its validation, the platform-aware
archive suffix table (Windows, macOS, Linux), the fan-out over targets, the
seven outcome states, and the legacy flat configuration keeping its historical
output contract. The flat form stays flat when a channel is changed.

Not yet delivered, and therefore not usable:

- Reading the artifact source of a **private** target. Such a target currently
  reports `no access to update source` unconditionally. This is deliberate and
  is replaced by the authenticated access path.
- Distinguishing an authorization problem from a transport problem for a public
  target: every source-list failure currently reports as a network error.
- Structured (`json`) output. The `--format` flag documented in the feature
  contract is not present yet; only the human-readable form exists.
- Per-target installation: installing a target still reaches the runtime's
  service name unless the target overrides `service_name`, and a service-less
  archive cannot be installed.
- A macOS CI leg. The macOS suffix mapping exists and is unit-tested on every
  host, but no CI leg runs on macOS, and the release matrix publishes Windows
  archives only.
- Operator documentation of the multi-target form beyond
  `config/updates.example.toml`.
