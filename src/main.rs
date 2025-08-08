use std::process::ExitCode;

use charming::{
    Chart, HtmlRenderer,
    component::{Legend, Title},
    element::Tooltip,
    series::{Graph, GraphCategory, GraphData, GraphLink, GraphNode},
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
            let link = GraphLink {
                source: node.id.clone(),
                target: resource_node.id.clone(),
                value: None,
            };
            id += 1;

            graph_data.nodes.push(resource_node);
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
        .series(Graph::new().data(graph_data));

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
