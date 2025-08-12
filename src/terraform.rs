use std::{collections::HashMap, fmt::Debug, path};

use log::{debug, info};

const RESOURCE_KEY: &str = "resource";
const MODULE_KEY: &str = "module";
const MODULE_SOURCE_KEY: &str = "source";
const SYMBOL_SIZE: f64 = 10.0;
const SYMBOL_SIZE_FACTOR: f64 = 2.0;

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
        if !self.resources.is_empty() || !self.modules.is_empty() {
            let module_node_id = uuid::Uuid::new_v4().to_string();
            let module_node = charming::series::GraphNode {
                id: module_node_id.clone(),
                name: self.name.clone(),
                x: 0.0,
                y: 0.0,
                category: 0,
                value: 1.0,
                symbol_size: SYMBOL_SIZE,
                label: None,
            };
            modules.insert(module_node.name.clone(), module_node);

            for resource in self.resources.iter() {
                if let Some(resource_node) = resources.get_mut(&resource.kind) {
                    debug!("Resource {resource:?} already exists, incrementing value");
                    resource_node.value += 1.0;
                    resource_node.symbol_size =
                        resource_node.value * SYMBOL_SIZE_FACTOR + SYMBOL_SIZE;

                    if let Some(link) = links.iter_mut().find(|link| {
                        link.source == module_node_id.clone() && link.target == resource_node.id
                    }) {
                        link.value = Some(link.value.unwrap_or(1.0f64) + 1.0f64);
                    } else {
                        let link = charming::series::GraphLink {
                            source: module_node_id.clone(),
                            target: resource_node.id.clone(),
                            value: Some(1.0),
                        };
                        links.push(link);
                    }

                    continue;
                }

                info!("Found new resource {resource:?}");
                let resource_node_id = uuid::Uuid::new_v4().to_string();
                let resource_node = charming::series::GraphNode {
                    id: resource_node_id.clone(),
                    name: resource.kind.clone(),
                    x: 0.0,
                    y: 0.0,
                    category: 1,
                    value: 1.0,
                    symbol_size: SYMBOL_SIZE,
                    label: None,
                };
                resources.insert(resource.kind.clone(), resource_node);
                links.push(charming::series::GraphLink {
                    source: module_node_id.clone(),
                    target: resource_node_id.clone(),
                    value: Some(1.0),
                });
            }

            for module in self.modules.iter() {
                if let Some(module_ref_node) = modules.get_mut(&module.name()) {
                    debug!("Module {module:?} already exists, incrementing value");
                    module_ref_node.value += 1.0;
                    module_ref_node.symbol_size =
                        module_ref_node.value * SYMBOL_SIZE_FACTOR + SYMBOL_SIZE;

                    if let Some(link) = links.iter_mut().find(|link| {
                        link.source == module_node_id.clone() && link.target == module_ref_node.id
                    }) {
                        link.value = Some(link.value.unwrap_or(1.0f64) + 1.0f64);
                    } else {
                        let link = charming::series::GraphLink {
                            source: module_node_id.clone(),
                            target: module_ref_node.id.clone(),
                            value: Some(1.0),
                        };
                        links.push(link);
                    }

                    continue;
                }

                info!("Found new module {module:?}");
                let module_ref_node_id = uuid::Uuid::new_v4().to_string();
                let module_ref_node = charming::series::GraphNode {
                    id: module_ref_node_id.clone(),
                    name: module.name().to_string(),
                    x: 0.0,
                    y: 0.0,
                    category: 1,
                    value: 1.0,
                    symbol_size: SYMBOL_SIZE,
                    label: None,
                };

                modules.insert(module.name().to_string(), module_ref_node);
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

trait ModuleSource {
    fn source(&self) -> &str;
    fn name(&self) -> String;
}

#[derive(Debug)]
pub struct LocalModuleRef {
    source: String,
}

impl LocalModuleRef {
    fn new(source: String) -> Self {
        Self { source }
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
}

impl ModuleSource for GitModuleRef {
    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn name(&self) -> String {
        url::Url::parse(self.source().strip_prefix("git::").unwrap_or(self.source()))
            .ok()
            .as_ref()
            .and_then(|url| url.path_segments())
            .and_then(|mut segments| segments.next_back())
            .unwrap_or_default()
            .to_string()
    }
}

impl ModuleSource for LocalModuleRef {
    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn name(&self) -> String {
        let path = path::Path::new(self.source());

        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string()
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

impl ModuleSource for ModuleRef {
    fn source(&self) -> &str {
        match self {
            Self::Local(module_ref) => module_ref.source(),
            Self::Git(module_ref) => module_ref.source(),
            _ => todo!(),
        }
    }

    fn name(&self) -> String {
        match self {
            Self::Local(module_ref) => module_ref.name(),
            Self::Git(module_ref) => module_ref.name(),
            _ => todo!(),
        }
    }
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
