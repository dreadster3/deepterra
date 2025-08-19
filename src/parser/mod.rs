use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use hcl::Body;
use indextree::Arena;
use log::{debug, info};
use tokio::task::JoinSet;

use crate::discovery::File;

const RESOURCE_KEY: &str = "resource";
const MODULE_KEY: &str = "module";
const MODULE_SOURCE_KEY: &str = "source";

#[derive(Debug)]
pub struct Terraform {
    pub name: String,
    resources: Vec<Resource>,
    modules: Vec<ModuleRef>,
}

impl Terraform {
    fn new<N: AsRef<str>>(name: N) -> Self {
        Self {
            name: name.as_ref().to_string(),
            resources: Vec::new(),
            modules: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty() && self.modules.is_empty()
    }

    fn combine(&mut self, other: Self) {
        self.resources.extend(other.resources);
        self.modules.extend(other.modules);
    }
}

impl From<hcl::Body> for Terraform {
    fn from(body: hcl::Body) -> Self {
        let mut resources = Vec::new();
        let mut modules = Vec::new();

        for block in body.blocks() {
            match block.identifier() {
                RESOURCE_KEY => {
                    resources.push(block.into());
                }
                MODULE_KEY => {
                    modules.push(block.into());
                }
                _ => {}
            };
        }

        Self {
            name: "".to_string(),
            resources,
            modules,
        }
    }
}

#[derive(Debug)]
pub struct Resource {
    pub name: String,
    pub kind: String,
}

impl From<&hcl::Block> for Resource {
    fn from(block: &hcl::Block) -> Self {
        let labels = block.labels();
        let kind = labels[0].as_str();
        let name = labels[1].as_str();

        Self {
            name: name.to_string(),
            kind: kind.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct LocalModuleRef {
    source: String,
}

impl LocalModuleRef {
    fn new(source: String) -> Self {
        Self { source }
    }

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn name(&self) -> String {
        let path = Path::new(self.source());

        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string()
    }
}

#[derive(Debug)]
pub struct GitModuleRef {
    source: String,
}

impl GitModuleRef {
    fn new(source: String) -> Self {
        Self { source }
    }

    fn name(&self) -> String {
        let source = self.source();
        url::Url::parse(source.strip_prefix("git::").unwrap_or(source))
            .ok()
            .as_ref()
            .and_then(|url| url.path_segments())
            .and_then(|mut segments| segments.next_back())
            .unwrap_or_default()
            .to_string()
    }

    fn source(&self) -> &str {
        self.source.as_str()
    }
}

#[derive(Debug)]
pub enum ModuleRef {
    Local(LocalModuleRef),
    Git(GitModuleRef),
    S3(String),
    Bitbucket(String),
    Mercurial(String),
    Http(String),
    GCS(String),
    Registry(String),
    Unknown,
}

impl ModuleRef {
    pub fn parse(source: &str) -> Self {
        let source = source.trim();

        if source.starts_with("./") || source.starts_with("../") {
            return Self::Local(LocalModuleRef::new(source.to_string()));
        }

        if source.starts_with("git::") || source.contains("github.com") {
            return Self::Git(GitModuleRef::new(source.to_string()));
        }

        if source.starts_with("bitbucket.org") {
            return Self::Bitbucket(source.to_string());
        }

        if source.starts_with("hg::") {
            return Self::Mercurial(source.to_string());
        }

        if source.starts_with("s3::") {
            return Self::S3(source.to_string());
        }

        if source.starts_with("gcs::") {
            return Self::GCS(source.to_string());
        }

        if source.starts_with("http::") || source.starts_with("https::") {
            return Self::Http(source.to_string());
        }

        Self::Registry(source.to_string())
    }

    pub fn source(&self) -> &str {
        match self {
            Self::Local(module_ref) => module_ref.source(),
            Self::Git(module_ref) => module_ref.source(),
            _ => todo!(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::Local(module_ref) => module_ref.name(),
            Self::Git(module_ref) => module_ref.name(),
            _ => todo!(),
        }
    }
}

impl From<&hcl::Block> for ModuleRef {
    fn from(block: &hcl::Block) -> Self {
        let body = block.body();
        let source_expression = body
            .attributes()
            .find(|attr| attr.key() == MODULE_SOURCE_KEY)
            .map(|attr| attr.expr());

        match source_expression {
            Some(hcl::expr::Expression::String(source)) => ModuleRef::parse(source),
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Failed reading file: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed parsing HCL: {0}")]
    HCLError(#[from] hcl::Error),
}

type Result<T> = std::result::Result<T, ParseError>;

pub struct Parser {}

impl Parser {
    /*
     * ./aggregator/module4/main.tf
     * ./aggregator/module1/main.tf
     * ./aggregator/module2/main.tf
     *
     * root
     *   ├── module4
     *   │   └── main.tf
     *   ├── module1
     *   │   └── main.tf
     *   └── module2
     *       └── main.tf
     */
    pub async fn parse(files: impl Iterator<Item = impl File + Send>) -> Result<Arena<Terraform>> {
        let mut arena = Arena::new();
        info!("Parsing files");

        let root = arena.new_node(Terraform::new("root"));

        for file in files {
            let path = file.path();

            debug!("Parsing file: {:?}", path);

            let path_parts = path
                .iter()
                .skip_while(|part| part == &".")
                .take(path.iter().count() - 2);
            debug!("Path parts: {:?}", path_parts.clone().collect::<Vec<_>>());

            let mut current_node_id = root;

            for part in path_parts {
                current_node_id = current_node_id
                    .children(&arena)
                    .into_iter()
                    .find(|&node_id| {
                        arena
                            .get(node_id)
                            .is_some_and(|node| node.get().name == part.to_string_lossy())
                    })
                    .unwrap_or_else(|| {
                        current_node_id
                            .append_value(Terraform::new(part.to_string_lossy()), &mut arena)
                    });
            }

            let terraform = arena
                .get_mut(current_node_id)
                .map(|node| node.get_mut())
                .unwrap();

            Parser::parse_file(file, terraform).await?;
        }

        debug!("Arena:\n{:?}", root.debug_pretty_print(&arena));
        Ok(arena)
    }

    async fn parse_file(file: impl File, terraform: &mut Terraform) -> Result<()> {
        let contents = file.get_contents()?;

        let hcl: Body = hcl::from_str(contents.as_str())?;
        let parsed_terraform: Terraform = hcl.into();

        terraform.combine(parsed_terraform);

        Ok(())
    }
}
