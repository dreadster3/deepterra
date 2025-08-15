use crate::terraform;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

mod github;
mod local;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to read file: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed to parse HCL: {0}")]
    HCLError(#[from] hcl::Error),

    #[error("Invalid path provided")]
    PathError,

    #[error("Invalid glob pattern: {0}")]
    GlobError(#[from] glob::PatternError),

    #[error("Github parser error: {0}")]
    GithubError(#[from] github::GithubParseError),

    #[error("Skipping directory")]
    Skip,
}

type Result<T> = std::result::Result<T, ParseError>;

pub struct ParserOptions {
    ignore: Option<String>,
}

impl ParserOptions {
    pub fn new(ignore: Option<String>) -> Self {
        Self { ignore }
    }
}

pub struct Parser {
    options: Arc<ParserOptions>,
}

impl Parser {
    pub fn new(options: ParserOptions) -> Self {
        Self {
            options: Arc::new(options),
        }
    }

    pub async fn parse(&self, path: impl Into<String>) -> Result<terraform::TerraformManifest> {
        let path: String = path.into();
        if let Some(path) = path.strip_prefix("github:") {
            return github::GithubParser::parse(path).await;
        }

        let path = PathBuf::from(path);
        local::DirectoryParser::parse(path, self.options.clone()).await
    }
}
