#![allow(unexpected_cfgs)]

#[cfg(feature = "commercial")]
use std::path::PathBuf;

#[cfg(feature = "commercial")]
use crate::verification::calibre::SKY130_LAYERPROPS_PATH;
pub use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use ngspice::Ngspice;
#[cfg(feature = "commercial")]
use sky130_commercial_pdk::Sky130CommercialPdk;
#[cfg(feature = "commercial")]
use spectre::Spectre;
#[cfg(feature = "commercial")]
use sub_calibre::CalibreDrc;
#[cfg(feature = "commercial")]
use sub_calibre::CalibreLvs;
#[cfg(feature = "commercial")]
use sub_calibre::CalibrePex;
use substrate::data::{SubstrateConfig, SubstrateCtx};
use substrate::schematic::netlist::impls::spice::SpiceNetlister;
use substrate::verification::simulation::{Simulator, SimulatorOpts};
use tera::Tera;

pub mod abs;
pub mod assets;
pub mod blocks;
pub mod cli;
#[cfg(feature = "commercial")]
pub mod liberate;
pub mod liberty;
pub mod measure;
pub mod paths;
pub mod pex;
pub mod plan;
pub mod spice;
pub mod tech;
pub mod verification;
pub mod verilog;

pub const BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");
pub const LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lib");
#[cfg(feature = "commercial")]
pub const SKY130_COMMERCIAL_PDK_ROOT: &str = env!("SKY130_COMMERCIAL_PDK_ROOT");

lazy_static! {
    pub static ref TEMPLATES: Tera = match Tera::new(
        assets::path("templates/*")
            .to_str()
            .expect("invalid temporary path")
    ) {
        Ok(t) => t,
        Err(e) => panic!("Error parsing templates: {e}"),
    };
}

pub fn bus_bit(name: &str, index: usize) -> String {
    format!("{name}[{index}]")
}

pub fn setup_ctx() -> SubstrateCtx {
    try_setup_ctx().expect("failed to initialize SRAM22")
}

pub fn try_setup_open_ctx() -> Result<SubstrateCtx> {
    let cfg = SubstrateConfig::builder()
        .pdk(tech::sky130::OpenPdk::new()?)
        .netlister(SpiceNetlister::new())
        .simulator(Ngspice::new(SimulatorOpts::default())?)
        .build();
    Ok(SubstrateCtx::from_config(cfg)?)
}

pub fn try_setup_ctx() -> Result<SubstrateCtx> {
    #[cfg(not(feature = "commercial"))]
    let simulator = Ngspice::new(SimulatorOpts::default())?;

    #[cfg(feature = "commercial")]
    let simulator = Spectre::new(SimulatorOpts::default())?;

    let mut builder = SubstrateConfig::builder();

    #[cfg(feature = "commercial")]
    let builder = builder
        .pdk(Sky130CommercialPdk::new(
            PathBuf::from(SKY130_COMMERCIAL_PDK_ROOT),
            assets::standard_cell_root(),
        )?)
        .drc_tool(
            CalibreDrc::builder()
                .rules_file(PathBuf::from(
                    crate::verification::calibre::SKY130_DRC_RULES_PATH,
                ))
                .runset_file(PathBuf::from(
                    crate::verification::calibre::SKY130_DRC_RUNSET_PATH,
                ))
                .layerprops(PathBuf::from(SKY130_LAYERPROPS_PATH))
                .build()?,
        )
        .lvs_tool(
            CalibreLvs::builder()
                .rules_file(PathBuf::from(
                    crate::verification::calibre::SKY130_LVS_RULES_PATH,
                ))
                .layerprops(PathBuf::from(SKY130_LAYERPROPS_PATH))
                .build()?,
        )
        .pex_tool(CalibrePex::new(PathBuf::from(
            crate::verification::calibre::SKY130_PEX_RULES_PATH,
        )));
    #[cfg(not(feature = "commercial"))]
    let builder = builder.pdk(tech::sky130::OpenPdk::new()?);

    #[cfg(feature = "commercial")]
    builder.simulation_bashrc("/tools/B/rahulkumar/sky130/priv/drc/.bashrc");

    let cfg = builder
        .netlister(SpiceNetlister::new())
        .simulator(simulator)
        .build();

    Ok(SubstrateCtx::from_config(cfg)?)
}

#[cfg(test)]
pub mod tests {
    use std::path::PathBuf;

    use super::BUILD_PATH;

    pub(crate) fn test_work_dir(name: &str) -> PathBuf {
        PathBuf::from(BUILD_PATH).join(name)
    }
}
