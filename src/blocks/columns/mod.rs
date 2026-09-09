//! Column peripheral circuitry.

use subgeom::Dir;
use substrate1::component::Component;
use substrate1::layout::context::LayoutCtx;

use super::decoder::{DecoderPhysicalDesignParams, DecoderStageParams, DecoderStyle, RoutingStyle};
use super::gate::sizing::InverterGateTreeNode;
use super::gate::{GateParams, PrimitiveGateParams, PrimitiveGateType};
use super::precharge::PrechargeParams;
use super::sram::schematic::buffer_chain_num_stages;
use super::tgatemux::TGateMuxParams;
use super::wrdriver::WriteDriverParams;
use crate::blocks::latch::DiffLatchParams;
use crate::script::{DesignContext, Script};
use serde::{Deserialize, Serialize};
use subgeom::Span;
use substrate1::layout::layers::selector::Selector;
use substrate1::layout::layers::LayerKey;
use substrate1::layout::routing::tracks::{Boundary, CenteredTrackParams, FixedTracks};

pub mod layout;
pub mod schematic;

#[derive(Debug, Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct ColParams {
    pub pc: PrechargeParams,
    pub mux: TGateMuxParams,
    pub wrdriver: WriteDriverParams,
    pub latch: DiffLatchParams,
    pub cols: usize,
    pub include_wmask: bool,
    pub wmask_granularity: usize,
}

impl ColParams {
    pub const fn mux_ratio(&self) -> usize {
        self.mux.mux_ratio
    }

    pub const fn word_length(&self) -> usize {
        self.cols / self.mux_ratio()
    }

    pub const fn wmask_bits(&self) -> usize {
        self.word_length() / self.wmask_granularity
    }
}

#[derive(Hash, PartialEq, Eq)]
pub struct ColPeripherals {
    pub(crate) params: ColParams,
}

pub struct WmaskPeripherals {
    params: ColParams,
}

#[derive(Hash, PartialEq, Eq)]
pub struct Column {
    params: ColParams,
}

impl Component for ColPeripherals {
    type Params = ColParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("col_peripherals")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for ColPeripherals {
    type Params = ColParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
}
impl substrate::block::Block for ColPeripherals {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("col_peripherals")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("clk", 1, crate::schematic::Direction::Input),
            ("rstb", 1, crate::schematic::Direction::Input),
            ("sense_en", 1, crate::schematic::Direction::Input),
            ("pc_b", 1, crate::schematic::Direction::Input),
            ("we", 1, crate::schematic::Direction::Input),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
            ("bl", self.params.cols, crate::schematic::Direction::InOut),
            ("br", self.params.cols, crate::schematic::Direction::InOut),
            (
                "sel",
                self.params.mux_ratio(),
                crate::schematic::Direction::Input,
            ),
            (
                "sel_b",
                self.params.mux_ratio(),
                crate::schematic::Direction::Input,
            ),
            (
                "wmask",
                self.params.wmask_bits(),
                crate::schematic::Direction::Input,
            ),
            (
                "din",
                self.params.word_length(),
                crate::schematic::Direction::Input,
            ),
            (
                "dout",
                self.params.word_length(),
                crate::schematic::Direction::Output,
            ),
        ])
    }
}
impl substrate::schematic::Schematic for ColPeripherals {
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
crate::impl_sky130_build!(ColPeripherals);

impl Component for WmaskPeripherals {
    type Params = ColParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("peripherals_wmask")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl Component for Column {
    type Params = ColParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("column")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for Column {
    type Params = ColParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
}
impl substrate::block::Block for Column {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("column")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("clk", 1, crate::schematic::Direction::Input),
            ("rstb", 1, crate::schematic::Direction::Input),
            ("pc_b", 1, crate::schematic::Direction::Input),
            ("we", 1, crate::schematic::Direction::Input),
            ("we_b", 1, crate::schematic::Direction::Input),
            ("din", 1, crate::schematic::Direction::Input),
            ("sense_en", 1, crate::schematic::Direction::Input),
            ("dout", 1, crate::schematic::Direction::Output),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
            (
                "bl",
                self.params.mux_ratio(),
                crate::schematic::Direction::InOut,
            ),
            (
                "br",
                self.params.mux_ratio(),
                crate::schematic::Direction::InOut,
            ),
            (
                "sel",
                self.params.mux_ratio(),
                crate::schematic::Direction::Input,
            ),
            (
                "sel_b",
                self.params.mux_ratio(),
                crate::schematic::Direction::Input,
            ),
        ])
    }
}
impl substrate::schematic::Schematic for Column {
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
crate::impl_sky130_build!(Column);

pub struct ColumnsPhysicalDesignScript;

pub struct ColumnsPhysicalDesign {
    pub cl_max: f64,
    pub wmask_unit_width: i64,
    pub nand: DecoderStageParams,
}

impl Script for ColumnsPhysicalDesignScript {
    type Params = ColParams;
    type Output = ColumnsPhysicalDesign;

