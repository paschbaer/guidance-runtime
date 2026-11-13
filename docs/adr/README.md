# Architecture Decision Records

Durable records of the architecture decisions in this repository, in
[Nygard](https://www.nygards.org/2011/architecture-decision-records-are-usable-and-useful/)
format (`Status` / `Context` / `Decision` / `Consequences`, plus an `Evidence`
line citing the repo artifacts the decision is grounded in).

Conventions:

- ADRs are **immutable records**. To change a decision, write a new ADR that
  supersedes the old one and link both; never edit an accepted ADR's decision
  in place.
- Numbering is sequential (`ADR-0001`, `ADR-0002`, …) and never reused.
- All ADRs are written in English; dates are the date the *decision* was made,
  not the date the record was written.

## Index

| ADR | Decision | Status |
|---|---|---|
| [ADR-0001](ADR-0001-typed-method-enum.md) | Typed `Method` enum with exhaustive IPC dispatch | Accepted (2026-11-12) |
| [ADR-0002](ADR-0002-update-pipeline-ports.md) | Ports for the update pipeline (Releases, Downloader, Installer, CredentialStore, ServiceControl) | Accepted (2026-11-12/13) |
| [ADR-0003](ADR-0003-version-marker.md) | `install.json` marker as the authoritative installed-version source | Accepted (2026-11-12) |
| [ADR-0004](ADR-0004-transactional-install.md) | Transactional Windows install with full rollback | Accepted (2026-11-12/13) |
| [ADR-0005](ADR-0005-outcome-model-exit-taxonomy.md) | Seven-state outcome model with a 0/1/2 exit taxonomy | Accepted (2026-11-12/13) |
| [ADR-0006](ADR-0006-multi-target-config.md) | Normalized multi-target configuration, legacy flat form preserved | Accepted (2026-11-12) |
| [ADR-0007](ADR-0007-credential-handling.md) | Platform credential store, least-privilege, indistinguishable access failures | Accepted (2026-11-12) |
| [ADR-0008](ADR-0008-archive-driven-content.md) | Archive-driven install content (`bin/` scoping, copy-and-overwrite) | Accepted (2026-11-12) |
| [ADR-0009](ADR-0009-asset-suffix-table.md) | Platform-architecture asset suffix table | Accepted (2026-11-12) |

See also `memory-bank/systemPatterns.md` (patterns and component map) and
`docs/IMPLEMENTATION_STATUS.md` (delivered behaviour and known limitations).
