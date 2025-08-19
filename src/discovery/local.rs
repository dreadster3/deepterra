use std::path::{Path, PathBuf};

use futures::future::BoxFuture;
use log::{debug, info};
use tokio::task::JoinSet;

use crate::discovery::{Discoverer, DiscoveryError, File};

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
}

#[derive(Debug)]
struct LocalFile {
    path: PathBuf,
}

impl LocalFile {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl File for LocalFile {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn get_contents(&self) -> std::result::Result<String, std::io::Error> {
        std::fs::read_to_string(&self.path)
    }
}

impl LocalDiscoverer {
    pub fn new(source: impl AsRef<Path>) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
        }
    }

    fn discover_impl(
        self,
    ) -> BoxFuture<'static, Result<Vec<impl File + Send>, LocalDiscoveryError>> {
        Box::pin(async move {
            let path = self.source.clone();
            info!("Discovering local files in {path:?}");

            if !path.exists() {
                return Err(LocalDiscoveryError::PathError(path));
            }

            if path.is_file() {
                return Ok(vec![LocalFile::new(path)]);
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
                    directory_tasks.spawn(async move {
                        let discoverer = LocalDiscoverer::new(path);
                        discoverer.discover_impl().await
                    });

                    continue;
                }

                if !file_name.ends_with(".tf") {
                    debug!("Skipping non-terraform file: {file_name}");
                    continue;
                }

                files.push(LocalFile::new(path));
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
    async fn discover(self) -> Result<Vec<impl File>, DiscoveryError> {
        let result = self.discover_impl().await?;
        Ok(result)
    }
}
