use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Output file name
    #[arg(short, long, default_value_t = String::from("deepterra.html"))]
    pub output: String,

    /// Ignore files matching this glob pattern
    #[arg(short, long)]
    pub ignore: Vec<String>,

    /// Enable verbose logging (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Path to the terraform manifest
    pub path: String,
}

impl Args {
    pub fn log_level(&self) -> &str {
        match self.verbose {
            0 => "off",
            1 => "info",
            _ => "debug",
        }
    }
}
