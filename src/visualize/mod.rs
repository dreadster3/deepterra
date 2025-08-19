use std::collections::HashMap;

use charming::series::{GraphCategory, GraphData, GraphLink, GraphNode};
use log::{debug, info};

use crate::parser::Manifest;

const SYMBOL_SIZE: f64 = 10.0;
const SYMBOL_SIZE_FACTOR: f64 = 2.0;

fn add_node(
    nodes: &mut HashMap<String, GraphNode>,
    links: &mut Vec<GraphLink>,
    name: &str,
    category: u64,
    source: Option<&str>,
) -> GraphNode {
    let value = (source.is_some() as u64) as f64;

    if let Some(node) = nodes.get_mut(name) {
        debug!("Node {name} already exists, incrementing value");
        node.value += value;
        node.symbol_size = node.value * SYMBOL_SIZE_FACTOR + SYMBOL_SIZE;

        if let Some(source) = source {
            if let Some(link) = links
                .iter_mut()
                .find(|link| link.source == source && link.target == node.id)
            {
                link.value = Some(link.value.unwrap_or(1.0f64) + 1.0f64);
            } else {
                let link = charming::series::GraphLink {
                    source: source.to_string(),
                    target: node.id.clone(),
                    value: Some(1.0),
                };
                links.push(link);
            }
        }

        return node.to_owned();
    }

    info!("Node {name} doesn't exist, creating new node");
    let node_id = uuid::Uuid::new_v4().to_string();
    let node = charming::series::GraphNode {
        id: node_id,
        name: name.to_string(),
        x: 0.0,
        y: 0.0,
        category,
        value,
        symbol_size: SYMBOL_SIZE,
        label: None,
    };
    nodes.insert(name.to_string(), node.clone());

    if let Some(source) = source {
        links.push(charming::series::GraphLink {
            source: source.to_string(),
            target: node.id.clone(),
            value: Some(1.0),
        });
    }

    node
}

impl From<Manifest> for GraphData {
    fn from(manifest: Manifest) -> Self {
        let mut modules: HashMap<String, GraphNode> = HashMap::new();
        let mut resources: HashMap<String, GraphNode> = HashMap::new();
        let mut links: Vec<GraphLink> = Vec::new();

        for node in manifest.iter() {
            let terraform = node.get();
            if terraform.is_empty() {
                continue;
            }

            info!("Node: {terraform:?}");

            let source_node = add_node(&mut modules, &mut links, &terraform.name, 0, None);

            terraform.resources.iter().for_each(|resource| {
                add_node(
                    &mut resources,
                    &mut links,
                    &resource.kind,
                    1,
                    Some(&source_node.id),
                );
            });

            terraform.modules.iter().for_each(|module_ref| {
                add_node(
                    &mut modules,
                    &mut links,
                    &module_ref.name(),
                    0,
                    Some(&source_node.id),
                );
            })
        }

        info!("modules: {modules:?}");

        GraphData {
            nodes: resources
                .into_values()
                .chain(modules.into_values())
                .collect(),
            links,
            categories: vec![
                GraphCategory {
                    name: "Module".to_string(),
                },
                GraphCategory {
                    name: "Resource".to_string(),
                },
            ],
        }
    }
}
