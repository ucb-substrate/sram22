use anyhow::Result;
use sram22::cli::run;

fn main() -> Result<()> {
    let result = run();
    sram22::assets::cleanup();
    result
}
