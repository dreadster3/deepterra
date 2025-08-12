use super::{ParseError, ParserOptions, Result};
use futures::future::BoxFuture;
use log::{debug, info, warn};
use std::sync::Arc;
use std::{fs, path};
use tokio::join;
use tokio::task::JoinSet;

use crate::terraform;

struct FileParser {}

impl FileParser {
    pub async fn parse(
        path: path::PathBuf,
        options: Arc<ParserOptions>,
    ) -> Result<terraform::TerraformFile> {
        let path = path.as_ref();
        info!("Parsing file: {path:?}");

        if let Some(ignore) = options.ignore.as_ref() {
            let pattern = glob::Pattern::new(ignore)?;
            if pattern.matches_path(path) {
                debug!("Skipping file: {path:?}");
                return Err(ParseError::Skip);
            }
        }

        if !path.exists() || !path.is_file() {
            return Err(ParseError::PathError);
        }

        let contents = fs::read_to_string(path)
            .inspect_err(|e| warn!("failed to read file: {path:?}\n{e}"))?;
        let body: hcl::Body = hcl::from_str(contents.as_str())
            .inspect_err(|e| warn!("failed to parse file: {path:?}\n{e}"))?;

        Ok(body.into())
    }
}

pub struct DirectoryParser {}

impl DirectoryParser {
    pub fn parse(
        path: path::PathBuf,
        options: Arc<ParserOptions>,
    ) -> BoxFuture<'static, Result<terraform::TerraformManifest>> {
        Box::pin(async move {
            if !path.exists() || !path.is_dir() {
                warn!("Invalid path provided: {path:?}");
                return Err(ParseError::PathError);
            }

            if let Some(ignore) = options.ignore.as_ref() {
                let pattern = glob::Pattern::new(ignore)?;
                if pattern.matches_path(path.as_path()) {
                    debug!("Skipping directory: {path:?}");
                    return Err(ParseError::Skip);
                }
            }

            info!("Parsing directory: {path:?}");
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
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                if file_name.starts_with(".") {
                    debug!("Skipping hidden file or directory: {file_name}");
                    continue; // Skip hidden files and directories
                }

                if path.is_dir() {
                    submodule_tasks.spawn(DirectoryParser::parse(path, options.clone()));
                } else {
                    file_tasks.spawn(FileParser::parse(path, options.clone()));
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
