//! Substrate 2 schematic views of the SPICE-backed hard macros.
//!
//! Each hard macro is a SCIR cell parsed from `tech/sky130/spice` and converted into the
//! [`Sky130`] schema so that its transistors are netlisted with the correct device names for
//! whichever PDK flavor (open-source or NDA) is in use.

use sky130::Sky130;
use spice::Spice;
use substrate::block::Block;
use substrate::error::Result;
use substrate::schematic::{CellBuilder, Schematic};
use substrate::types::schematic::IoNodeBundle;
use substrate::types::{InOut, Input, Io, Output, Signal};

use super::hard_macro_spice_path;
use super::{
    SenseAmp, SenseAmpWithOffset, SpCell, SpCellReplica, SpColend, SpHorizWlstrapP, SpHstrap,
    SpRowtapendReplica, SvtInv2, SvtInv4,
};

/// Declares the Substrate 2 schematic view of a SPICE-backed hard macro.
///
/// `$ident` must already be declared as a unit struct (see `layout_hard_macro!`).
/// Each `($field, $port, $dir)` entry maps an IO field to a port of the SPICE subcircuit.
macro_rules! spice_hard_macro {
    (
        $(#[$meta:meta])*
        $ident:ident, $io:ident,
        name = $name:literal,
        subckt = $subckt:literal,
        ports = [$(($field:ident, $port:literal, $dir:ident)),* $(,)?]
    ) => {
        $(#[$meta])*
        #[derive(Debug, Default, Clone, Io)]
        pub struct $io {
            $(pub $field: $dir<Signal>,)*
        }

        impl Block for $ident {
            type Io = $io;

            fn name(&self) -> arcstr::ArcStr {
                arcstr::literal!($name)
            }

            fn io(&self) -> Self::Io {
                Default::default()
            }
        }

        impl Schematic for $ident {
            type Schema = Sky130;
            type NestedData = ();

            fn schematic(
                &self,
                io: &IoNodeBundle<Self>,
                cell: &mut CellBuilder<<Self as Schematic>::Schema>,
            ) -> Result<Self::NestedData> {
                let mut scir = Spice::scir_cell_from_file(hard_macro_spice_path($name), $subckt)
                    .convert_schema::<Sky130>()?;
                $(scir.connect($port, io.$field);)*
                cell.set_scir(scir);
                Ok(())
            }
        }
    };
}

spice_hard_macro!(
    /// IO of a standard cell style inverter hard macro.
    SvtInv2, SvtInvIo,
    name = "sramgen_svt_inv_2",
    subckt = "sramgen_svt_inv_2",
    ports = [
        (a, "A", Input),
        (vgnd, "VGND", InOut),
        (vnb, "VNB", InOut),
        (vpb, "VPB", InOut),
        (vpwr, "VPWR", InOut),
        (y, "Y", Output),
    ]
);

impl Block for SvtInv4 {
    type Io = SvtInvIo;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sramgen_svt_inv_4")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for SvtInv4 {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let mut scir = Spice::scir_cell_from_file(
            hard_macro_spice_path("sramgen_svt_inv_4"),
            "sramgen_svt_inv_4",
        )
        .convert_schema::<Sky130>()?;
        scir.connect("A", io.a);
        scir.connect("VGND", io.vgnd);
        scir.connect("VNB", io.vnb);
        scir.connect("VPB", io.vpb);
        scir.connect("VPWR", io.vpwr);
        scir.connect("Y", io.y);
        cell.set_scir(scir);
        Ok(())
    }
}

spice_hard_macro!(
    /// IO of the single-port SRAM bitcell.
    SpCell, SpCellIo,
    name = "sram_sp_cell",
    subckt = "sram_sp_cell",
    ports = [
        (bl, "BL", InOut),
        (br, "BR", InOut),
        (vdd, "VDD", InOut),
        (vss, "VSS", InOut),
        (wl, "WL", Input),
        (vnb, "VNB", InOut),
        (vpb, "VPB", InOut),
    ]
);

spice_hard_macro!(
    /// IO of the replica bitcell.
    SpCellReplica, SpCellReplicaIo,
    name = "sram_sp_cell_replica",
    subckt = "sram_sp_cell_replica",
    ports = [
        (bl, "BL", InOut),
        (br, "BR", InOut),
        (vss, "VSS", InOut),
        (vdd, "VDD", InOut),
        (vpb, "VPB", InOut),
        (vnb, "VNB", InOut),
        (wl, "WL", Input),
    ]
);

spice_hard_macro!(
    /// IO of a bitcell column end cap.
    SpColend, SpColendIo,
    name = "sram_sp_colend",
    subckt = "sram_sp_colend",
    ports = [
        (br, "BR", InOut),
        (vdd, "VDD", InOut),
        (vss, "VSS", InOut),
        (bl, "BL", InOut),
        (vnb, "VNB", InOut),
        (vpb, "VPB", InOut),
    ]
);

impl Block for SpHstrap {
    type Io = SpColendIo;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sram_sp_hstrap")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for SpHstrap {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let mut scir =
            Spice::scir_cell_from_file(hard_macro_spice_path("sram_sp_hstrap"), "sram_sp_hstrap")
                .convert_schema::<Sky130>()?;
        scir.connect("BR", io.br);
        scir.connect("VDD", io.vdd);
        scir.connect("VSS", io.vss);
        scir.connect("BL", io.bl);
        scir.connect("VNB", io.vnb);
        scir.connect("VPB", io.vpb);
        cell.set_scir(scir);
        Ok(())
    }
}

spice_hard_macro!(
    /// IO of a horizontal wordline strap cell.
    SpHorizWlstrapP, WlstrapIo,
    name = "sram_sp_horiz_wlstrap_p2",
    subckt = "sram_sp_horiz_wlstrap_p2",
    ports = [
        (vss, "VSS", InOut),
        (vnb, "VNB", InOut),
    ]
);

impl Block for SpRowtapendReplica {
    type Io = WlstrapIo;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sram_sp_rowtapend_replica")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for SpRowtapendReplica {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let mut scir = Spice::scir_cell_from_file(
            hard_macro_spice_path("sram_sp_rowtapend_replica"),
            "sram_sp_rowtapend_replica",
        )
        .convert_schema::<Sky130>()?;
        scir.connect("VSS", io.vss);
        scir.connect("VNB", io.vnb);
        cell.set_scir(scir);
        Ok(())
    }
}

spice_hard_macro!(
    /// IO of the sense amplifier.
    SenseAmp, SenseAmpIo,
    name = "sramgen_sp_sense_amp",
    subckt = "sramgen_sp_sense_amp",
    ports = [
        (clk, "clk", Input),
        (inn, "inn", Input),
        (inp, "inp", Input),
        (outn, "outn", Output),
        (outp, "outp", Output),
        (vdd, "VDD", InOut),
        (vss, "VSS", InOut),
    ]
);

impl Block for SenseAmpWithOffset {
    type Io = SenseAmpIo;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sramgen_sp_sense_amp_offset")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for SenseAmpWithOffset {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let mut scir = Spice::scir_cell_from_file(
            hard_macro_spice_path("sramgen_sp_sense_amp_offset"),
            "sramgen_sp_sense_amp_offset",
        )
        .convert_schema::<Sky130>()?;
        scir.connect("clk", io.clk);
        scir.connect("inn", io.inn);
        scir.connect("inp", io.inp);
        scir.connect("outn", io.outn);
        scir.connect("outp", io.outp);
        scir.connect("VDD", io.vdd);
        scir.connect("VSS", io.vss);
        cell.set_scir(scir);
        Ok(())
    }
}
