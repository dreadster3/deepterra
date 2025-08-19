use std::sync::Arc;

use log::info;
use tokio::task::JoinSet;

use crate::parser::{ParseError, Result};
use crate::terraform;

#[derive(Debug, thiserror::Error)]
pub enum GithubParseError {
    #[error("octocrab error: {0}")]
    OctocrabError(#[from] octocrab::Error),

    #[error("Could not find a commit for the repository")]
    NoCommitFound,

    #[error("Missing authentication")]
    MissingAuthentication,
}

pub struct GithubParser {}

impl GithubParser {
    fn authentication() -> Option<octocrab::auth::Auth> {
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            info!("Using GITHUB_TOKEN");
            return Some(octocrab::auth::Auth::PersonalToken(token.into()));
        }

        if let Ok(app_id) = std::env::var("GITHUB_APP_ID") {
            if let Ok(app_key) = std::env::var("GITHUB_APP_KEY") {
                let app_id = app_id.parse::<u64>().ok()?;
                info!("Using GITHUB_APP_ID({app_id}) and GITHUB_APP_KEY");

                return Some(octocrab::auth::Auth::App(octocrab::auth::AppAuth {
                    key: jsonwebtoken::EncodingKey::from_secret(app_key.as_bytes()),
                    app_id: octocrab::models::AppId::from(app_id),
                }));
            }
        }

        let command = std::process::Command::new("gh")
            .args(["auth", "token"])
            .output();

        if let Ok(output) = command {
            if output.status.success() {
                if let Ok(token) = String::from_utf8(output.stdout) {
                    let token = token.trim();
                    return Some(octocrab::auth::Auth::PersonalToken(token.into()));
                }
            }
        }

        None
    }

    fn octocrab() -> std::result::Result<octocrab::Octocrab, GithubParseError> {
        let mut builder = octocrab::OctocrabBuilder::new();
        if let Some(auth) = Self::authentication() {
            builder = match auth {
                octocrab::auth::Auth::PersonalToken(token) => builder.personal_token(token),
                octocrab::auth::Auth::App(app) => builder.app(app.app_id, app.key),
                _ => builder,
            };
        }

        let mut instance = builder.build()?;
        if let Ok(installation_id) = std::env::var("GITHUB_APP_INSTALLATION_ID") {
            if let Ok(installation_id) = installation_id.parse::<u64>() {
                instance = instance
                    .installation(octocrab::models::InstallationId::from(installation_id))?;
            }
        }

        Ok(instance)
    }

    async fn parse_impl(
        octocrab: Arc<octocrab::Octocrab>,
        scope: &str,
    ) -> std::result::Result<terraform::TerraformManifest, GithubParseError> {
        let owner_str = scope;

        let current_page = match octocrab.users(owner_str).repos().send().await {
            Ok(page) => page,
            Err(_) => octocrab.orgs(owner_str).list_repos().send().await?,
        };

        let repositories = octocrab.all_pages(current_page).await?;
        let mut joinset: JoinSet<std::result::Result<(), GithubParseError>> = JoinSet::new();

        for repository in repositories {
            let octocrab = octocrab.clone();
            joinset.spawn(async move {
                let commits = octocrab
                    .repos_by_id(repository.id)
                    .get_content()
                    .send()
                    .await?;

                for item in commits.items {
                    info!("item: {:?}", item);
                }

                Ok(())
            });
        }

        joinset.join_all().await;
        todo!()
    }

    pub async fn parse(scope: impl Into<String>) -> Result<terraform::TerraformManifest> {
        let octocrab = Self::octocrab()?;
        let scope = scope.into();

        let result = Self::parse_impl(Arc::new(octocrab), &scope).await?;

        Ok(result)
    }
}
