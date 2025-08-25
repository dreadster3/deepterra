use std::{fmt::Debug, path::PathBuf};

use async_trait::async_trait;

use crate::discovery::local::LocalDiscoveryError;

mod github;
mod local;

#[async_trait]
pub trait File: Debug + Send {
    fn path(&self) -> PathBuf;
    fn boxed(self) -> Box<dyn File>;
    async fn get_contents(&self) -> anyhow::Result<String>;
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Local discovery error: {0}")]
    LocalDiscoveryError(#[from] LocalDiscoveryError),

    #[error("Github discovery error: {0}")]
    GithubDiscoveryError(#[from] github::GithubDiscoveryError),
}

#[derive(Debug)]
pub struct DiscoveryOptions {
    ignore: Option<String>,
}

impl DiscoveryOptions {
    pub fn new(ignore: Option<String>) -> Self {
        Self { ignore }
    }
}

#[async_trait]
pub trait Discoverer {
    async fn discover(&self) -> Result<Vec<Box<dyn File>>, DiscoveryError>;
}

pub fn get_discoverer<S: AsRef<str>>(source: S) -> Result<Box<dyn Discoverer>, DiscoveryError> {
    let source = source.as_ref();

    if let Some(source) = source.strip_prefix("github:") {
        let discoverer = github::GithubDiscoverer::new(source)?;
        return Ok(Box::new(discoverer));
    }

    let discoverer = local::LocalDiscoverer::new(source);
    Ok(Box::new(discoverer))
}
