use anyhow::{Context, Result};
use charming::{
    Chart, HtmlRenderer,
    component::{Legend, Title},
    element::{LineStyle, Tooltip},
    series::{Graph, GraphData, GraphLayout},
};
use clap::Parser;
use std::process::ExitCode;

use env_logger::Env;
use log::{debug, error, info};

use crate::discovery::{Discoverer, File};

mod cli;
mod discovery;
mod parser;
mod visualize;

async fn _main() -> Result<()> {
    let args = cli::Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or(args.log_level())).init();

    let discoverer = discovery::get_discoverer(args.path);

    let files = discoverer.discover().await?;

    let glob_patterns = args
        .ignore
        .iter()
        .map(|ignore| glob::Pattern::new(ignore.as_str()))
        .filter_map(|pattern| pattern.ok());

    let files_filtered: Vec<_> = files
        .into_iter()
        .filter(|file| {
            glob_patterns
                .clone()
                .all(|pattern| !pattern.matches(&file.path().to_string_lossy()))
        })
        .collect();

    debug!("Discovered files: {:?}", files_filtered);

    let manifest = parser::Parser::parse(files_filtered.into_iter()).await?;
    debug!("Manifest:\n{}", manifest);

    let graph_data: GraphData = manifest.into();

    let legend = graph_data
        .categories
        .iter()
        .map(|category| category.name.as_ref())
        .collect();

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
    renderer
        .save(&chart, &args.output)
        .with_context(|| format!("Failed to save to {}", args.output))?;
    info!("Saved to {}", args.output);

    println!("Saved to {}", args.output);

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(e) = _main().await {
        error!("{e}");
        eprintln!("{e:?}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