    fn run(
        params: &Self::Params,
        ctx: &substrate::context::Context,
    ) -> anyhow::Result<Self::Output> {
        let pc_design = ctx.run_script::<ColumnDesignScript>(&crate::schematic::NoParams)?;
        let wmask_unit_width = params.wmask_granularity as i64
            * (pc_design.width * params.mux_ratio() as i64 + pc_design.tap_width);
        let we_i_cap = params.wmask_granularity as f64
            * COL_CAPACITANCES.we_i
            * (params.wrdriver.pwidth_driver as f64 / COL_PARAMS.wrdriver.pwidth_driver as f64);
        let we_ib_cap = params.wmask_granularity as f64
            * COL_CAPACITANCES.we_ib
            * (params.wrdriver.pwidth_driver as f64 / COL_PARAMS.wrdriver.pwidth_driver as f64);
        let cl_max = f64::max(we_i_cap, we_ib_cap);
        let wmask_buffer_stages = buffer_chain_num_stages(cl_max);
        let mut wmask_buffer_gates = InverterGateTreeNode {
            gate: PrimitiveGateType::Nand2,
            id: 1,
            n_invs: wmask_buffer_stages,
            n_branching: 1,
            children: vec![],
        }
        .elaborate()
        .size(cl_max)
        .as_chain();
        wmask_buffer_gates.push(*wmask_buffer_gates.last().unwrap());

        Ok(ColumnsPhysicalDesign {
            cl_max,
            wmask_unit_width,
            nand: DecoderStageParams {
                pd: DecoderPhysicalDesignParams {
                    style: DecoderStyle::Minimum,
                    dir: Dir::Horiz,
                },
                routing_style: RoutingStyle::Decoder,
                // 0.38 min spacing between nsdm/psdm, alternatively can merge nsdm/psdm.
                max_width: Some(wmask_unit_width - 380),
                gate: GateParams::Nand2(wmask_buffer_gates[0]),
                invs: wmask_buffer_gates.iter().copied().skip(1).collect(),
                num: 1,
                use_multi_finger_invs: false,
                dont_connect_outputs: false,
                child_sizes: vec![1, 1],
            },
        })
    }
}

pub const WRITE_DRIVER_PARAMS: WriteDriverParams = WriteDriverParams {
    length: 150,
    pwidth_driver: 3_000,
    nwidth_driver: 3_000,
};
pub const MUX_PARAMS: TGateMuxParams = TGateMuxParams {
    length: 150,
    pwidth: 3_600,
    nwidth: 2_400,
    mux_ratio: 4,
    idx: 2,
    sel_width: 360,
};
pub const PRECHARGE_PARAMS: PrechargeParams = PrechargeParams {
    length: 150,
    pull_up_width: 2_000,
    equalizer_width: 1_200,
    en_b_width: 360,
};

pub const DIFF_LATCH_PARAMS: DiffLatchParams = DiffLatchParams {
    inv_in: PrimitiveGateParams {
        nwidth: 1_200,
        pwidth: 2_000,
        length: 150,
    },
    lch: 150,
    inv_out: PrimitiveGateParams {
        nwidth: 1_200,
        pwidth: 2_000,
        length: 150,
    },
    invq: PrimitiveGateParams {
        nwidth: 1_200,
        pwidth: 2_000,
        length: 150,
    },
    nwidth: 2_000,
};

pub const COL_WMASK_PARAMS: ColParams = ColParams {
    pc: PRECHARGE_PARAMS,
    wrdriver: WRITE_DRIVER_PARAMS,
    mux: MUX_PARAMS,
    latch: DIFF_LATCH_PARAMS,
    cols: 16,
    include_wmask: true,
    wmask_granularity: 2,
};

pub const COL_PARAMS: ColParams = ColParams {
    pc: PRECHARGE_PARAMS,
    wrdriver: WRITE_DRIVER_PARAMS,
    mux: MUX_PARAMS,
    latch: DIFF_LATCH_PARAMS,
    cols: 128,
    include_wmask: false,
    wmask_granularity: 8,
};

pub const COL_CAPACITANCES: ColCapacitances = ColCapacitances {
    pc_b: 591.432e-15 / (COL_PARAMS.cols + 2) as f64,
    saen: 393.347e-15 / (COL_PARAMS.cols / COL_PARAMS.mux.mux_ratio) as f64,
    sel: 186.458e-15 / (COL_PARAMS.cols / COL_PARAMS.mux.mux_ratio) as f64,
    sel_b: 198.964e-15 / (COL_PARAMS.cols / COL_PARAMS.mux.mux_ratio) as f64,
    we: 36.462e-15 / COL_PARAMS.wmask_bits() as f64,
    we_i: 11.3990e-15,
    we_ib: 12.0547e-15,
};

pub struct ColCapacitances {
    pub saen: f64,
    pub pc_b: f64,
    pub sel: f64,
    pub sel_b: f64,
    pub we: f64,
    pub we_i: f64,
    pub we_ib: f64,
}

pub struct ColumnDesignScript;

pub struct ColumnPhysicalDesign {
    pub(crate) h_metal: LayerKey,
    pub(crate) width: i64,
    pub(crate) in_tracks: FixedTracks,
    pub(crate) out_tracks: FixedTracks,
    pub(crate) v_metal: LayerKey,
    pub(crate) v_line: i64,
    pub(crate) v_space: i64,
    pub(crate) m0: LayerKey,
    pub(crate) grid: i64,
    pub(crate) tap_width: i64,
}

impl Script for ColumnDesignScript {
    type Params = crate::schematic::NoParams;
    type Output = ColumnPhysicalDesign;

