use std::{collections::HashMap, fmt::Debug, path};

const RESOURCE_KEY: &str = "resource";
const MODULE_KEY: &str = "module";
const MODULE_SOURCE_KEY: &str = "source";
const SYMBOL_SIZE: f64 = 20.0;

#[derive(Debug)]
pub struct TerraformManifest {
    pub name: String,
    pub path: path::PathBuf,
    pub resources: Vec<Resource>,
    pub modules: Vec<ModuleRef>,
    pub submodules: Vec<TerraformManifest>,
}

impl TerraformManifest {
    pub fn new<N: Into<String>, P: AsRef<path::Path>>(name: N, path: P) -> Self {
        let path = path.as_ref();
        Self {
            name: name.into(),
            path: path.to_path_buf(),
            resources: Vec::new(),
            modules: Vec::new(),
            submodules: Vec::new(),
        }
    }

    pub fn merge_file(&mut self, file: TerraformFile) -> &Self {
        self.resources.extend(file.resources);
        self.modules.extend(file.modules);

        self
    }

    pub fn add_submodule(&mut self, submodule: TerraformManifest) -> &Self {
        self.submodules.push(submodule);

        self
    }

    fn process_manifest_resources(
        &self,
        modules: &mut HashMap<String, charming::series::GraphNode>,
        resources: &mut HashMap<String, charming::series::GraphNode>,
        links: &mut Vec<charming::series::GraphLink>,
    ) {
        if !self.resources.is_empty() {
            let module_node = charming::series::GraphNode {
                id: uuid::Uuid::new_v4().to_string(),
                name: self.name.clone(),
                x: 0.0,
                y: 0.0,
                category: 0,
                value: 1.0,
                symbol_size: SYMBOL_SIZE,
                label: None,
            };
            modules.insert(module_node.name.clone(), module_node.clone());

            for resource in self.resources.iter() {
                if let Some(resource_node) = resources.get_mut(&resource.kind) {
                    resource_node.value += 1.0;
                    resource_node.symbol_size = resource_node.value * SYMBOL_SIZE;

                    if let Some(link) = links.iter_mut().find(|link| {
                        link.source == module_node.id && link.target == resource_node.id
                    }) {
                        link.value = Some(link.value.unwrap_or(1.0f64) + 1.0f64);
                    } else {
                        let link = charming::series::GraphLink {
                            source: module_node.id.clone(),
                            target: resource_node.id.clone(),
                            value: Some(1.0),
                        };
                        links.push(link);
                    }

                    continue;
                }

                let resource_node = charming::series::GraphNode {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: resource.kind.clone(),
                    x: 0.0,
                    y: 0.0,
                    category: 1,
                    value: 1.0,
                    symbol_size: SYMBOL_SIZE,
                    label: None,
                };
                resources.insert(resource.kind.clone(), resource_node.clone());
                links.push(charming::series::GraphLink {
                    source: module_node.id.clone(),
                    target: resource_node.id.clone(),
                    value: Some(1.0),
                });
            }
        }

        for submodule in self.submodules.iter() {
            submodule.process_manifest_resources(modules, resources, links);
        }
    }

    pub fn to_graph(&self) -> charming::series::GraphData {
        let mut graph_data = charming::series::GraphData {
            nodes: vec![],
            links: vec![],
            categories: vec![
                charming::series::GraphCategory {
                    name: "module".to_string(),
                },
                charming::series::GraphCategory {
                    name: "resource".to_string(),
                },
            ],
        };

        let mut modules = HashMap::<String, charming::series::GraphNode>::new();
        let mut resources = HashMap::<String, charming::series::GraphNode>::new();
        let mut links = Vec::<charming::series::GraphLink>::new();

        self.process_manifest_resources(&mut modules, &mut resources, &mut links);

        graph_data.nodes.extend(modules.into_values());
        graph_data.nodes.extend(resources.into_values());
        graph_data.links.extend(links);

        graph_data
    }
}

#[derive(Debug)]
pub struct TerraformFile {
    pub resources: Vec<Resource>,
    pub modules: Vec<ModuleRef>,
}

impl From<hcl::Body> for TerraformFile {
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

        Self { resources, modules }
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
pub enum ModuleRef {
    Local(String),
    Git(String),
    S3(String),
    Bitbucket(String),
    Mercurial(String),
    Http(String),
    GCS(String),
    Registry(String),
    Unknown,
}

impl ModuleRef {
    pub fn source(&self) -> &str {
        match self {
            Self::Local(source) => source,
            Self::Git(source) => source,
            Self::S3(source) => source,
            Self::Bitbucket(source) => source,
            Self::Mercurial(source) => source,
            Self::Http(source) => source,
            Self::GCS(source) => source,
            Self::Registry(source) => source,
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(source: &str) -> Self {
        let source = source.trim();

        if source.starts_with("./") || source.starts_with("../") {
            return Self::Local(source.to_string());
        }

        if source.starts_with("git::") || source.contains("github.com") {
            return Self::Git(source.to_string());
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
}

impl From<&hcl::Block> for ModuleRef {
    fn from(block: &hcl::Block) -> Self {
        let body = block.body();
        let source = body
            .attributes()
            .find(|attr| attr.key() == MODULE_SOURCE_KEY);

        if let Some(source) = source {
            if let hcl::expr::Expression::String(source) = source.expr() {
                return ModuleRef::parse(source);
            }
        }

        Self::Unknown
    }
}
