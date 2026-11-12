//! Platform credential backends.
//!
//! The configuration names a credential entry; this module decides where that
//! name lives on the current platform. Three backends, each behind `cfg`:
//!
//! - Windows: Credential Manager, read through the `windows-sys` API.
//! - macOS: Keychain, read through the `security` command.
//! - Linux: the freedesktop Secret Service, read through `secret-tool`.
//!
//! The two command-based backends build their invocation in a pure function
//! ([`command_for`]) that a test can inspect. CI has no Keychain and no Secret
//! Service daemon, so without that seam the only thing a test could prove is
//! that the file compiles — which would not catch a wrong flag or a dropped
//! entry argument.

use crate::ports::{BoxFuture, CredentialStore, CredentialStoreError, Secret};

/// The service name used to namespace entries in every backend.
///
/// One constant so an operator's provisioning command and the lookup cannot
/// disagree about the namespace.
pub const SERVICE: &str = "guidance-update";

/// A platform command that reads one credential.
///
/// Pure data: the program and its arguments, never a shell string. The entry
/// name comes from an operator-edited configuration file, so it is passed as a
/// single argument element and never interpolated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialCommand {
    pub program: &'static str,
    pub args: Vec<String>,
}

/// The command that reads `entry`, or `None` on platforms that have no
/// command-based backend.
///
/// Windows has none because it uses the Credential Manager API directly.
pub fn command_for(entry: &str) -> Option<CredentialCommand> {
    #[cfg(target_os = "macos")]
    {
        Some(CredentialCommand {
            program: "security",
            args: vec![
                "find-generic-password".to_string(),
                "-s".to_string(),
                SERVICE.to_string(),
                "-a".to_string(),
                entry.to_string(),
                "-w".to_string(),
            ],
        })
    }
    #[cfg(target_os = "linux")]
    {
        Some(CredentialCommand {
            program: "secret-tool",
            args: vec![
                "lookup".to_string(),
                "service".to_string(),
                SERVICE.to_string(),
                "username".to_string(),
                entry.to_string(),
            ],
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = entry;
        None
    }
}

/// The platform's credential store.
///
/// Constructing it performs no I/O, so a caller can build one for a public
/// target and never touch the store.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlatformCredentialStore;

impl PlatformCredentialStore {
    pub fn new() -> Self {
        Self
    }
}

impl CredentialStore for PlatformCredentialStore {
    fn get<'a>(
        &'a self,
        entry: &'a str,
    ) -> BoxFuture<'a, std::result::Result<Option<Secret>, CredentialStoreError>> {
        Box::pin(async move { read_entry(entry) })
    }
}

#[cfg(windows)]
fn read_entry(entry: &str) -> std::result::Result<Option<Secret>, CredentialStoreError> {
    windows_backend::read(entry)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn read_entry(entry: &str) -> std::result::Result<Option<Secret>, CredentialStoreError> {
    let Some(command) = command_for(entry) else {
        return Err(CredentialStoreError::Unavailable(format!(
            "no credential backend for {}",
            std::env::consts::OS
        )));
    };
    run_command(command)
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn read_entry(_entry: &str) -> std::result::Result<Option<Secret>, CredentialStoreError> {
    Err(CredentialStoreError::Unavailable(format!(
        "no credential backend for {}",
        std::env::consts::OS
    )))
}

/// Exit codes that mean "the entry is not provisioned".
///
/// Everything else is treated as an unusable store, because the remedy
/// differs: one is fixed by provisioning a credential, the other by fixing the
/// environment. Treating every non-zero exit as "absent" would send an
/// operator hunting for a missing credential while the real cause is a locked
/// keychain or a missing D-Bus session.
#[cfg(target_os = "macos")]
const ABSENT_EXIT_CODES: &[i32] = &[44]; // errSecItemNotFound surfaced by `security`

#[cfg(target_os = "linux")]
const ABSENT_EXIT_CODES: &[i32] = &[1]; // `secret-tool` exits 1 for no matching item

/// Runs a credential command and interprets its exit status.
///
/// Only the documented "item not found" status means absent. Any other
/// non-zero status is reported as an unusable store with the status and the
/// child's stderr summary, so an operator sees the actual problem. Stderr is
/// included deliberately: both backends are expected to emit diagnostics and
/// no credential material there, but the message is truncated to keep a
/// misbehaving tool from flooding the output.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn run_command(
    command: CredentialCommand,
) -> std::result::Result<Option<Secret>, CredentialStoreError> {
    use std::process::Command;

    let output = Command::new(command.program)
        .args(&command.args)
        .output()
        .map_err(|e| {
            CredentialStoreError::Unavailable(format!("cannot run {}: {e}", command.program))
        })?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        // Backends terminate the value with a newline, and some emit a trailing
        // NUL. Leading whitespace is left alone because a token could contain
        // it; trimming it silently would produce a 401 that looks like a
        // permission problem.
        let trimmed = text.trim_end_matches(['\r', '\n', '\0']);
        if trimmed.is_empty() {
            return Ok(None);
        }
        return Ok(Some(Secret::new(trimmed)));
    }

    if let Some(code) = output.status.code() {
        if ABSENT_EXIT_CODES.contains(&code) {
            return Ok(None);
        }
    }

    // Anything else — a locked keychain, no D-Bus session, a denied prompt, a
    // signal — is a store we could not use, not a missing credential.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let summary: String = stderr.trim().chars().take(200).collect();
    Err(CredentialStoreError::Unavailable(format!(
        "{} exited with {}: {summary}",
        command.program, output.status
    )))
}

