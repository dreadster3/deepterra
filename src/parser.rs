use futures::future::join_all;
use std::{fs, path};
use thiserror::Error;

use crate::terraform::Terraform;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to read file: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed to parse HCL: {0}")]
    HCLError(#[from] hcl::Error),

    #[error("Invalid path provided")]
    InvalidPath,
}

type Result<T> = std::result::Result<T, ParseError>;

pub trait Parser {
    async fn parse<P: AsRef<path::Path>>(&self, path: P) -> Result<Terraform>;
}

pub struct FileParser {}

impl FileParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl Parser for FileParser {
    async fn parse<P: AsRef<path::Path>>(&self, path: P) -> Result<Terraform> {
        let path = path.as_ref();
        if !path.exists() || !path.is_file() {
            return Err(ParseError::InvalidPath);
        }

        let contents = fs::read_to_string(path)?;
        let body: hcl::Body = hcl::from_str(contents.as_str())?;
        Ok(body.into())
    }
}

pub struct DirectoryParser {}

impl DirectoryParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl Parser for DirectoryParser {
    async fn parse<P: AsRef<path::Path>>(&self, path: P) -> Result<Terraform> {
        let path = path.as_ref();
        if !path.exists() || !path.is_dir() {
            return Err(ParseError::InvalidPath);
        }

        let file_parser = FileParser::new();
        let mut terraform = Terraform::default();

        let entries = fs::read_dir(path)?;

        let futures = entries
            .into_iter()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .map(|path| async {
                if path.is_dir() {
                    return self.parse(path).await;
                }

                return file_parser.parse(path).await;
            });

        let results = join_all(futures).await;
        for result in results {
            let result = result?;
            terraform += result;
        }

        Ok(terraform)
    }
}
