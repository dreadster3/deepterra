use std::path::PathBuf;

use crate::discovery::local::LocalDiscoveryError;

pub mod local;

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Local discovery error: {0}")]
    LocalDiscoveryError(#[from] LocalDiscoveryError),
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
    async fn discover(self) -> Result<Vec<PathBuf>, DiscoveryError>;
}