    fn run(
        _params: &Self::Params,
        _ctx: &substrate::context::Context,
    ) -> anyhow::Result<Self::Output> {
        let layers = crate::layout_ctx().layers();
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let in_tracks = FixedTracks::from_centered_tracks(CenteredTrackParams {
            line: 140,
            space: 230,
            span: Span::new(0, 1_200),
            num: 4,
            lower_boundary: Boundary::HalfTrack,
            upper_boundary: Boundary::HalfTrack,
            grid: 5,
        });
        let out_tracks = FixedTracks::from_centered_tracks(CenteredTrackParams {
            line: 140,
            space: 230,
            span: Span::new(0, 1_200),
            num: 3,
            lower_boundary: Boundary::HalfSpace,
            upper_boundary: Boundary::HalfSpace,
            grid: 5,
        });

        Ok(ColumnPhysicalDesign {
            h_metal: m2,
            width: 1_200,
            v_metal: m1,
            v_line: 140,
            v_space: 140,
            in_tracks,
            out_tracks,
            grid: 5,
            tap_width: 1_300,
            m0,
        })
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "commercial")]
    use crate::verification::calibre::CalibreContext;
    #[cfg(feature = "commercial")]
    use arcstr::ArcStr;

    #[cfg(feature = "commercial")]
    use crate::measure::impedance::AcImpedanceTbNode;
    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;
    #[cfg(feature = "commercial")]
    use std::collections::HashMap;

