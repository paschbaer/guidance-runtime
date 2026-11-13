# Project Brief

> Sources: `README.md`, `docs/SPECIFICATION-v1.1.md`, `docs/IMPLEMENTATION_STATUS.md`, the Cargo
> workspace. Items marked **(assumption)** are inferred and need maintainer confirmation.

## Project

- **Name:** Guidance Runtime Service (GRS), repository `guidance-runtime`
- **Repository:** `guidance-runtime` (Cargo workspace, version 1.1.0)
- **Owner:** User
- **Status:** Functional implementation baseline; not a certified production release
  (`docs/IMPLEMENTATION_STATUS.md`)
- **Last reviewed:** 2026-11-13

## Purpose

GRS is a runtime host and process orchestrator for local MCP servers on a developer workstation.
Instead of running each MCP server process by hand, it supervises a configured set of *workloads*,
exposes a small request/response protocol to local clients (CLI, tray), and can update itself from
GitHub Releases.

## Target Users

- **Primary users:** User
- **Secondary users:** automation and agent tooling that drives GRS through its IPC protocol.
- **Maintainers and operators:** whoever installs the service, configures `config/runtime.toml`,
  and publishes releases.

## Goals

1. Supervise MCP workload processes: start, stop, restart, list, report status, and return logs.
2. Offer a predictable control surface for both humans (tray, CLI) and programs (JSON IPC).
3. Deliver verified, reversible updates through versioned GitHub Releases.

## Non-Goals

- Operating as an MCP server itself — GRS hosts and orchestrates, it does not implement tools.
- Being a certified production release in its current state: service control dispatcher
  integration, pipe security, job objects, and signed packages are explicitly still open
  (`docs/IMPLEMENTATION_STATUS.md`).
- Cross-platform delivery today — Linux and macOS support is planned for later releases only.

## Scope

### In Scope

- Rust workspace with service, CLI, tray, updater, and shared protocol/IPC/supervision crates
- WiX v7 MSI installer for Windows (`installer/GuidanceRuntime.wxs`)
- TOML workload discovery and bounded restart policy
- JSON Lines control protocol over Windows Named Pipes / Unix development socket
- Update check, SHA-256 verification, backup/restore installation

### Out of Scope

- Health probes, dependency graphs, event subscriptions, Credential Manager, Authenticode signing
  (see "Production hardening still required")

## Success Criteria

- CI is green on all three matrix OSes (`windows-latest`, `ubuntu-latest`, `macos-latest`).
- A configured workload can be started, stopped, restarted, and inspected via CLI and tray.
- An update is applied only from a release with a matching SHA-256 asset, and rolls back on
  failure.

## Constraints

- **Technical:** Windows-only for service, tray, and installation paths; `cfg`-gated so the
  workspace still builds on other platforms.
- **Organizational:** `feature/*` → `develop` (beta) → PR → `main` (stable); `[skip release]`
  suppresses a release for a commit.
- **Security and compliance:** no secrets in the repository; fail-closed checksum verification;
  archive paths constrained by `enclosed_name()`.
- **Time and budget:** not stated in the repository.

## Key Stakeholders

- Repository owner: User
- Operators installing and running the Windows service

## References

- Product documentation: `README.md`, `README.de.md`
- Architecture decisions: `docs/SPECIFICATION-v1.1.md` (intent), `memory-bank/systemPatterns.md`
- Implementation status and open hardening work: `docs/IMPLEMENTATION_STATUS.md`
