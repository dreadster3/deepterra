use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

use env_logger::Env;
use log::{debug, error, info};

use crate::discovery::{Discoverer, File};

mod cli;
mod discovery;
mod parser;

async fn _main() -> Result<()> {
    let args = cli::Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or(args.log_level())).init();

    let discoverer = discovery::local::LocalDiscoverer::new(args.path);

    let files = discoverer.discover().await?;

    let glob_pattern = args
        .ignore
        .as_ref()
        .map(|ignore| glob::Pattern::new(ignore))
        .transpose()?;

    let files_filtered: Vec<_> = files
        .into_iter()
        .filter(|file| {
            glob_pattern
                .as_ref()
                .is_none_or(|pattern| !pattern.matches_path(file.path()))
        })
        .collect();

    debug!("Discovered files: {:?}", files_filtered);

    let manifest = parser::Parser::parse(files_filtered.into_iter()).await?;
    info!("Manifest:\n{}", manifest);

    Ok(())

    // let terraform = parser
    //     .parse(args.path.as_str())
    //     .await
    //     .context("Failed to parse terraform manifest")?;
    // debug!("{terraform:?}");
    //
    // let graph_data = terraform.to_graph();
    // debug!("{graph_data:?}");
    //
    // let legend = graph_data
    //     .categories
    //     .iter()
    //     .map(|category| category.name.as_ref())
    //     .collect();
    // let chart = Chart::new()
    //     .title(Title::new().text("DeepTerra"))
    //     .legend(Legend::new().data(legend))
    //     .tooltip(Tooltip::new())
    //     .series(
    //         Graph::new()
    //             .layout(GraphLayout::Circular)
    //             .roam(true)
    //             .data(graph_data)
    //             .line_style(LineStyle::new().color("source").curveness(0.3)),
    //     );
    //
    // let mut renderer = HtmlRenderer::new("DeepTerra", 1000, 1000);
    // renderer
    //     .save(&chart, &args.output)
    //     .with_context(|| format!("Failed to save to {}", args.output))?;
    // println!("Saved to {}", args.output);
    //
    // Ok(())
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
