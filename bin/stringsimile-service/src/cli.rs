use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[clap(short, long)]
    pub rules_path: Option<PathBuf>,
    #[clap(short, long)]
    pub verbose: bool,
}
