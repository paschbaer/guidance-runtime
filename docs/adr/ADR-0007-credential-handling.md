# ADR-0007: Platform credential store with least-privilege, indistinguishable access failures

- **Status:** Accepted (2026-11-12)
- **Context:** Private update sources need a bearer token. Storing tokens in
  the repository or config files is unacceptable; embedding them in command
  output would leak them; and answering *not found* differently from
  *unauthorized* would disclose which private repositories exist.
- **Decision:** Credentials live in the platform's protected per-user store
  (Windows Credential Manager, macOS Keychain, Linux Secret Service) under the
  service `guidance:update:<entry>`. A `Secret` newtype with a redacting
  `Debug` carries the value; the token is attached as a bearer header **only**
  for a private target, and asset bytes are fetched from the API asset
  endpoint rather than the public download address — a public target sends no
  `Authorization` header at all. A *not found*, *unauthorized* or *forbidden*
  answer from the source is reported identically as `no access to update
  source`, naming the credential entry to provision (the provisioning hint is
  derived from the same per-platform rule the lookup uses, so docs and lookup
  cannot disagree). Command-shaped backends enumerate the documented
  not-found status and treat every other non-zero exit as an unusable store,
  never as "credential absent".
- **Consequences:** A typo in a public repository name reads as an access
  problem, not a connectivity problem — deliberate, and documented. Missing
  store daemons are distinguishable from missing credentials, which prevents
  misdirecting operators to re-provision something that exists. Least
  privilege is contractual: the token must grant read access to the release
  artifacts of the source repository only.
- **Evidence:** `crates/update/src/credentials.rs`,
  `crates/update/src/github.rs`, the credential/command tests in
  `crates/update/src/tests.rs` (redaction, single-argument entry, public
  target sends no Authorization header, pinned `no access to update source`
  phrase), `docs/IMPLEMENTATION_STATUS.md` (behaviour-change section),
  `specs/001-multi-target-update-sources/research.md` (R2, expired vs absent
  credential).