#[cfg(windows)]
mod windows_backend {
    use crate::ports::{CredentialStoreError, Secret};

    /// Reads `guidance:update:<entry>` from the Windows Credential Manager.
    ///
    /// A missing credential yields `Ok(None)`; any API failure yields
    /// `Unavailable`, so an operator can tell "not provisioned" from
    /// "the store is broken".
    pub fn read(entry: &str) -> Result<Option<Secret>, CredentialStoreError> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
        use windows_sys::Win32::Security::Credentials::{
            CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };

        let target = format!("guidance:update:{entry}");
        let wide: Vec<u16> = OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut raw: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: `wide` is a NUL-terminated UTF-16 buffer that outlives the
        // call, and `raw` is a valid out-parameter. `CredFree` is called on
        // every success path below.
        let ok = unsafe { CredReadW(wide.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) };

        if ok == 0 {
            let err = std::io::Error::last_os_error();
            return if err.raw_os_error() == Some(ERROR_NOT_FOUND as i32) {
                Ok(None)
            } else {
                // The OS error text, not the numeric code: an operator needs to
                // know whether to provision a credential or to fix the account
                // the service runs under.
                Err(CredentialStoreError::Unavailable(format!(
                    "cannot read credential '{entry}' from Credential Manager: {err}"
                )))
            };
        }

        // SAFETY: `ok` was non-zero, so `raw` points to a CREDENTIALW that the
        // API owns until CredFree.
        let value = unsafe {
            let cred = &*raw;
            let bytes =
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            let text = String::from_utf16_lossy(&units);
            CredFree(raw as *const _);
            text
        };

        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Secret::new(value)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The command must carry the entry as one element, never joined into a
    /// shell string: the entry comes from an operator-edited file, so joining
    /// would turn a typo into command injection.
    #[test]
    fn the_command_carries_the_entry_as_a_single_argument() {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let cmd = command_for("guidance with space").expect("a backend exists here");
            assert!(
                cmd.args.iter().any(|a| a == "guidance with space"),
                "the entry must be one argument element: {:?}",
                cmd.args
            );
            assert!(
                !cmd.args
                    .iter()
                    .any(|a| a.contains(' ') && a.contains("guidance")),
                "no argument may join the entry with other text: {:?}",
                cmd.args
            );
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            assert!(command_for("guidance").is_none());
        }
    }

    /// The full argument vector is pinned, not merely its members: a swapped
    /// `-s`/`-a` or a reordered `service`/`username` pair would pass an
    /// "contains" assertion while provisioning and lookup disagree.
    #[test]
    fn the_command_shape_is_pinned_exactly() {
        #[cfg(target_os = "macos")]
        {
            let cmd = command_for("my-entry").unwrap();
            assert_eq!(cmd.program, "security");
            assert_eq!(
                cmd.args,
                vec![
                    "find-generic-password".to_string(),
                    "-s".to_string(),
                    SERVICE.to_string(),
                    "-a".to_string(),
                    "my-entry".to_string(),
                    "-w".to_string(),
                ]
            );
        }
        #[cfg(target_os = "linux")]
        {
            let cmd = command_for("my-entry").unwrap();
            assert_eq!(cmd.program, "secret-tool");
            assert_eq!(
                cmd.args,
                vec![
                    "lookup".to_string(),
                    "service".to_string(),
                    SERVICE.to_string(),
                    "username".to_string(),
                    "my-entry".to_string(),
                ]
            );
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            assert!(command_for("my-entry").is_none());
        }
    }

    /// Only the documented "not found" status means the entry is absent. Every
    /// other failure is an unusable store, because the remedy differs — a
    /// locked keychain must not be reported as a missing credential.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn the_absent_exit_code_is_the_documented_one() {
        #[cfg(target_os = "macos")]
        assert_eq!(ABSENT_EXIT_CODES, &[44]);
        #[cfg(target_os = "linux")]
        assert_eq!(ABSENT_EXIT_CODES, &[1]);
    }

    /// The backend names the documented program and namespaces the entry under
    /// the shared service name, so the provisioning command an operator runs
    /// matches what the lookup asks for.
    #[test]
    fn the_command_uses_the_documented_program_and_service() {
        #[cfg(target_os = "macos")]
        {
            let cmd = command_for("guidance").unwrap();
            assert_eq!(cmd.program, "security");
            assert!(cmd.args.iter().any(|a| a == SERVICE));
            assert!(cmd.args.iter().any(|a| a == "find-generic-password"));
        }
        #[cfg(target_os = "linux")]
        {
            let cmd = command_for("guidance").unwrap();
            assert_eq!(cmd.program, "secret-tool");
            assert!(cmd.args.iter().any(|a| a == SERVICE));
            assert!(cmd.args.iter().any(|a| a == "lookup"));
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            assert!(command_for("guidance").is_none());
        }
    }

    /// On a platform without a backend, the store must report the named
    /// condition instead of silently returning "no credential".
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    #[test]
    fn an_unsupported_platform_reports_unavailable() {
        let err = read_entry("guidance").unwrap_err();
        assert!(matches!(err, CredentialStoreError::Unavailable(_)), "{err}");
        assert!(err.to_string().contains("no credential backend"), "{err}");
    }
}
