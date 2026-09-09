use serde::{Deserialize, Serialize};
use subgeom::snap_to_grid;
use substrate1::component::Component;

pub mod layout;
pub mod schematic;

#[derive(Hash, PartialEq, Eq)]
pub struct Precharge {
    params: PrechargeParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PrechargeParams {
    pub length: i64,
    pub pull_up_width: i64,
    pub equalizer_width: i64,
    pub en_b_width: i64,
}

impl PrechargeParams {
    pub fn scale(&self, scale: f64) -> Self {
        let pull_up_width = snap_to_grid(
            i64::max((self.pull_up_width as f64 * scale).round() as i64, 800),
            50,
        );
        let equalizer_width = snap_to_grid(
            i64::max((self.equalizer_width as f64 * scale).round() as i64, 500),
            50,
        );
        Self {
            length: self.length,
            pull_up_width,
            equalizer_width,
            en_b_width: self.en_b_width,
        }
    }
}

impl Component for Precharge {
    type Params = PrechargeParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("precharge")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for Precharge {
    type Params = PrechargeParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self { params: *params })
    }
}
impl substrate::block::Block for Precharge {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("precharge")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("en_b", 1, crate::schematic::Direction::Input),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("bl", 1, crate::schematic::Direction::InOut),
            ("br", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for Precharge {
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
crate::impl_sky130_build!(Precharge);

#[cfg(test)]
mod tests {

    use crate::blocks::columns::PRECHARGE_PARAMS;
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::layout::{PrechargeCent, PrechargeEnd, PrechargeEndParams};
    use super::*;

    #[test]
    fn test_precharge() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_precharge");

        let params = PRECHARGE_PARAMS;
        crate::layout_ctx()
            .write_layout::<Precharge>(&params, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        crate::netlist::write_schematic::<Precharge>(
            &ctx,
            &params,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");
    }

    #[test]
    fn test_precharge_cent() {
        let work_dir = test_work_dir("test_precharge_cent");
        crate::layout_ctx()
            .write_layout::<PrechargeCent>(&PRECHARGE_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_precharge_end() {
        let work_dir = test_work_dir("test_precharge_end");
        crate::layout_ctx()
            .write_layout::<PrechargeEnd>(
                &PrechargeEndParams {
                    via_top: false,
                    inner: PRECHARGE_PARAMS,
                },
                out_gds(work_dir, "layout"),
            )
            .expect("failed to write layout");
    }
}
