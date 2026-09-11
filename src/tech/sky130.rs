//! SKY130 geometry, bundled cell views, and external simulation models.
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use sky130_open_pdk::Sky130OpenPdk;
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::ViaParams;
use substrate::layout::layers::Layers;
use substrate::pdk::corner::CornerDb;
use substrate::pdk::mos::spec::MosSpec;
use substrate::pdk::mos::{LayoutMosParams, MosParams};
use substrate::pdk::stdcell::StdCellDb;
use substrate::pdk::{Pdk, PdkParams, Supplies, Units};
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::netlist::{IncludeBundle, NetlistPurpose};
use substrate::units::SiPrefix;

pub const SKY130_DOMAIN: &str = "sky130";
pub const SRAM_SP_CELL: &str = "sram_sp_cell";
pub const SRAM_SP_COLEND: &str = "sky130_fd_bd_sram__sram_sp_colend";
pub const SRAM_SP_CELL_REPLICA: &str = "sram_sp_cell_replica";
pub const OPENRAM_DFF: &str = "openram_dff";
pub const SRAM_CONTROL_SIMPLE: &str = "sramgen_control_simple";
pub const SRAM_CONTROL_REPLICA_V1: &str = "sramgen_control_replica_v1";
pub const SRAM_CONTROL_BUFBUF_16: &str = "control_logic_bufbuf_16";
pub const SRAM_SP_SENSE_AMP: &str = "sramgen_sp_sense_amp";
pub const CONTROL_LOGIC_INV: &str = "control_logic_inv";

pub const BITCELL_HEIGHT: isize = 1580;
pub const BITCELL_WIDTH: isize = 1200;
pub const TAPCELL_WIDTH: isize = 1300;
pub const COLUMN_WIDTH: isize = BITCELL_WIDTH + TAPCELL_WIDTH;

#[inline]
pub fn custom_gds_dir() -> PathBuf {
    crate::assets::path("tech/sky130/gds")
}

#[inline]
pub fn custom_spice_dir() -> PathBuf {
    crate::assets::path("tech/sky130/spice")
}

/// Uses bundled standard cells for every view and a separate model library for simulation.
pub struct OpenPdk {
    inner: Sky130OpenPdk,
    model_pdk_root: Option<PathBuf>,
}

impl OpenPdk {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Sky130OpenPdk::new(&PdkParams {
                pdk_root: crate::assets::standard_cell_root(),
            })?,
            model_pdk_root: std::env::var_os("SKY130_OPEN_PDK_ROOT").map(PathBuf::from),
        })
    }

    fn model_library(&self) -> Result<PathBuf> {
        let root = self
            .model_pdk_root
            .as_ref()
            .context("Set SKY130_OPEN_PDK_ROOT to the open SKY130 PDK root to run simulations")?;
        let path = root.join("libraries/sky130_fd_pr/latest/models/sky130.lib.spice");
        if !path.is_file() {
            bail!(
                "SKY130_OPEN_PDK_ROOT is missing the model library: {}",
                path.display()
            );
        }
        path.canonicalize()
            .with_context(|| format!("cannot open SKY130 model library {}", path.display()))
    }
}

impl Pdk for OpenPdk {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn process(&self) -> &'static str {
        self.inner.process()
    }

    fn lengths(&self) -> Units {
        self.inner.lengths()
    }

    fn voltages(&self) -> SiPrefix {
        self.inner.voltages()
    }

    fn layers(&self) -> Layers {
        self.inner.layers()
    }

    fn supplies(&self) -> Supplies {
        self.inner.supplies()
    }

    fn mos_devices(&self) -> Vec<MosSpec> {
        self.inner.mos_devices()
    }

    fn mos_schematic(
        &self,
        ctx: &mut SchematicCtx,
        params: &MosParams,
    ) -> substrate::error::Result<()> {
        self.inner.mos_schematic(ctx, params)
    }

    fn mos_layout(
        &self,
        ctx: &mut LayoutCtx,
        params: &LayoutMosParams,
    ) -> substrate::error::Result<()> {
        self.inner.mos_layout(ctx, params)
    }

    fn via_layout(&self, ctx: &mut LayoutCtx, params: &ViaParams) -> substrate::error::Result<()> {
        self.inner.via_layout(ctx, params)
    }

    fn layout_grid(&self) -> i64 {
        self.inner.layout_grid()
    }

    fn includes(&self, purpose: NetlistPurpose) -> substrate::error::Result<IncludeBundle> {
        if let NetlistPurpose::Simulation { corner } = purpose {
            Ok(IncludeBundle {
                lib_includes: vec![(self.model_library()?, corner.name().clone())],
                ..Default::default()
            })
        } else {
            Ok(IncludeBundle::default())
        }
    }

    fn standard_cells(&self) -> substrate::error::Result<StdCellDb> {
        self.inner.standard_cells()
    }

    fn corners(&self) -> substrate::error::Result<CornerDb> {
        self.inner.corners()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_models_are_required_only_for_simulation() {
        let mut pdk = OpenPdk::new().unwrap();
        pdk.model_pdk_root = None;
        assert!(pdk.standard_cells().is_ok());
        let includes = pdk.includes(NetlistPurpose::Library).unwrap();
        assert!(includes.includes.is_empty() && includes.lib_includes.is_empty());

        let corners = pdk.corners().unwrap();
        let corner = corners.try_corner_named("ss").unwrap().clone();
        let simulation = || NetlistPurpose::Simulation {
            corner: corner.clone(),
        };
        assert!(pdk
            .includes(simulation())
            .unwrap_err()
            .to_string()
            .contains("SKY130_OPEN_PDK_ROOT"));

        let directory = tempfile::tempdir().unwrap();
        let models = directory
            .path()
            .join("libraries/sky130_fd_pr/latest/models/sky130.lib.spice");
        pdk.model_pdk_root = Some(directory.path().to_path_buf());
        assert!(pdk.includes(NetlistPurpose::Library).is_ok());
        assert!(pdk.includes(simulation()).is_err());
        std::fs::create_dir_all(models.parent().unwrap()).unwrap();
        std::fs::write(&models, ".lib ss\n.endl\n").unwrap();
        let includes = pdk.includes(simulation()).unwrap();
        assert_eq!(
            includes.lib_includes,
            vec![(models.canonicalize().unwrap(), arcstr::literal!("ss"))]
        );
    }
}
