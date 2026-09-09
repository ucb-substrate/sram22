use arcstr::ArcStr;
use serde::{Deserialize, Serialize};
use substrate1::component::{Component, NoParams};

pub mod layout;
pub mod schematic;
pub mod testbench;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ControlLogicReplicaV2 {
    params: ControlLogicParams,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ControlLogicParams {
    pub decoder_delay_invs: usize,
    pub wlen_pulse_invs: usize,
    pub pc_set_delay_invs: usize,
    pub wrdrven_set_delay_invs: usize,
    pub wrdrven_rst_delay_invs: usize,
}

impl Component for ControlLogicReplicaV2 {
    type Params = ControlLogicParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        assert_eq!(
            params.decoder_delay_invs % 2,
            0,
            "decoder replica delay chain must have an even number of inverters"
        );
        assert_eq!(
            params.wlen_pulse_invs % 2,
            1,
            "wordline pulse delay chain must have an odd number of inverters"
        );
        assert_eq!(
            params.pc_set_delay_invs % 2,
            0,
            "pc set delay chain must have an even number of inverters"
        );
        assert_eq!(
            params.wrdrven_set_delay_invs % 2,
            0,
            "write drive enable set delay chain must have an even number of inverters"
        );
        assert_eq!(
            params.wrdrven_rst_delay_invs % 2,
            0,
            "write drive enable rst delay chain must have an even number of inverters"
        );
        Ok(Self { params: *params })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("control_logic_replica_v2")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for ControlLogicReplicaV2 {
    type Params = ControlLogicParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        assert_eq!(
            params.decoder_delay_invs % 2,
            0,
            "decoder replica delay chain must have an even number of inverters"
        );
        assert_eq!(
            params.wlen_pulse_invs % 2,
            1,
            "wordline pulse delay chain must have an odd number of inverters"
        );
        assert_eq!(
            params.pc_set_delay_invs % 2,
            0,
            "pc set delay chain must have an even number of inverters"
        );
        assert_eq!(
            params.wrdrven_set_delay_invs % 2,
            0,
            "write drive enable set delay chain must have an even number of inverters"
        );
        assert_eq!(
            params.wrdrven_rst_delay_invs % 2,
            0,
            "write drive enable rst delay chain must have an even number of inverters"
        );
        Ok(Self { params: *params })
    }
}
impl substrate::block::Block for ControlLogicReplicaV2 {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("control_logic_replica_v2")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("clk", 1, crate::schematic::Direction::Input),
            ("ce", 1, crate::schematic::Direction::Input),
            ("we", 1, crate::schematic::Direction::Input),
            ("rstb", 1, crate::schematic::Direction::Input),
            ("rbl", 1, crate::schematic::Direction::Input),
            ("saen", 1, crate::schematic::Direction::Output),
            ("pc_b", 1, crate::schematic::Direction::Output),
            ("rwl", 1, crate::schematic::Direction::Output),
            ("wlen", 1, crate::schematic::Direction::Output),
            ("wrdrven", 1, crate::schematic::Direction::Output),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for ControlLogicReplicaV2 {
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
crate::impl_sky130_build!(ControlLogicReplicaV2);

#[derive(Hash, PartialEq, Eq)]
pub struct SrLatch;

impl Component for SrLatch {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sr_latch")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for SrLatch {
    type Params = crate::schematic::NoParams;
    fn from_params(_params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
impl substrate::block::Block for SrLatch {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sr_latch")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("sb", 1, crate::schematic::Direction::Input),
            ("rb", 1, crate::schematic::Direction::Input),
            ("q", 1, crate::schematic::Direction::Output),
            ("qb", 1, crate::schematic::Direction::Output),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for SrLatch {
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
crate::impl_sky130_build!(SrLatch);

#[derive(Hash, PartialEq, Eq)]
pub struct InvChain {
    n: usize,
}

impl Component for InvChain {
    type Params = usize;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        let n = *params;
        assert!(n >= 1, "inverter chain must have at least one inverter");
        Ok(Self { n })
    }
    fn name(&self) -> arcstr::ArcStr {
        ArcStr::from(format!("inv_chain_{}", self.n))
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for InvChain {
    type Params = usize;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        let n = *params;
        assert!(n >= 1, "inverter chain must have at least one inverter");
        Ok(Self { n })
    }
}
impl substrate::block::Block for InvChain {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        ArcStr::from(format!("inv_chain_{}", self.n))
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("din", 1, crate::schematic::Direction::Input),
            ("dout", 1, crate::schematic::Direction::Output),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for InvChain {
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
crate::impl_sky130_build!(InvChain);

#[derive(Hash, PartialEq, Eq)]
pub struct SvtInvChain {
    n: usize,
}

impl Component for SvtInvChain {
    type Params = usize;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        let n = *params;
        assert!(n >= 1, "inverter chain must have at least one inverter");
        Ok(Self { n })
    }
    fn name(&self) -> arcstr::ArcStr {
        ArcStr::from(format!("svt_inv_chain_{}", self.n))
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for SvtInvChain {
    type Params = usize;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        let n = *params;
        assert!(n >= 1, "inverter chain must have at least one inverter");
        Ok(Self { n })
    }
}
impl substrate::block::Block for SvtInvChain {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        ArcStr::from(format!("svt_inv_chain_{}", self.n))
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("din", 1, crate::schematic::Direction::Input),
            ("dout", 1, crate::schematic::Direction::Output),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for SvtInvChain {
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
crate::impl_sky130_build!(SvtInvChain);

#[derive(Hash, PartialEq, Eq)]
pub struct EdgeDetector {
    invs: usize,
}

impl Component for EdgeDetector {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self { invs: 9 })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("edge_detector")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for EdgeDetector {
    type Params = crate::schematic::NoParams;
    fn from_params(_params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self { invs: 9 })
    }
}
impl substrate::block::Block for EdgeDetector {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("edge_detector")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("din", 1, crate::schematic::Direction::Input),
            ("dout", 1, crate::schematic::Direction::Output),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for EdgeDetector {
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
crate::impl_sky130_build!(EdgeDetector);

#[cfg(test)]
pub mod test {
    #[cfg(feature = "commercial")]
    use crate::verification::calibre::CalibreContext;
    use substrate1::component::NoParams;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::{ControlLogicParams, ControlLogicReplicaV2, EdgeDetector, SrLatch};

    const CONTROL_LOGIC_PARAMS: ControlLogicParams = ControlLogicParams {
        decoder_delay_invs: 12,
        wlen_pulse_invs: 11,
        pc_set_delay_invs: 14,
        wrdrven_set_delay_invs: 4,
        wrdrven_rst_delay_invs: 0,
    };

    #[test]
    fn test_control_logic_replica_v2() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_control_logic_replica_v2");

        crate::netlist::write_schematic::<ControlLogicReplicaV2>(
            &ctx,
            &CONTROL_LOGIC_PARAMS,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        crate::layout_ctx()
            .write_layout::<ControlLogicReplicaV2>(
                &CONTROL_LOGIC_PARAMS,
                out_gds(&work_dir, "layout"),
            )
            .expect("failed to write layout");

        #[cfg(feature = "commercial")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<ControlLogicReplicaV2>(&CONTROL_LOGIC_PARAMS, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                crate::verification::calibre::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<ControlLogicReplicaV2>(&CONTROL_LOGIC_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                crate::verification::calibre::LvsSummary::Pass
            ));
        }
    }

    #[cfg(feature = "commercial")]
    #[test]
    fn test_control_logic_replica_v2_tb() {
        use crate::blocks::control::testbench::ControlLogicTestbench;

        use super::testbench::tb_params;

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_control_logic_replica_v2_tb");

        crate::sim::run_with_corner::<ControlLogicTestbench>(
            &ctx,
            &tb_params(1.8),
            &work_dir,
            sky130::corner::Sky130Corner::Tt,
        )
        .expect("failed to run simulation");
    }

    #[test]
    fn test_sr_latch() {
        let work_dir = test_work_dir("test_sr_latch");

        crate::layout_ctx()
            .write_layout::<SrLatch>(&NoParams, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_edge_detector() {
        let work_dir = test_work_dir("test_edge_detector");

        crate::layout_ctx()
            .write_layout::<EdgeDetector>(&NoParams, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }
}
