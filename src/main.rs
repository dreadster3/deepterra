use std::{collections::HashMap, process::ExitCode};

use charming::{
    Chart, HtmlRenderer,
    component::{Legend, Title},
    element::{LineStyle, Tooltip},
    series::{Graph, GraphCategory, GraphData, GraphLayout, GraphLink, GraphNode},
};
use env_logger::Env;
use log::{error, info};

mod parser;
mod terraform;

const SYMBOL_SIZE: f64 = 20.0;

async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let terraform = parser::DirectoryParser::parse("./terraform").await?;
    info!("{terraform:?}");

    let mut graph_data = GraphData {
        nodes: vec![],
        links: vec![],
        categories: vec![
            GraphCategory {
                name: "module".to_string(),
            },
            GraphCategory {
                name: "resource".to_string(),
            },
        ],
    };

    let mut resources = HashMap::<String, GraphNode>::new();
    let current = terraform;
    let mut id = 0;
    loop {
        let node = GraphNode {
            id: id.to_string(),
            name: current.name.clone(),
            x: id as f64,
            y: id as f64,
            category: 0,
            value: 1.0,
            symbol_size: SYMBOL_SIZE,
            label: None,
        };

        id += 1;

        graph_data.nodes.push(node.clone());

        for resource in current.resources.iter() {
            if let Some(resource_node) = resources.get_mut(&resource.kind) {
                resource_node.value += 1.0;
                resource_node.symbol_size = resource_node.value * SYMBOL_SIZE;
                continue;
            }

            let resource_node = GraphNode {
                id: id.to_string(),
                name: resource.kind.clone(),
                x: id as f64,
                y: id as f64,
                category: 1,
                value: 1.0,
                symbol_size: SYMBOL_SIZE,
                label: None,
            };
            id += 1;
            resources.insert(resource.kind.clone(), resource_node);
        }

        for resource_node in resources.values() {
            let link = GraphLink {
                source: node.id.clone(),
                target: resource_node.id.clone(),
                value: Some(resource_node.value),
            };

            graph_data.nodes.push(resource_node.to_owned());
            graph_data.links.push(link);
        }

        if true {
            break;
        }
    }

    let legend = vec!["module", "resource"];
    let chart = Chart::new()
        .title(Title::new().text("DeepTerra"))
        .legend(Legend::new().data(legend))
        .tooltip(Tooltip::new())
        .series(
            Graph::new()
                .layout(GraphLayout::Circular)
                .roam(true)
                .data(graph_data)
                .line_style(LineStyle::new().color("source").curveness(0.3)),
        );

    let mut renderer = HtmlRenderer::new("DeepTerra", 1000, 1000);
    renderer.save(&chart, "deepterra.html")?;

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = _main().await {
        error!("{e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
