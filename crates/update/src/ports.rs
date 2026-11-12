//! Narrow ports for the update pipeline. Every piece of I/O the update logic
//! needs is expressed here so the logic can be exercised with test doubles.
//!
//! Async methods use boxed futures instead of `async fn` in traits: the
//! workspace deliberately avoids the `async-trait` dependency.

use crate::{Release, UpdateConfig};
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Lists published releases of a repository.
pub trait Releases: Send + Sync {
    fn list<'a>(&'a self, repository: &'a str) -> BoxFuture<'a, Result<Vec<Release>>>;
}

/// Fetches raw bytes from a URL.
pub trait Downloader: Send + Sync {
    fn get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<Vec<u8>>>;
}

/// Replaces the installed binaries with the ones contained in an archive.
pub trait Installer {
    fn install(&self, config: &UpdateConfig, bytes: &[u8]) -> Result<()>;
}
