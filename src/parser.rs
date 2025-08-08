use futures::future::BoxFuture;
use log::info;
use std::{fs, path};
use thiserror::Error;
use tokio::join;
use tokio::task::JoinSet;

use crate::terraform;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to read file: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed to parse HCL ({0}): {1}")]
    HCLError(String, hcl::Error),

    #[error("Invalid path provided")]
    InvalidPath,
}

type Result<T> = std::result::Result<T, ParseError>;

struct FileParser {}

impl FileParser {
    pub async fn parse<P: AsRef<path::Path>>(path: P) -> Result<terraform::TerraformFile> {
        let path = path.as_ref();
        info!("Parsing file: {path:?}");

        if !path.exists() || !path.is_file() {
            return Err(ParseError::InvalidPath);
        }

        let contents = fs::read_to_string(path)?;
        let body: hcl::Body = hcl::from_str(contents.as_str())
            .map_err(|e| ParseError::HCLError(path.to_string_lossy().to_string(), e))?;

        Ok(body.into())
    }
}

pub struct DirectoryParser {}

impl DirectoryParser {
    pub fn parse<P: AsRef<path::Path> + Send + 'static>(
        path: P,
    ) -> BoxFuture<'static, Result<terraform::TerraformManifest>> {
        Box::pin(async move {
            let path = path.as_ref();
            info!("Parsing directory: {path:?}");

            if !path.exists() || !path.is_dir() {
                return Err(ParseError::InvalidPath);
            }

            let absolute_path = path.canonicalize()?;

            let folder_name = match absolute_path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => String::from("root"),
            };

            let mut terraform = terraform::TerraformManifest::new(folder_name, path);

            let mut file_tasks = JoinSet::new();
            let mut submodule_tasks = JoinSet::new();

            let entries = fs::read_dir(absolute_path)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    submodule_tasks.spawn(DirectoryParser::parse(path));
                } else {
                    file_tasks.spawn(FileParser::parse(path));
                }
            }

            let (files, submodules) = join!(file_tasks.join_all(), submodule_tasks.join_all());

            files
                .into_iter()
                .filter_map(|file| file.ok())
                .for_each(|file| {
                    terraform.merge_file(file);
                });

            submodules
                .into_iter()
                .filter_map(|submodule| submodule.ok())
                .for_each(|submodule| {
                    terraform.add_submodule(submodule);
                });

            Ok(terraform)
        })
    }
}
