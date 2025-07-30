use std::ops::{Add, AddAssign};

const RESOURCE_KEY: &str = "resource";
const MODULE_KEY: &str = "module";
const MODULE_SOURCE_KEY: &str = "source";

#[derive(Debug, Default)]
pub struct Terraform {
    resources: Vec<Resource>,
    modules: Vec<Module>,
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
            modules: self
                .modules
                .iter()
                .chain(other.modules.iter())
                .cloned()
                .collect(),
        }
    }
}

impl Add for Terraform {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.merge(&rhs)
    }
}

impl AddAssign for Terraform {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.merge(&rhs);
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

        Self { resources, modules }
    }
}

#[derive(Debug, Clone)]
struct Resource {
    name: String,
    kind: String,
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

#[derive(Debug, Clone)]
struct Module {
    source: String,
}

impl From<&hcl::Block> for Module {
    fn from(block: &hcl::Block) -> Self {
        let body = block.body();
        let source = body
            .attributes()
            .find(|attr| attr.key() == MODULE_SOURCE_KEY);

        if let Some(source) = source {
            if let hcl::expr::Expression::String(source) = source.expr() {
                return Self {
                    source: source.to_string(),
                };
            }
        }

        Self {
            source: "invalid".to_string(),
        }
    }
}
