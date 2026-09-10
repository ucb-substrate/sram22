use clap::Parser;
use std::path::PathBuf;

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

    /// Open SKY130 model corner embedded in the portable SPICE output.
    #[arg(long, default_value = "tt", value_parser = ["tt", "ss", "ff"])]
    pub spice_corner: String,

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

    /// Maximum number of SRAMs to generate concurrently. Defaults to no limit
    /// (all at once). This limit also applies when PEX or Liberate MX is selected.
    #[arg(short = 'p', long)]
    pub parallel: Option<usize>,
}
