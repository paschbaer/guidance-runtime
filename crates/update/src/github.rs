//! HTTP adapters for the update ports (GitHub Releases API and asset downloads).

use crate::ports::{BoxFuture, Downloader, Releases};
use crate::Release;
use anyhow::Result;

const USER_AGENT: &str = "guidance-runtime-updater";

/// Reads the release list from the public GitHub Releases API.
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

impl Releases for GitHubReleases {
    fn list<'a>(&'a self, repository: &'a str) -> BoxFuture<'a, Result<Vec<Release>>> {
        Box::pin(async move {
            let url = format!("https://api.github.com/repos/{repository}/releases?per_page=100");
            Ok(self
                .client
                .get(url)
                .header("User-Agent", USER_AGENT)
                .send()
                .await?
                .error_for_status()?
                .json::<Vec<Release>>()
                .await?)
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
    fn get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<Vec<u8>>> {
        Box::pin(async move {
            Ok(self
                .client
                .get(url)
                .header("User-Agent", USER_AGENT)
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?
                .to_vec())
        })
    }
}
