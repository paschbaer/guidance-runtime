# Security

Use a dedicated service identity. Protect ProgramData manifests from user writes. Restrict the pipe to local authenticated users and authorize mutation methods against local Reader, Operator and Administrator groups. Add Windows token impersonation and DACL creation before production rollout. Never place secrets in TOML; integrate Windows Credential Manager or DPAPI. Sign binaries and installer with Authenticode.
