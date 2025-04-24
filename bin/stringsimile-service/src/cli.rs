use std::path::PathBuf;

use clap::{ArgAction, Parser};

#[derive(Parser)]
#[command(version, about, long_about = None, rename_all = "kebab-case")]
pub struct CliArgs {
    /// Path to the directory (or file) containing rules
    #[clap(short, long, default_value = "/var/lib/stringsimile")]
    pub rules_path: PathBuf,

    /// Increase log verbosity. May be repeated for further increase.
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    /// Decrease log verbosity. May be repeated for further decrease.
    #[clap(short, long, action = ArgAction::Count)]
    pub quiet: u8,

    /// Path to configuration file.
    #[clap(short, long, default_value = "/etc/stringsimile/stringsimile.yaml")]
    pub config: PathBuf,

    /// Optionally set file to read input data from.
    #[clap(long)]
    pub input_file: Option<PathBuf>,

    /// Set to true to read input data from stdin.
    #[clap(long, default_value_t = false)]
    pub input_from_stdin: bool,

    /// Optionally set file to write output data to.
    #[clap(long)]
    pub output_file: Option<PathBuf>,

    /// Set to true to write output data to stdout.
    #[clap(long, default_value_t = false)]
    pub output_to_stdout: bool,

    /// Field to take from input JSON object, to match against rules.
    #[clap(long, default_value = ".domain_name")]
    pub input_field: String,
}
