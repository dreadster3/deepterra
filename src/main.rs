use anyhow::Result;
use std::{env, process::ExitCode};

use charming::{
    Chart, HtmlRenderer,
    component::{Legend, Title},
    element::{LineStyle, Tooltip},
    series::{Graph, GraphLayout},
};
use env_logger::Env;
use log::{debug, error};

mod parser;
mod terraform;

async fn _main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("off")).init();

    let mut args = env::args();
    let path = match args.nth(1) {
        Some(path) => path,
        None => {
            eprintln!("Usage: deepterra <path>");
            return Err(anyhow::anyhow!("No path provided"));
        }
    };

    let terraform = parser::DirectoryParser::parse(path).await?;
    debug!("{terraform:?}");

    let graph_data = terraform.to_graph();
    debug!("{graph_data:?}");

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
