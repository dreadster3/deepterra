use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use log::{error, info, warn};
use octocrab::models::repos::Content;
use tokio::task::JoinSet;

use crate::discovery::{Discoverer, DiscoveryError, File};

#[derive(Debug, thiserror::Error)]
pub enum GithubDiscoveryError {
    #[error("Octocrab error: {0}")]
    OctocrabError(#[from] octocrab::Error),

    #[error("Failed to parse ID: {0}")]
    ParseIdError(#[from] std::num::ParseIntError),
}

pub struct GithubDiscoverer {
    owner: String,
    repo: Option<String>,
    octocrab: Arc<octocrab::Octocrab>,
}

#[derive(Debug)]
pub struct GithubFile {
    content: Content,
}

impl GithubFile {
    fn new(content: Content) -> Self {
        Self { content }
    }
}

#[async_trait]
impl File for GithubFile {
    fn path(&self) -> PathBuf {
        self.content.path.clone().into()
    }

    async fn get_contents(&self) -> anyhow::Result<String> {
        let content = self.content.decoded_content();

        match content {
            Some(content) => Ok(content),
            None => Err(anyhow::anyhow!("No content found")),
        }
    }

    fn boxed(self) -> Box<dyn File> {
        Box::new(self)
    }
}

impl GithubDiscoverer {
    fn try_app_auth() -> Option<octocrab::auth::Auth> {
        let app_id = std::env::var("GITHUB_APP_ID").ok()?;
        let app_key = std::env::var("GITHUB_APP_KEY").ok()?;
        let app_id = app_id.parse::<u64>().ok()?;

        info!("Using GITHUB_APP_ID({app_id}) and GITHUB_APP_KEY");

        Some(octocrab::auth::Auth::App(octocrab::auth::AppAuth {
            key: jsonwebtoken::EncodingKey::from_secret(app_key.as_bytes()),
            app_id: octocrab::models::AppId::from(app_id),
        }))
    }

    fn try_cli_auth() -> Option<octocrab::auth::Auth> {
        let output = std::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .ok()?;

        if output.status.success() {
            let token = String::from_utf8(output.stdout).ok()?;
            return Some(octocrab::auth::Auth::PersonalToken(token.trim().into()));
        }

        None
    }

    fn try_personal_token_auth() -> Option<octocrab::auth::Auth> {
        let token = std::env::var("GITHUB_TOKEN").ok()?;
        Some(octocrab::auth::Auth::PersonalToken(token.trim().into()))
    }

    fn authentication() -> Option<octocrab::auth::Auth> {
        Self::try_personal_token_auth()
            .or_else(Self::try_app_auth)
            .or_else(Self::try_cli_auth)
    }

    fn octocrab() -> Result<octocrab::Octocrab, GithubDiscoveryError> {
        let mut builder = octocrab::OctocrabBuilder::new();
        if let Some(auth) = Self::authentication() {
            builder = match auth {
                octocrab::auth::Auth::PersonalToken(token) => builder.personal_token(token),
                octocrab::auth::Auth::App(app) => builder.app(app.app_id, app.key),
                _ => builder,
            };
        }

        let mut instance = builder.build()?;
        if let Some(installation_id) = std::env::var("GITHUB_APP_INSTALLATION_ID")
            .ok()
            .and_then(|id| id.parse::<u64>().ok())
        {
            instance =
                instance.installation(octocrab::models::InstallationId::from(installation_id))?;
        }

        Ok(instance)
    }

    pub fn new<S: AsRef<str>>(owner: S) -> Result<Self, GithubDiscoveryError> {
        let octocrab = Self::octocrab()?;

        Ok(Self {
            owner: owner.as_ref().to_string(),
            octocrab: Arc::new(octocrab),
            repo: None,
        })
    }

    async fn discover_repository(
        octocrab: Arc<octocrab::Octocrab>,
        repository: &octocrab::models::Repository,
    ) -> Result<Vec<GithubFile>, GithubDiscoveryError> {
        info!("Discovering repository: {:?}", repository.name);

        let mut content = octocrab
            .repos_by_id(repository.id)
            .get_content()
            .send()
            .await?;

        Ok(content
            .take_items()
            .into_iter()
            .filter(|item| item.path.ends_with(".tf"))
            .map(GithubFile::new)
            .collect())
    }

    async fn discover_impl(&self) -> Result<Vec<GithubFile>, GithubDiscoveryError> {
        let owner = &self.owner;
        let repositories_page = match self.octocrab.users(owner).repos().send().await {
            Ok(response) => response,
            Err(_) => self.octocrab.orgs(owner).list_repos().send().await?,
        };

        let repositories = self.octocrab.all_pages(repositories_page).await?;
        let mut joinset = JoinSet::new();
        for repository in repositories {
            let octocrab = self.octocrab.clone();
            joinset.spawn(async move {
                GithubDiscoverer::discover_repository(octocrab, &repository).await
            });
        }

        let results = joinset.join_all().await;
        let mut files = Vec::new();
        for result in results {
            if let Err(error) = result {
                warn!("Github discovery error: {error}");
                continue;
            }

            let result = result.unwrap();
            files.extend(result);
        }

        Ok(files)
    }
}

#[async_trait]
impl Discoverer for GithubDiscoverer {
    async fn discover(&self) -> Result<Vec<Box<dyn File>>, DiscoveryError> {
        let result = self.discover_impl().await?;
        let result = result.into_iter().map(|file| file.boxed()).collect();
        Ok(result)
    }
}
