//! HTTP adapters for the update ports (GitHub Releases API and asset downloads).
//!
//! Reading a private source needs two things the public API does not: an
//! `Authorization` header, and the asset endpoint rather than the public
//! download address. Both are handled here so the rest of the pipeline never
//! has to know whether a source is private.
//!
//! Error text deliberately carries no repository path. An unauthorized caller
//! and a non-existent repository must be indistinguishable, and a message that
//! echoed the URL would disclose which repositories exist.

use crate::ports::{Access, AuthenticatedAsset, BoxFuture, Downloader, Releases};
use crate::Release;
use anyhow::Result;
use reqwest::StatusCode;

const USER_AGENT: &str = "guidance-runtime-updater";

/// Why a source read failed.
///
/// Kept as a plain classification so the caller decides which `TargetState`
/// it maps to, rather than parsing an error string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceError {
    /// The source refused the read: not found, unauthorized or forbidden.
    /// Deliberately one variant, because telling them apart discloses whether a
    /// private repository exists.
    Inaccessible,
    /// The source was not reached, or answered in a way that is not a refusal.
    Transport,
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inaccessible => f.write_str("no access to update source"),
            Self::Transport => f.write_str("cannot reach update source"),
        }
    }
}

impl std::error::Error for SourceError {}

/// Classifies an HTTP status without disclosing which case it was.
///
/// 404 is included on purpose: GitHub answers 404, not 403, for a private
/// repository the caller cannot see.
pub fn classify_status(status: StatusCode) -> SourceError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND => {
            SourceError::Inaccessible
        }
        _ => SourceError::Transport,
    }
}

/// Reads releases from the GitHub API, authenticated when a token is supplied.
pub struct GitHubReleases {
    client: reqwest::Client,
}

impl GitHubReleases {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for GitHubReleases {
    fn default() -> Self {
        Self::new()
    }
}

/// A request builder with the shared headers, adding the token only when one
/// is present so a public target sends no `Authorization` at all.
fn authorize(
    request: reqwest::RequestBuilder,
    token: Option<&crate::Secret>,
) -> reqwest::RequestBuilder {
    let request = request
        .header("User-Agent", USER_AGENT)
        .header("X-GitHub-Api-Version", "2022-11-28");
    match token {
        Some(secret) => request.bearer_auth(secret.expose()),
        None => request,
    }
}

impl Releases for GitHubReleases {
    fn list<'a>(&'a self, access: Access<'a>) -> BoxFuture<'a, Result<Vec<Release>>> {
        Box::pin(async move {
            let url = format!(
                "https://api.github.com/repos/{}/releases?per_page=100",
                access.repository
            );
            let response = authorize(self.client.get(url), access.token)
                .send()
                .await
                .map_err(|_| SourceError::Transport)?;

            let status = response.status();
            if !status.is_success() {
                return Err(classify_status(status).into());
            }

            response
                .json::<Vec<Release>>()
                .await
                .map_err(|_| SourceError::Transport.into())
        })
    }
}

/// Downloads release assets over HTTP.
pub struct HttpDownloader {
    client: reqwest::Client,
}

impl HttpDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader for HttpDownloader {
    fn get<'a>(&'a self, asset: &'a AuthenticatedAsset) -> BoxFuture<'a, Result<Vec<u8>>> {
        Box::pin(async move {
            // `Accept: application/octet-stream` is what makes the asset
            // endpoint return bytes rather than JSON metadata.
            let request = authorize(self.client.get(&asset.url), asset.token.as_ref())
                .header("Accept", "application/octet-stream");
            let response = request.send().await.map_err(|_| SourceError::Transport)?;

            let status = response.status();
            if !status.is_success() {
                return Err(classify_status(status).into());
            }

            response
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|_| SourceError::Transport.into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unauthorized caller and a non-existent repository must be
    /// indistinguishable, and GitHub answers 404 rather than 403 for a private
    /// repository the caller cannot see.
    #[test]
    fn refusal_statuses_classify_identically() {
        for status in [
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ] {
            assert_eq!(classify_status(status), SourceError::Inaccessible);
        }
    }

    /// A refusal must not be reported as a transport problem: the operator's
    /// remedy differs between checking permissions and checking connectivity.
    #[test]
    fn other_failures_classify_as_transport() {
        for status in [
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
        ] {
            assert_eq!(classify_status(status), SourceError::Transport);
        }
    }

    /// The rendered text of an access failure is the phrase the specification
    /// pins, and it names no repository.
    #[test]
    fn the_inaccessible_message_is_the_pinned_phrase() {
        assert_eq!(
            SourceError::Inaccessible.to_string(),
            "no access to update source"
        );
        assert_eq!(
            SourceError::Transport.to_string(),
            "cannot reach update source"
        );
    }
}
