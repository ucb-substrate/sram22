use super::gate::PrimitiveGateParams;
use serde::{Deserialize, Serialize};
use substrate1::component::Component;

pub mod layout;
pub mod schematic;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct DiffLatchParams {
    pub inv_in: PrimitiveGateParams,
    pub invq: PrimitiveGateParams,
    pub inv_out: PrimitiveGateParams,
    pub nwidth: i64,
    pub lch: i64,
}

#[derive(Hash, PartialEq, Eq)]
pub struct DiffLatch {
    params: DiffLatchParams,
}

impl Component for DiffLatch {
    type Params = DiffLatchParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("diff_latch")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for DiffLatch {
    type Params = DiffLatchParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self { params: *params })
    }
}
impl substrate::block::Block for DiffLatch {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("diff_latch")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("din1", 1, crate::schematic::Direction::Input),
            ("din2", 1, crate::schematic::Direction::Input),
            ("dout1", 1, crate::schematic::Direction::Output),
            ("dout2", 1, crate::schematic::Direction::Output),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for DiffLatch {
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
crate::impl_sky130_build!(DiffLatch);

#[cfg(test)]
mod tests {

    use crate::blocks::columns::DIFF_LATCH_PARAMS;
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::layout::DiffLatchCent;
    use super::*;

    #[test]
    fn test_diff_latch() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_diff_latch");
        crate::layout_ctx()
            .write_layout::<DiffLatch>(&DIFF_LATCH_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        crate::netlist::write_schematic::<DiffLatch>(
            &ctx,
            &DIFF_LATCH_PARAMS,
            out_spice(work_dir, "schematic"),
        )
        .expect("failed to write schematic");
    }

    #[test]
    fn test_diff_latch_cent() {
        let work_dir = test_work_dir("test_diff_latch_cent");
        crate::layout_ctx()
            .write_layout::<DiffLatchCent>(&DIFF_LATCH_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
