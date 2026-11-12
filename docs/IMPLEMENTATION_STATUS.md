# Implementation Status

Implemented: Rust workspace; service/CLI/tray binaries; TOML workload discovery; Windows Named Pipe and Unix development socket; JSON Lines control protocol; process start/stop/restart/status/logs; bounded restart count; tray refresh/start-all/stop-all/exit; build/install scripts and tests.

Production hardening still required: Windows Service Control Dispatcher integration; named-pipe DACL and caller-token role enforcement; Job Objects; graceful termination; durable desired state; health probes and dependency graph; event subscriptions and notifications; signed package update/rollback; Credential Manager; MSI and Authenticode; Windows CI validation.

The source is a functional implementation baseline, not a certified production release.

## Update channels and updater

The update checker supports `stable` and `beta`, uses GitHub Releases, applies SemVer ordering, requires a SHA-256 release asset, and refuses unverified archives. Installation uses a temporary copy of `guidance-updater.exe`, waits for the invoking CLI to exit, stops the service, creates an installation backup, replaces binaries, restarts the service, and restores the backup if replacement or restart fails.