    use super::layout::{ColCentParams, ColumnCent, TappedColumn};
    use super::*;

    #[test]
    fn test_col_peripherals() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_col_peripherals");
        crate::layout_ctx()
            .write_layout::<ColPeripherals>(&COL_WMASK_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        crate::netlist::write_schematic::<ColPeripherals>(
            &ctx,
            &COL_WMASK_PARAMS,
            out_spice(&work_dir, "netlist"),
        )
        .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let drc_work_dir = work_dir.join("drc");
            let output = ctx
                .write_drc::<ColPeripherals>(&COL_WMASK_PARAMS, drc_work_dir)
                .expect("failed to run DRC");
            assert!(matches!(
                output.summary,
                crate::verification::calibre::DrcSummary::Pass
            ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<ColPeripherals>(&COL_WMASK_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                crate::verification::calibre::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_column_wmask_4() {
        let work_dir = test_work_dir("test_column_wmask_4");
        crate::layout_ctx()
            .write_layout::<Column>(&COL_WMASK_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_column_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_4");
        crate::layout_ctx()
            .write_layout::<Column>(&COL_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        crate::netlist::write_schematic::<Column>(
            &ctx,
            &COL_PARAMS,
            out_spice(work_dir, "schematic"),
        )
        .expect("failed to write layout");
    }

    #[test]
    fn test_tapped_column_4() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tapped_column_4");
        crate::layout_ctx()
            .write_layout::<TappedColumn>(&COL_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        crate::netlist::write_schematic::<TappedColumn>(
            &ctx,
            &COL_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write layout");

        #[cfg(feature = "commercial")]
        {
            // let drc_work_dir = work_dir.join("drc");
            // let output = ctx
            //     .write_drc::<TappedColumn>(&COL_PARAMS, drc_work_dir)
            //     .expect("failed to run DRC");
            // assert!(matches!(
            //     output.summary,
            //     crate::verification::calibre::DrcSummary::Pass
            // ));
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<TappedColumn>(&COL_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                crate::verification::calibre::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_column_cent_4() {
        let work_dir = test_work_dir("test_column_cent_4");
        crate::layout_ctx()
            .write_layout::<ColumnCent>(
                &ColCentParams {
                    col: COL_WMASK_PARAMS,
                    end: false,
                    cut_wmask: false,
                },
                out_gds(work_dir, "layout"),
            )
            .expect("failed to write layout");
    }

    #[test]
    fn test_column_end_4() {
        let work_dir = test_work_dir("test_column_end_4");
        crate::layout_ctx()
            .write_layout::<ColumnCent>(
                &ColCentParams {
                    col: COL_WMASK_PARAMS,
                    end: true,
                    cut_wmask: true,
                },
                out_gds(work_dir, "layout"),
            )
            .expect("failed to write layout");
    }

    #[cfg(feature = "commercial")]
    fn col_peripherals_default_conns() -> HashMap<&'static str, Vec<AcImpedanceTbNode>> {
        let dut = ColPeripherals { params: COL_PARAMS };
        let io = dut.io();
        HashMap::from_iter(io.iter().map(|(&k, &v)| {
            let conn = match k {
                "clk" => AcImpedanceTbNode::Vdd,
                "rstb" => AcImpedanceTbNode::Vdd,
                "vdd" => AcImpedanceTbNode::Vdd,
                "vss" => AcImpedanceTbNode::Vdd,
                "sense_en" => AcImpedanceTbNode::Vss,
                "bl" => AcImpedanceTbNode::Vdd,
                "br" => AcImpedanceTbNode::Vdd,
                "pc_b" => AcImpedanceTbNode::Vdd,
                "sel" => AcImpedanceTbNode::Vss,
                "sel_b" => AcImpedanceTbNode::Vdd,
                "we" => AcImpedanceTbNode::Vss,
                "wmask" => AcImpedanceTbNode::Vdd,
                "din" => AcImpedanceTbNode::Vss,
                "dout" => AcImpedanceTbNode::Vdd,
                x => panic!("unexpected signal {x}"),
            };
            (k, vec![conn; v])
        }))
    }

    #[cfg(feature = "commercial")]
    fn column_default_conns() -> HashMap<&'static str, Vec<AcImpedanceTbNode>> {
        let col = Column { params: COL_PARAMS };
        let io = col.io();
        HashMap::from_iter(io.iter().map(|(&k, &v)| {
            let conn = match k {
                "clk" => AcImpedanceTbNode::Vdd,
                "rstb" => AcImpedanceTbNode::Vdd,
                "vdd" => AcImpedanceTbNode::Vdd,
                "vss" => AcImpedanceTbNode::Vdd,
                "bl" => AcImpedanceTbNode::Vdd,
                "br" => AcImpedanceTbNode::Vdd,
                "pc_b" => AcImpedanceTbNode::Vdd,
                "sel" => AcImpedanceTbNode::Vss,
                "sel_b" => AcImpedanceTbNode::Vdd,
                "we" => AcImpedanceTbNode::Vss,
                "we_b" => AcImpedanceTbNode::Vss,
                "din" => AcImpedanceTbNode::Vss,
                "dout" => AcImpedanceTbNode::Vdd,
                "sense_en" => AcImpedanceTbNode::Vss,
                x => panic!("unexpected signal {x}"),
            };
            (k, vec![conn; v])
        }))
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_columns_cap() {
        use crate::measure::impedance::{
            AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench,
        };

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_columns_cap");
        let params = COL_PARAMS;

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        crate::netlist::write_schematic::<ColPeripherals>(&ctx, &params, &pex_path)
            .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        crate::layout_ctx()
            .write_layout::<ColPeripherals>(&params, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(crate::verification::calibre::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("col_peripherals"),
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("col_peripherals"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        for port in ["clk", "rstb", "sense_en", "pc_b", "sel", "sel_b", "we"] {
            let mut conns = col_peripherals_default_conns();
            conns.get_mut(port).unwrap()[0] = AcImpedanceTbNode::Vmeas;

            let sim_dir = work_dir.join(format!("{port}_cap"));
            let cap_ac = crate::sim::run::<AcImpedanceTestbench<ColPeripherals>>(
                &ctx,
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 10e6,
                    points: 10,
                    dut: params.clone(),
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vdd,
                    connections: HashMap::from_iter(
                        conns.into_iter().map(|(k, v)| (ArcStr::from(k), v)),
                    ),
                },
                &sim_dir,
            )
            .expect("failed to write simulation");

            println!("C{port} = {}fF", 1e15 * cap_ac.max_freq_cap());
        }
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_column_cap() {
        use crate::measure::impedance::{
            AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench,
        };

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_column_cap");
        let params = COL_PARAMS;

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        crate::netlist::write_schematic::<TappedColumn>(&ctx, &params, &pex_path)
            .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        crate::layout_ctx()
            .write_layout::<TappedColumn>(&params, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(crate::verification::calibre::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("tapped_column"),
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("tapped_column"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        for port in [
            "clk", "rstb", "pc_b", "sel", "sel_b", "we", "we_b", "sense_en",
        ] {
            let mut conns = column_default_conns();
            conns.get_mut(port).unwrap()[0] = AcImpedanceTbNode::Vmeas;

            let sim_dir = work_dir.join(format!("{port}_cap"));
            let cap_ac = crate::sim::run::<AcImpedanceTestbench<TappedColumn>>(
                &ctx,
                &AcImpedanceTbParams {
                    vdd: 1.8,
                    fstart: 100.,
                    fstop: 10e6,
                    points: 10,
                    dut: params.clone(),
                    pex_netlist: Some(pex_netlist_path.clone()),
                    vmeas_conn: AcImpedanceTbNode::Vdd,
                    connections: HashMap::from_iter(
                        conns.into_iter().map(|(k, v)| (ArcStr::from(k), v)),
                    ),
                },
                &sim_dir,
            )
            .expect("failed to write simulation");

            println!("C{port} = {}fF", 1e15 * cap_ac.max_freq_cap());
        }
    }
}
