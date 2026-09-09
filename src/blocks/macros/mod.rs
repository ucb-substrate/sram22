#[cfg(all(test, feature = "commercial"))]
use crate::verification::calibre::CalibreContext;
use std::path::PathBuf;

use crate::blocks::columns::ColumnDesignScript;
use crate::tech::{external_gds_path, external_spice_path};
use subgeom::bbox::BoundBox;
use subgeom::{Rect, Span};
use substrate1::component::{Component, NoParams};
use substrate1::data::SubstrateCtx;
use substrate1::layout::cell::{CellPort, Port};
use substrate1::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate1::layout::layers::selector::Selector;
use substrate1::layout::layers::LayerBoundBox;

pub mod schematic;

/// Path to the GDS file backing a layout hard macro.
pub(crate) fn hard_macro_gds_path(name: &str) -> PathBuf {
    external_gds_path().join(format!("{name}.gds"))
}

/// Path to the SPICE file backing a schematic hard macro.
pub(crate) fn hard_macro_spice_path(name: &str) -> PathBuf {
    external_spice_path().join(format!("{name}.spice"))
}

/// Declares a Substrate 1 layout-only hard macro backed by a GDS file in `tech/sky130/gds`.
///
/// Schematic views of hard macros are implemented with Substrate 2 in [`schematic`].
macro_rules! layout_hard_macro {
    ($(#[$meta:meta])* $ident:ident, name = $name:literal, gds_cell_name = $gds:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
        pub struct $ident;

        impl Component for $ident {
            type Params = NoParams;

            fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate1::error::Result<Self> {
                Ok(Self)
            }

            fn name(&self) -> arcstr::ArcStr {
                arcstr::literal!($name)
            }

            fn layout(
                &self,
                ctx: &mut substrate1::layout::context::LayoutCtx,
            ) -> substrate1::error::Result<()> {
                ctx.from_gds_flattened(hard_macro_gds_path($name), $gds)?;
                Ok(())
            }
        }
    };
    ($(#[$meta:meta])* $ident:ident, name = $name:literal) => {
        layout_hard_macro!($(#[$meta])* $ident, name = $name, gds_cell_name = $name);
    };
}

layout_hard_macro!(
    SvtInv2,
    name = "sramgen_svt_inv_2",
    gds_cell_name = "sramgen_svt_inv_2"
);

layout_hard_macro!(
    SvtInv4,
    name = "sramgen_svt_inv_4",
    gds_cell_name = "sramgen_svt_inv_4"
);

layout_hard_macro!(
    SpCell,
    name = "sram_sp_cell",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_cell_opt1"
);

layout_hard_macro!(
    SpCellReplica,
    name = "sram_sp_cell_replica",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_cell_opt1_replica"
);

layout_hard_macro!(
    SpColend,
    name = "sram_sp_colend",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colend"
);

layout_hard_macro!(
    SpHstrap,
    name = "sram_sp_hstrap",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_hstrap"
);

layout_hard_macro!(
    SenseAmp,
    name = "sramgen_sp_sense_amp",
    gds_cell_name = "sramgen_sp_sense_amp"
);

layout_hard_macro!(SenseAmpWithOffset, name = "sramgen_sp_sense_amp_offset");

#[derive(Hash, PartialEq, Eq)]
pub struct SenseAmpCent;

impl Component for SenseAmpCent {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate1::data::SubstrateCtx,
    ) -> substrate1::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sense_amp_cent")
    }

    fn layout(
        &self,
        ctx: &mut substrate1::layout::context::LayoutCtx,
    ) -> substrate1::error::Result<()> {
        let layers = ctx.layers();
        let nwell = layers.get(Selector::Name("nwell"))?;
        let nsdm = layers.get(Selector::Name("nsdm"))?;
        let psdm = layers.get(Selector::Name("psdm"))?;
        let outline = layers.get(Selector::Name("outline"))?;
        let tap = layers.get(Selector::Name("tap"))?;
        let m0 = layers.get(Selector::Metal(0))?;
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;

        let pc = crate::script::run_for_layout::<ColumnDesignScript>(
            ctx.inner(),
            &crate::schematic::NoParams,
        )?;

        let sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        let hspan = Span::new(0, pc.tap_width);
        let bounds = Rect::from_spans(hspan, sa.brect().vspan());

        ctx.draw_rect(
            nwell,
            Rect::from_spans(hspan, sa.layer_bbox(nwell).into_rect().vspan()),
        );

        let nspan = sa.layer_bbox(nsdm).into_rect().vspan();
        let pspan = sa.layer_bbox(psdm).into_rect().vspan();

        for (span, vdd) in [(pspan, true), (nspan, false)] {
            let r = Rect::from_spans(hspan, span).shrink(200);
            let viap = ViaParams::builder().layers(tap, m0).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;
            let sdm_rect = via.layer_bbox(tap).into_rect().expand(130);
            ctx.draw_rect(if vdd { nsdm } else { psdm }, sdm_rect);

            let pspan = sa
                .port(if vdd { "vdd" } else { "vss" })?
                .largest_rect(m2)?
                .vspan();
            let power_stripe = Rect::from_spans(hspan, pspan);

            let viap = ViaParams::builder().layers(m0, m1).geometry(r, r).build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw_ref(&via)?;

            let viap = ViaParams::builder()
                .layers(m1, m2)
                .geometry(via.layer_bbox(m1).into_rect(), power_stripe)
                .expand(ViaExpansion::LongerDirection)
                .build();
            let via = ctx.instantiate::<Via>(&viap)?;
            ctx.draw(via)?;

            ctx.draw_rect(m2, power_stripe);

            let name = if vdd {
                arcstr::literal!("vdd")
            } else {
                arcstr::literal!("vss")
            };
            ctx.merge_port(CellPort::with_shape(name, m2, power_stripe));
        }
        ctx.draw_rect(outline, bounds);

        Ok(())
    }
}

impl crate::schematic::FromParams for SenseAmpCent {
    type Params = crate::schematic::NoParams;
    fn from_params(_params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
impl substrate::block::Block for SenseAmpCent {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sense_amp_cent")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::default()
    }
}
impl substrate::schematic::Schematic for SenseAmpCent {
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
impl SenseAmpCent {
    fn build_schematic(&self, _ctx: &mut crate::schematic::CircuitBuilder) -> anyhow::Result<()> {
        Ok(())
    }
}
crate::impl_sky130_build!(SenseAmpCent);

layout_hard_macro!(
    Dff,
    name = "openram_dff",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff"
);

layout_hard_macro!(
    DffCol,
    name = "openram_dff_col",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff_col"
);

layout_hard_macro!(
    DffColCent,
    name = "openram_dff_col_cent",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff_col_cent"
);

layout_hard_macro!(
    DffColExtend,
    name = "openram_dff_col_extend",
    gds_cell_name = "sky130_fd_bd_sram__openram_dff_col_extend"
);

layout_hard_macro!(
    SpColendCent,
    name = "sram_sp_colend_cent",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colend_cent"
);

layout_hard_macro!(
    SpColendPCent,
    name = "sram_sp_colend_p_cent",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colend_p_cent"
);

layout_hard_macro!(
    SpCorner,
    name = "sram_sp_corner",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_corner"
);

layout_hard_macro!(
    SpRowend,
    name = "sram_sp_rowend",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowend"
);

layout_hard_macro!(
    SpRowendHstrap,
    name = "sram_sp_rowend_hstrap2",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowend_hstrap"
);

layout_hard_macro!(
    SpRowendReplica,
    name = "sram_sp_rowend_replica",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_rowend_replica"
);

layout_hard_macro!(
    SpWlstrap,
    name = "sram_sp_wlstrap",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrap"
);

layout_hard_macro!(
    SpWlstrapP,
    name = "sram_sp_wlstrap_p",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrap_p"
);

layout_hard_macro!(
    SpHorizWlstrapP,
    name = "sram_sp_horiz_wlstrap_p2",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_horiz_wlstrap_p"
);

layout_hard_macro!(
    SpCellOpt1a,
    name = "sram_sp_cell_opt1a",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_cell_opt1a"
);

layout_hard_macro!(
    SpCellOpt1aReplica,
    name = "sram_sp_cell_opt1a_replica",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_cell_opt1a_replica"
);

layout_hard_macro!(
    SpColenda,
    name = "sram_sp_colenda",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda"
);

layout_hard_macro!(
    SpColendaCent,
    name = "sram_sp_colenda_cent",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda_cent"
);

layout_hard_macro!(
    SpColendaPCent,
    name = "sram_sp_colenda_p_cent",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_colenda_p_cent"
);

layout_hard_macro!(
    SpCornera,
    name = "sram_sp_cornera",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_cornera"
);

layout_hard_macro!(
    SpRowenda,
    name = "sram_sp_rowenda",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowenda"
);

layout_hard_macro!(
    SpRowendaReplica,
    name = "sram_sp_rowenda_replica",
    gds_cell_name = "sky130_fd_bd_sram__openram_sp_rowenda_replica"
);

layout_hard_macro!(
    SpRowtapendReplica,
    name = "sram_sp_rowtapend_replica",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_rowtapend_replica"
);

layout_hard_macro!(
    SpWlstrapa,
    name = "sram_sp_wlstrapa",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrapa"
);

layout_hard_macro!(
    SpWlstrapaP,
    name = "sram_sp_wlstrapa_p",
    gds_cell_name = "sky130_fd_bd_sram__sram_sp_wlstrapa_p"
);

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(feature = "commercial")]
    #[ignore = "slow"]
    fn test_sense_amp_clk_cap() {
        use std::collections::HashMap;

        use substrate1::component::NoParams;

        use crate::paths::{out_gds, out_spice};
        use crate::setup_ctx;
        use crate::tests::test_work_dir;

        use super::*;
        use crate::measure::impedance::{
            AcImpedanceTbNode, AcImpedanceTbParams, AcImpedanceTestbench,
        };

        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sense_amp_clk_cap");

        let pex_path = out_spice(&work_dir, "pex_schematic");
        let pex_dir = work_dir.join("pex");
        let pex_level = calibre::pex::PexLevel::Rc;
        let pex_netlist_path = crate::paths::out_pex(&work_dir, "pex_netlist", pex_level);
        crate::netlist::write_schematic::<SenseAmp>(&ctx, &crate::schematic::NoParams, &pex_path)
            .expect("failed to write pex source netlist");
        let mut opts = std::collections::HashMap::with_capacity(1);
        opts.insert("level".into(), pex_level.as_str().into());

        let gds_path = out_gds(&work_dir, "layout");
        crate::layout_ctx()
            .write_layout::<SenseAmp>(&NoParams, &gds_path)
            .expect("failed to write layout");

        ctx.run_pex(crate::verification::calibre::PexInput {
            work_dir: pex_dir,
            layout_path: gds_path.clone(),
            layout_cell_name: arcstr::literal!("sramgen_sp_sense_amp"),
            source_paths: vec![pex_path],
            source_cell_name: arcstr::literal!("sramgen_sp_sense_amp"),
            pex_netlist_path: pex_netlist_path.clone(),
            ground_net: "vss".to_string(),
            opts,
        })
        .expect("failed to run pex");

        let sim_work_dir = work_dir.join("sim");
        let cap = crate::sim::run::<AcImpedanceTestbench<SenseAmp>>(
            &ctx,
            &AcImpedanceTbParams {
                fstart: 100.,
                fstop: 100e6,
                points: 10,
                vdd: 1.8,
                dut: crate::schematic::NoParams,
                pex_netlist: Some(pex_netlist_path.clone()),
                vmeas_conn: AcImpedanceTbNode::Vss,
                connections: HashMap::from_iter([
                    (arcstr::literal!("VDD"), vec![AcImpedanceTbNode::Vdd]),
                    (arcstr::literal!("VSS"), vec![AcImpedanceTbNode::Vss]),
                    (arcstr::literal!("clk"), vec![AcImpedanceTbNode::Vmeas]),
                    (arcstr::literal!("inn"), vec![AcImpedanceTbNode::Vdd]),
                    (arcstr::literal!("inp"), vec![AcImpedanceTbNode::Vss]),
                    (arcstr::literal!("outp"), vec![AcImpedanceTbNode::Floating]),
                    (arcstr::literal!("outn"), vec![AcImpedanceTbNode::Floating]),
                ]),
            },
            &sim_work_dir,
        )
        .expect("failed to write simulation");
        println!("Cclk = {}", cap.max_freq_cap());
    }
}

impl crate::schematic::FromParams for SvtInv2 {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SvtInv2);

impl crate::schematic::FromParams for SvtInv4 {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SvtInv4);

impl crate::schematic::FromParams for SpCell {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SpCell);

impl crate::schematic::FromParams for SpCellReplica {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SpCellReplica);

impl crate::schematic::FromParams for SpColend {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SpColend);

impl crate::schematic::FromParams for SpHstrap {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SpHstrap);

impl crate::schematic::FromParams for SpHorizWlstrapP {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SpHorizWlstrapP);

impl crate::schematic::FromParams for SpRowtapendReplica {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SpRowtapendReplica);

impl crate::schematic::FromParams for SenseAmp {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SenseAmp);

impl crate::schematic::FromParams for SenseAmpWithOffset {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(SenseAmpWithOffset);
