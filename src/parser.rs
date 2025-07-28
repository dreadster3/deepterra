use std::{fs, path};

use futures::future::join_all;
use thiserror::Error;

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

#[derive(Debug, Default)]
pub struct Terraform {
    resources: Vec<String>,
}

impl Terraform {
    pub fn merge(&self, other: &Self) -> Self {
        Terraform {
            resources: self
                .resources
                .iter()
                .chain(other.resources.iter())
                .cloned()
                .collect(),
        }
    }
}

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
        let mut resources = Vec::new();

        for block in body.blocks() {
            if block.identifier() == "resource" {
                let labels = block.labels();
                let resource_type = labels[0].as_str();
                resources.push(resource_type.to_string());
            }
        }

        Ok(Terraform { resources })
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
            .filter(|path| path.is_file())
            .map(|path| file_parser.parse(path));

        let results = join_all(futures).await;
        for result in results {
            let result = result?;
            terraform = terraform.merge(&result);
        }

        Ok(terraform)
    }
}
