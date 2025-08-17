use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::future::BoxFuture;
use glob::Pattern;
use log::{debug, info};
use tokio::task::JoinSet;

use crate::discovery::{Discoverer, DiscoveryError, DiscoveryOptions};

#[derive(Debug, thiserror::Error)]
pub enum LocalDiscoveryError {
    #[error("Invalid path provided: {0}")]
    PathError(PathBuf),

    #[error("Error reading directory: {0}")]
    ReadDirError(#[from] std::io::Error),

    #[error("Error parsing glob: {0}")]
    GlobError(#[from] glob::PatternError),
}

pub struct LocalDiscoverer {
    source: PathBuf,
    options: Arc<DiscoveryOptions>,
}

impl LocalDiscoverer {
    pub fn new(source: impl AsRef<Path>, options: Arc<DiscoveryOptions>) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            options,
        }
    }

    fn discover_impl(self) -> BoxFuture<'static, Result<Vec<PathBuf>, LocalDiscoveryError>> {
        Box::pin(async move {
            let path = self.source.clone();
            info!("Discovering local files in {path:?}");

            if !path.exists() {
                return Err(LocalDiscoveryError::PathError(path));
            }

            let glob_pattern = self
                .options
                .ignore
                .as_ref()
                .map(|ignore| Pattern::new(ignore))
                .transpose()?;

            if let Some(pattern) = glob_pattern.as_ref()
                && pattern.matches_path(path.as_path())
            {
                debug!("Skipping file: {path:?}");
                return Ok(Vec::new());
            }

            if path.is_file() {
                return Ok(vec![path]);
            }

            let mut files = Vec::new();
            let mut directory_tasks = JoinSet::new();

            let entries = std::fs::read_dir(path)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                if file_name.starts_with(".") {
                    debug!("Skipping hidden file or directory: {file_name}");
                    continue; // Skip hidden files and directories
                };

                if path.is_dir() {
                    let options = self.options.clone();
                    directory_tasks.spawn(async move {
                        let discoverer = LocalDiscoverer::new(path, options);
                        discoverer.discover_impl().await
                    });

                    continue;
                }

                if !file_name.ends_with(".tf") {
                    debug!("Skipping non-terraform file: {file_name}");
                    continue;
                }

                if let Some(pattern) = glob_pattern.as_ref()
                    && pattern.matches_path(path.as_path())
                {
                    debug!("Skipping file: {path:?}");
                    continue;
                }

                files.push(path);
            }

            let directories = directory_tasks.join_all().await;
            for directory in directories {
                let directory = directory?;
                files.extend(directory);
            }

            Ok(files)
        })
    }
}

impl Discoverer for LocalDiscoverer {
    async fn discover(self) -> Result<Vec<PathBuf>, DiscoveryError> {
        let result = self.discover_impl().await?;
        Ok(result)
    }
}
