use std::{fmt::Debug, path::PathBuf};

use crate::discovery::local::LocalDiscoveryError;

mod github;
mod local;

pub trait File: Debug + Send {
    fn path(&self) -> &PathBuf;
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

pub trait Discoverer {
    async fn discover(self) -> Result<Vec<impl File>, DiscoveryError>;
}

pub fn get_discoverer<S: AsRef<str>>(source: S) -> impl Discoverer {
    let source = source.as_ref();

    local::LocalDiscoverer::new(source)
}
