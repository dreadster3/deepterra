use std::{fmt::Debug, path::PathBuf};

use crate::discovery::local::LocalDiscoveryError;

pub mod local;

pub trait File: Debug {
    fn path(&self) -> &PathBuf;
    fn get_contents(&self) -> Result<String, std::io::Error>;
}

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
    async fn discover(self) -> Result<Vec<impl File>, DiscoveryError>;
}
