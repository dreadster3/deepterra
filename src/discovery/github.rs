use std::{path::PathBuf, sync::Arc};

use crate::discovery::{Discoverer, DiscoveryError, File};

#[derive(Debug, thiserror::Error)]
pub enum GithubDiscoveryError {
    #[error("Octocrab error: {0}")]
    OctocrabError(#[from] octocrab::Error),

    #[error("Not implemented")]
    NotImplemented,
}

pub struct GithubDiscoverer {
    owner: String,
    repo: Option<String>,
    octocrab: Arc<octocrab::Octocrab>,
}

#[derive(Debug)]
pub struct GithubFile {
    octocrab: Arc<octocrab::Octocrab>,
    repository_id: u64,
    path: PathBuf,
}

impl File for GithubFile {
    fn path(&self) -> &PathBuf {
        &self.path
    }

    async fn get_contents(&self) -> anyhow::Result<String> {
        let mut contents = self
            .octocrab
            .repos_by_id(self.repository_id)
            .get_content()
            .path(self.path.to_string_lossy())
            .send()
            .await?;

        let items = contents.take_items();
        let content = items.first().and_then(|item| item.decoded_content());

        match content {
            Some(content) => Ok(content),
            None => Err(anyhow::anyhow!("No content found")),
        }
    }
}

impl GithubDiscoverer {
    pub fn new<S: AsRef<str>>(owner: S, octocrab: octocrab::Octocrab) -> Self {
        Self {
            octocrab: Arc::new(octocrab),
            owner: owner.as_ref().to_string(),
            repo: None,
        }
    }

    async fn discover_impl(self) -> Result<Vec<GithubFile>, GithubDiscoveryError> {
        let owner = self.owner;
        let repositories_page = match self.octocrab.users(&owner).repos().send().await {
            Ok(response) => response,
            Err(error) => self.octocrab.orgs(&owner).list_repos().send().await?,
        };

        let repositories = self.octocrab.all_pages(repositories_page).await?;

        Err(GithubDiscoveryError::NotImplemented)
    }
}

impl Discoverer for GithubDiscoverer {
    async fn discover(self) -> Result<Vec<impl File>, DiscoveryError> {
        let result = self.discover_impl().await?;
        Ok(result)
    }
}
