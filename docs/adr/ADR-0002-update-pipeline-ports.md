# ADR-0002: Ports for the update pipeline

- **Status:** Accepted (2026-11-12; extended 2026-11-13 with `ServiceControl`)
- **Context:** `guidance-update` has to talk to GitHub Releases, download
  archives, install files, read platform credential stores and control Windows
  services. Hard-coding those interactions made the pipeline untestable: the
  destructive install transaction originally had no seam for `sc.exe` or the
  filesystem (RW-005), and HTTP behaviour could only be exercised live.
- **Decision:** Every external interaction sits behind a small port owned by
  `guidance-update`: `Releases`, `Downloader`, `Installer` (`ports.rs`),
  `CredentialStore` (three cfg-gated platform backends), and — since the
  service-control port landed — `ServiceControl` with the real `sc.exe`
  adapter (`ScControl`, the sole `Command::new("sc.exe")` site, owning the 2 s
  settle delay). `lib.rs` is the composition root that wires the real
  adapters; production call sites construct them there so public signatures do
  not leak the ports. Tests drive the same traits with fakes (recording,
  failing-on-demand, in-memory release lists).
- **Consequences:** The pipeline is testable end to end without network,
  credentials or a real service; adapters can be replaced per platform without
  touching selection logic. The cost is trait boilerplate and a composition
  root that must stay the only place real adapters are constructed.
- **Evidence:** `crates/update/src/ports.rs`, `crates/update/src/install.rs`
  (`ServiceControl`, `ScControl`), `memory-bank/remaining-work-plan.md`
  (RW-004/RW-005, both Handled), `memory-bank/systemPatterns.md` (design
  pattern "Ports for the update pipeline").
