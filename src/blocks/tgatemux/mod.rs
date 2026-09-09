use serde::{Deserialize, Serialize};
use subgeom::snap_to_grid;
use substrate1::component::Component;
use substrate1::layout::cell::{CellPort, PortConflictStrategy};
use substrate1::layout::placement::align::AlignMode;
use substrate1::layout::placement::array::ArrayTiler;

mod layout;
mod schematic;

#[derive(Hash, PartialEq, Eq)]
pub struct TGateMux {
    params: TGateMuxParams,
}

/// [`TGateMux`] taps.
#[derive(Hash, PartialEq, Eq)]
pub struct TGateMuxCent {
    params: TGateMuxParams,
}

/// [`TGateMux`] end cap.
#[derive(Hash, PartialEq, Eq)]
pub struct TGateMuxEnd {
    params: TGateMuxParams,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct TGateMuxParams {
    pub length: i64,
    pub pwidth: i64,
    pub nwidth: i64,
    pub mux_ratio: usize,
    pub idx: usize,
    pub sel_width: i64,
}

impl TGateMuxParams {
    pub fn scale(&self, scale: f64) -> Self {
        let pwidth = snap_to_grid((self.pwidth as f64 * scale).round() as i64, 50);
        let nwidth = snap_to_grid((self.nwidth as f64 * scale).round() as i64, 50);
        Self {
            length: self.length,
            pwidth,
            nwidth,
            mux_ratio: self.mux_ratio,
            idx: self.idx,
            sel_width: self.sel_width,
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
pub struct TappedTGateMux {
    pub params: TGateMuxParams,
}

pub struct TGateMuxGroup {
    pub params: TGateMuxParams,
}

impl Component for TGateMux {
    type Params = TGateMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tgate_mux")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for TGateMux {
    type Params = TGateMuxParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
}
impl substrate::block::Block for TGateMux {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tgate_mux")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("sel_b", 1, crate::schematic::Direction::Input),
            ("sel", 1, crate::schematic::Direction::Input),
            ("bl", 1, crate::schematic::Direction::InOut),
            ("br", 1, crate::schematic::Direction::InOut),
            ("bl_out", 1, crate::schematic::Direction::InOut),
            ("br_out", 1, crate::schematic::Direction::InOut),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for TGateMux {
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
crate::impl_sky130_build!(TGateMux);

impl Component for TGateMuxCent {
    type Params = TGateMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tgate_mux_cent")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for TGateMuxCent {
    type Params = TGateMuxParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
}
impl substrate::block::Block for TGateMuxCent {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tgate_mux_cent")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::default()
    }
}
impl substrate::schematic::Schematic for TGateMuxCent {
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
impl TGateMuxCent {
    fn build_schematic(&self, _ctx: &mut crate::schematic::CircuitBuilder) -> anyhow::Result<()> {
        Ok(())
    }
}
crate::impl_sky130_build!(TGateMuxCent);

impl Component for TGateMuxEnd {
    type Params = TGateMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tgate_mux_end")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

impl crate::schematic::FromParams for TGateMuxEnd {
    type Params = TGateMuxParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
}
impl substrate::block::Block for TGateMuxEnd {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tgate_mux_end")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::default()
    }
}
impl substrate::schematic::Schematic for TGateMuxEnd {
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
impl TGateMuxEnd {
    fn build_schematic(&self, _ctx: &mut crate::schematic::CircuitBuilder) -> anyhow::Result<()> {
        Ok(())
    }
}
crate::impl_sky130_build!(TGateMuxEnd);

impl Component for TappedTGateMux {
    type Params = TGateMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(TappedTGateMux {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_tgate_mux")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        let params = TGateMuxParams {
            idx: 0,
            ..self.params
        };
        let tap = ctx.instantiate::<TGateMuxEnd>(&params)?;
        let mut tap_flipped = ctx.instantiate::<TGateMuxEnd>(&params)?;
        tap_flipped.reflect_horiz_anchored();
        let mut tiler = ArrayTiler::builder();
        tiler.push(tap);

        for i in 0..4 {
            let params = TGateMuxParams {
                idx: i,
                ..self.params
            };
            let mut gate = ctx.instantiate::<TGateMux>(&params)?;
            if i % 2 != 0 {
                gate.reflect_horiz_anchored();
            }
            tiler.push(gate);
        }
        let mut tiler = tiler
            .push(tap_flipped)
            .mode(AlignMode::ToTheRight)
            .alt_mode(AlignMode::Bottom)
            .build();
        tiler.expose_ports(
            |port: CellPort, i| match port.name().as_str() {
                "sel" | "sel_b" => {
                    if port.id().index() == 0 {
                        let name = port.name().clone();
                        Some(port.with_id(name))
                    } else {
                        None
                    }
                }
                "bl_out" => Some(port.with_id("bl_out")),
                "br_out" => Some(port.with_id("br_out")),
                "bl" | "br" => {
                    if i == 1 {
                        Some(port)
                    } else {
                        None
                    }
                }
                _ => Some(port),
            },
            PortConflictStrategy::Merge,
        )?;
        ctx.add_ports(tiler.ports().cloned()).unwrap();

        ctx.draw_ref(&tiler)?;

        Ok(())
    }
}

impl crate::schematic::FromParams for TappedTGateMux {
    type Params = TGateMuxParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(TappedTGateMux {
            params: params.clone(),
        })
    }
}
impl substrate::block::Block for TappedTGateMux {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_tgate_mux")
    }
    fn io(&self) -> Self::Io {
        <TGateMux as substrate::block::Block>::io(
            &<TGateMux as crate::schematic::FromParams>::from_params(&self.params)
                .expect("invalid parameters"),
        )
    }
}
impl substrate::schematic::Schematic for TappedTGateMux {
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
impl TappedTGateMux {
    fn build_schematic(&self, ctx: &mut crate::schematic::CircuitBuilder) -> anyhow::Result<()> {
        let mut gate = ctx.instantiate::<TGateMux>(&self.params)?;
        ctx.bubble_all_ports(&mut gate);
        ctx.add_instance(gate);
        Ok(())
    }
}
crate::impl_sky130_build!(TappedTGateMux);

impl Component for TGateMuxGroup {
    type Params = TGateMuxParams;
    fn new(
        params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(TGateMuxGroup {
            params: params.clone(),
        })
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tgate_mux_group")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        self.layout(ctx)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "commercial")]
    use crate::verification::calibre::CalibreContext;

    use crate::paths::{out_gds, out_spice};
    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    const TGATE_MUX_PARAMS: TGateMuxParams = TGateMuxParams {
        length: 150,
        pwidth: 3_200,
        nwidth: 1_600,
        mux_ratio: 4,
        idx: 2,
        sel_width: 320,
    };

    #[test]
    fn test_tgate_mux() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tgate_mux");
        crate::layout_ctx()
            .write_layout::<TGateMux>(&TGATE_MUX_PARAMS, out_gds(&work_dir, "layout"))
            .expect("failed to write layout");
        crate::netlist::write_schematic::<TGateMux>(
            &ctx,
            &TGATE_MUX_PARAMS,
            out_spice(&work_dir, "schematic"),
        )
        .expect("failed to write schematic");

        #[cfg(feature = "commercial")]
        {
            let lvs_work_dir = work_dir.join("lvs");
            let output = ctx
                .write_lvs::<TappedTGateMux>(&TGATE_MUX_PARAMS, lvs_work_dir)
                .expect("failed to run LVS");
            assert!(matches!(
                output.summary,
                crate::verification::calibre::LvsSummary::Pass
            ));
        }
    }

    #[test]
    fn test_tgate_mux_cent() {
        let work_dir = test_work_dir("test_tgate_mux_cent");
        crate::layout_ctx()
            .write_layout::<TGateMuxCent>(&TGATE_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_tgate_mux_end() {
        let work_dir = test_work_dir("test_tgate_mux_end");
        crate::layout_ctx()
            .write_layout::<TGateMuxEnd>(&TGATE_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    fn test_tgate_mux_group() {
        let work_dir = test_work_dir("test_tgate_mux_group");
        crate::layout_ctx()
            .write_layout::<TGateMuxGroup>(&TGATE_MUX_PARAMS, out_gds(work_dir, "layout"))
            .expect("failed to write layout");
    }

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_tgate_mux_cap() {
        use std::collections::HashMap;

        use crate::measure::impedance::{
            AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench,
        };

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_tgate_mux_cap");

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        crate::netlist::write_schematic::<TappedTGateMux>(&ctx, &TGATE_MUX_PARAMS, &pex_path)
            .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        crate::layout_ctx()
            .write_layout::<TappedTGateMux>(&TGATE_MUX_PARAMS, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(crate::verification::calibre::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("tapped_tgate_mux"),
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("tapped_tgate_mux"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let selb_work_dir = work_dir.join("selb_sim");
        let cap_selb = crate::sim::run::<AcImpedanceTestbench<TappedTGateMux>>(
            &ctx,
            &AcImpedanceTbParams {
                fstart: 100.,
                fstop: 100e6,
                points: 10,
                vdd: 1.8,
                dut: TGATE_MUX_PARAMS,
                pex_netlist: Some(pex_netlist_path.clone()),
                vmeas_conn: AcImpedanceTbNode::Vss,
                connections: HashMap::from_iter([
                    (arcstr::literal!("sel"), vec![AcImpedanceTbNode::Vdd]),
                    (arcstr::literal!("sel_b"), vec![AcImpedanceTbNode::Vmeas]),
                    (arcstr::literal!("bl"), vec![AcImpedanceTbNode::Vdd]),
                    (arcstr::literal!("br"), vec![AcImpedanceTbNode::Vdd]),
                    (
                        arcstr::literal!("bl_out"),
                        vec![AcImpedanceTbNode::Floating],
                    ),
                    (
                        arcstr::literal!("br_out"),
                        vec![AcImpedanceTbNode::Floating],
                    ),
                    (arcstr::literal!("vdd"), vec![AcImpedanceTbNode::Vdd]),
                    (arcstr::literal!("vss"), vec![AcImpedanceTbNode::Vss]),
                ]),
            },
            &selb_work_dir,
        )
        .expect("failed to write simulation");

        println!("Cselb = {}", cap_selb.max_freq_cap(),);
    }
}
