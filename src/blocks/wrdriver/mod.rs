use serde::{Deserialize, Serialize};
use subgeom::snap_to_grid;
use substrate1::component::Component;

pub mod layout;
pub mod schematic;

#[derive(Hash, PartialEq, Eq)]
pub struct WriteDriver {
    params: WriteDriverParams,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct WriteDriverParams {
    pub length: i64,
    pub pwidth_driver: i64,
    pub nwidth_driver: i64,
}

impl WriteDriverParams {
    pub fn scale(&self, scale: f64) -> Self {
        let pwidth_driver = snap_to_grid((self.pwidth_driver as f64 * scale).round() as i64, 50);
        let nwidth_driver = snap_to_grid((self.nwidth_driver as f64 * scale).round() as i64, 50);
        Self {
            length: self.length,
            pwidth_driver,
            nwidth_driver,
        }
    }
}

impl Component for WriteDriver {
    type Params = WriteDriverParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_driver")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for WriteDriver {
    type Params = WriteDriverParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
}
impl substrate::block::Block for WriteDriver {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("write_driver")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("en", 1, crate::schematic::Direction::Input),
            ("en_b", 1, crate::schematic::Direction::Input),
            ("data", 1, crate::schematic::Direction::Input),
            ("data_b", 1, crate::schematic::Direction::Input),
            ("bl", 1, crate::schematic::Direction::InOut),
            ("br", 1, crate::schematic::Direction::InOut),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for WriteDriver {
    type Schema = sky130::Sky130;
    type NestedData = ();
    fn schematic(
        &self,
        io: &substrate::types::schematic::IoNodeBundle<Self>,
        cell: &mut substrate::schematic::CellBuilder<Self::Schema>,
    ) -> substrate::error::Result<()> {
        let mut ctx = crate::schematic::CircuitBuilder::new(
            &<Self as substrate::block::Block>::io(self),
            io,
            cell,
        );
        self.build_schematic(&mut ctx)
            .map_err(|e| substrate::error::Error::Anyhow(std::sync::Arc::new(e)))
    }
}
crate::impl_sky130_build!(WriteDriver);

#[cfg(test)]
mod tests {

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    const WRITE_DRIVER_PARAMS: WriteDriverParams = WriteDriverParams {
        length: 150,
        pwidth_driver: 2_000,
        nwidth_driver: 2_000,
    };

    #[test]
    fn test_write_driver() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_write_driver");
        crate::netlist::write_schematic::<WriteDriver>(
            &ctx,
            &WRITE_DRIVER_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");
        crate::layout_ctx()
            .write_layout::<WriteDriver>(&WRITE_DRIVER_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
    }
}
