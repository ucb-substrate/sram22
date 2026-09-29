use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ProgressMode {
    Auto,
    Plain,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

// TODO: Add option to run Spectre simulations.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about,
    help_template(
        "{before-help}{name} {version}\n{author-with-newline}{about-with-newline}\n{usage-heading} {usage}\n\n{all-args}{after-help}"
    )
)]
pub struct Args {
    /// Path to TOML configuration file.
    #[arg(short, long, default_value = "sram22.toml")]
    pub config: PathBuf,

    /// Directory to which output files should be saved.
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Generate LIB timing with Liberate MX instead of interpolation.
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub liberate: bool,

    /// Run DRC using Calibre.
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub drc: bool,

    /// Run LVS using Calibre.
    #[cfg(feature = "commercial")]
    #[arg(long)]
    pub lvs: bool,

    #[cfg(feature = "commercial")]
    /// Run DRC and LVS as well as generation. Use --liberate for characterization.
    #[arg(short, long)]
    pub all: bool,

    /// Regenerate every SRAM, even if its outputs already exist.
    #[arg(short, long)]
    pub force: bool,

    /// Maximum number of SRAMs to generate concurrently. Defaults to no limit
    /// (all at once). This limit also applies when PEX or Liberate MX is selected.
    #[arg(short = 'p', long)]
    pub parallel: Option<usize>,

    /// Progress display: live table on terminals, plain lines when redirected.
    #[arg(long, value_enum, default_value = "auto")]
    pub progress: ProgressMode,

    /// When to use colored output. Auto respects NO_COLOR.
    #[arg(long, value_enum, default_value = "auto")]
    pub color: ColorMode,

    /// Print the full batch plan, stage transitions, and artifact directories.
    #[arg(short, long, conflicts_with = "quiet")]
    pub verbose: bool,

    /// Show only errors.
    #[arg(short, long)]
    pub quiet: bool,
}
