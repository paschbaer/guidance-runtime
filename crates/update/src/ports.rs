//! Narrow ports for the update pipeline. Every piece of I/O the update logic
//! needs is expressed here so the logic can be exercised with test doubles.
//!
//! Async methods use boxed futures instead of `async fn` in traits: the
//! workspace deliberately avoids the `async-trait` dependency.

use crate::{Release, UpdateTarget};
use anyhow::Result;
use semver::Version;
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Which target a source read is for, and the credential to read it with.
///
/// Identity and secret travel together on purpose. A signature that took a
/// repository string and a token as separate parameters would allow a caller to
/// pair a token with the wrong repository, and the adapter could no longer tell
/// which target it serves — the ambiguity that lets one product's credential be
/// spent on another product's source.
pub struct Access<'a> {
    /// The configured target name, for diagnostics.
    pub target: &'a str,
    /// `owner/repository` holding the releases.
    pub repository: &'a str,
    /// The credential, when the source is private. Never rendered: see
    /// [`Secret`].
    pub token: Option<&'a Secret>,
}

/// A release asset together with the credential needed to fetch it.
///
/// The archive and its checksum both travel as this type, so they cannot
/// diverge into one authenticated and one public request.
#[derive(Clone)]
pub struct AuthenticatedAsset {
    /// The address the adapter fetches. For a private source this is the API
    /// asset endpoint, never the public download address.
    pub url: String,
    pub token: Option<Secret>,
}

impl std::fmt::Debug for AuthenticatedAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthenticatedAsset")
            .field("url", &self.url)
            .field("token", &self.token)
            .finish()
    }
}

/// Lists published releases of a repository.
pub trait Releases: Send + Sync {
    fn list<'a>(&'a self, access: Access<'a>) -> BoxFuture<'a, Result<Vec<Release>>>;
}

/// Fetches raw bytes from a URL.
pub trait Downloader: Send + Sync {
    fn get<'a>(&'a self, asset: &'a AuthenticatedAsset) -> BoxFuture<'a, Result<Vec<u8>>>;
}

/// Replaces the installed binaries with the ones contained in an archive.
///
/// The resolved target is passed instead of the whole configuration: an
/// installer that has to re-derive which target it is installing is exactly
/// the ambiguity that lets one target's files land in another's directory. The
/// version is passed too because the installer is the component that records a
/// successful installation in the marker; a caller-supplied version after the
/// fact would allow the marker and the files to disagree.
pub trait Installer {
    fn install(&self, target: &UpdateTarget, version: &Version, bytes: &[u8]) -> Result<()>;
}

/// An access token whose value cannot be printed by accident.
///
/// `Debug` and `Display` are implemented by hand rather than derived: every
/// error path in this crate formats its context, so a derived `Debug` would
/// eventually put the token into a log line. The value is reachable only
/// through [`Secret::expose`], which is a readable call site in a review.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw token. Every call site of this method is a place a secret can
    /// escape, so keep them few and obvious.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Why a credential could not be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialStoreError {
    /// The platform's secure store could not be consulted at all: no daemon,
    /// no session, a locked keychain. Distinct from "the entry is absent",
    /// because the remedy is different.
    Unavailable(String),
}

impl std::fmt::Display for CredentialStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(f, "credential store unavailable: {reason}"),
        }
    }
}

impl std::error::Error for CredentialStoreError {}

/// Reads a named credential from the platform's protected per-user store.
///
/// The configuration names the entry; the store decides where that name
/// lives. Returning `Ok(None)` means the entry is absent, which is a
/// different outcome from a store that cannot be consulted.
pub trait CredentialStore: Send + Sync {
    fn get<'a>(
        &'a self,
        entry: &'a str,
    ) -> BoxFuture<'a, std::result::Result<Option<Secret>, CredentialStoreError>>;
}
