//! Schematics of the primitive and compound logic gates.

use sky130::mos::{Nfet01v8, Pfet01v8};
use sky130::Sky130;
use substrate::block::Block;
use substrate::error::Result;
use substrate::schematic::{CellBuilder, Schematic};
use substrate::types::schematic::{IoNodeBundle, Node, NodeBundle};
use substrate::types::{Array, InOut, Input, Io, Output, Signal};

use super::{
    And2, And3, FoldedInv, Gate, GateParams, Inv, MultiFingerInv, MultiFingerInvMosParams, Nand2,
    Nand3, Nor2, PrimitiveGateParams, TappedGate,
};

/// The IO of a single-input gate (an inverter).
#[derive(Debug, Default, Clone, Copy, Io)]
pub struct InvIo {
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
    pub a: Input<Signal>,
    pub y: Output<Signal>,
}

/// The IO of a two-input primitive gate.
#[derive(Debug, Default, Clone, Copy, Io)]
pub struct Gate2Io {
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
    pub a: Input<Signal>,
    pub b: Input<Signal>,
    pub y: Output<Signal>,
}

/// The IO of a three-input primitive gate.
#[derive(Debug, Default, Clone, Copy, Io)]
pub struct Gate3Io {
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
    pub a: Input<Signal>,
    pub b: Input<Signal>,
    pub c: Input<Signal>,
    pub y: Output<Signal>,
}

/// The IO of a two-input AND gate.
///
/// `yb` is the (inverted) output of the internal NAND gate.
#[derive(Debug, Default, Clone, Copy, Io)]
pub struct And2Io {
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
    pub a: Input<Signal>,
    pub b: Input<Signal>,
    pub y: Output<Signal>,
    pub yb: Output<Signal>,
}

/// The IO of a three-input AND gate.
///
/// `yb` is the (inverted) output of the internal NAND gate.
#[derive(Debug, Default, Clone, Copy, Io)]
pub struct And3Io {
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
    pub a: Input<Signal>,
    pub b: Input<Signal>,
    pub c: Input<Signal>,
    pub y: Output<Signal>,
    pub yb: Output<Signal>,
}

/// The IO of an arbitrary [`Gate`].
///
/// `inputs` has one element per gate input. `yb` has one element (the inverted output)
/// for AND gates and is empty otherwise.
#[derive(Debug, Clone, Io)]
pub struct GateIo {
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
    pub inputs: Input<Array<Signal>>,
    pub y: Output<Signal>,
    pub yb: Output<Array<Signal>>,
}

impl GateParams {
    /// The IO of a gate with these parameters.
    pub fn gate_io(&self) -> GateIo {
        GateIo {
            vdd: InOut(Signal),
            vss: InOut(Signal),
            inputs: Input(Array::new(self.num_inputs(), Signal)),
            y: Output(Signal),
            yb: Output(Array::new(
                if self.gate_type().is_and() { 1 } else { 0 },
                Signal,
            )),
        }
    }
}

fn nmos(w: i64, l: i64) -> Nfet01v8 {
    Nfet01v8::new((w, l))
}

fn pmos(w: i64, l: i64) -> Pfet01v8 {
    Pfet01v8::new((w, l))
}

fn inverter(
    cell: &mut CellBuilder<Sky130>,
    params: PrimitiveGateParams,
    vdd: Node,
    vss: Node,
    a: Node,
    y: Node,
    suffix: &str,
) {
    let mp = cell.instantiate_named(pmos(params.pwidth, params.length), format!("MP{suffix}"));
    cell.connect(mp.io().d, y);
    cell.connect(mp.io().g, a);
    cell.connect(mp.io().s, vdd);
    cell.connect(mp.io().b, vdd);

    let mn = cell.instantiate_named(nmos(params.nwidth, params.length), format!("MN{suffix}"));
    cell.connect(mn.io().d, y);
    cell.connect(mn.io().g, a);
    cell.connect(mn.io().s, vss);
    cell.connect(mn.io().b, vss);
}

impl Block for Inv {
    type Io = InvIo;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("inv")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for Inv {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        inverter(cell, self.params, io.vdd, io.vss, io.a, io.y, "0");
        Ok(())
    }
}

impl Block for FoldedInv {
    type Io = InvIo;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("folded_inv")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for FoldedInv {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let half_params = self.params.scale(0.5);
        for i in 0..2 {
            inverter(
                cell,
                half_params,
                io.vdd,
                io.vss,
                io.a,
                io.y,
                &i.to_string(),
            );
        }
        Ok(())
    }
}

impl Block for MultiFingerInv {
    type Io = InvIo;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("multi_finger_inv")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for MultiFingerInv {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let MultiFingerInvMosParams {
            nmos_nf,
            pmos_nf,
            unit_width,
            length,
        } = self.mos_params();

        for i in 0..pmos_nf {
            let mp = cell.instantiate_named(pmos(unit_width, length), format!("MP{i}"));
            cell.connect(mp.io().d, io.y);
            cell.connect(mp.io().g, io.a);
            cell.connect(mp.io().s, io.vdd);
            cell.connect(mp.io().b, io.vdd);
        }

        for i in 0..nmos_nf {
            let mn = cell.instantiate_named(nmos(unit_width, length), format!("MN{i}"));
            cell.connect(mn.io().d, io.y);
            cell.connect(mn.io().g, io.a);
            cell.connect(mn.io().s, io.vss);
            cell.connect(mn.io().b, io.vss);
        }

        Ok(())
    }
}

impl Block for Nand2 {
    type Io = Gate2Io;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("nand2")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for Nand2 {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let length = self.params.length;
        let x = cell.signal("x", Signal);

        let n1 = cell.instantiate_named(nmos(self.params.nwidth, length), "n1");
        cell.connect(n1.io().d, x);
        cell.connect(n1.io().g, io.a);
        cell.connect(n1.io().s, io.vss);
        cell.connect(n1.io().b, io.vss);

        let n2 = cell.instantiate_named(nmos(self.params.nwidth, length), "n2");
        cell.connect(n2.io().d, io.y);
        cell.connect(n2.io().g, io.b);
        cell.connect(n2.io().s, x);
        cell.connect(n2.io().b, io.vss);

        for (i, g) in [io.a, io.b].into_iter().enumerate() {
            let p = cell.instantiate_named(pmos(self.params.pwidth, length), format!("p{}", i + 1));
            cell.connect(p.io().d, io.y);
            cell.connect(p.io().g, g);
            cell.connect(p.io().s, io.vdd);
            cell.connect(p.io().b, io.vdd);
        }

        Ok(())
    }
}

impl Block for Nand3 {
    type Io = Gate3Io;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("nand3")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for Nand3 {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let length = self.params.length;
        let x1 = cell.signal("x1", Signal);
        let x2 = cell.signal("x2", Signal);

        let n1 = cell.instantiate_named(nmos(self.params.nwidth, length), "n1");
        cell.connect(n1.io().d, x1);
        cell.connect(n1.io().g, io.a);
        cell.connect(n1.io().s, io.vss);
        cell.connect(n1.io().b, io.vss);

        let n2 = cell.instantiate_named(nmos(self.params.nwidth, length), "n2");
        cell.connect(n2.io().d, x2);
        cell.connect(n2.io().g, io.b);
        cell.connect(n2.io().s, x1);
        cell.connect(n2.io().b, io.vss);

        let n3 = cell.instantiate_named(nmos(self.params.nwidth, length), "n3");
        cell.connect(n3.io().d, io.y);
        cell.connect(n3.io().g, io.c);
        cell.connect(n3.io().s, x2);
        cell.connect(n3.io().b, io.vss);

        for (i, g) in [io.a, io.b, io.c].into_iter().enumerate() {
            let p = cell.instantiate_named(pmos(self.params.pwidth, length), format!("p{}", i + 1));
            cell.connect(p.io().d, io.y);
            cell.connect(p.io().g, g);
            cell.connect(p.io().s, io.vdd);
            cell.connect(p.io().b, io.vdd);
        }

        Ok(())
    }
}

impl Block for Nor2 {
    type Io = Gate2Io;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("nor2")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for Nor2 {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let length = self.params.length;
        let x = cell.signal("x", Signal);

        for (i, g) in [io.a, io.b].into_iter().enumerate() {
            let n = cell.instantiate_named(nmos(self.params.nwidth, length), format!("n{}", i + 1));
            cell.connect(n.io().d, io.y);
            cell.connect(n.io().g, g);
            cell.connect(n.io().s, io.vss);
            cell.connect(n.io().b, io.vss);
        }

        let p1 = cell.instantiate_named(pmos(self.params.pwidth, length), "p1");
        cell.connect(p1.io().d, io.y);
        cell.connect(p1.io().g, io.a);
        cell.connect(p1.io().s, x);
        cell.connect(p1.io().b, io.vdd);

        let p2 = cell.instantiate_named(pmos(self.params.pwidth, length), "p2");
        cell.connect(p2.io().d, x);
        cell.connect(p2.io().g, io.b);
        cell.connect(p2.io().s, io.vdd);
        cell.connect(p2.io().b, io.vdd);

        Ok(())
    }
}

impl Block for And2 {
    type Io = And2Io;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("and2")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for And2 {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        cell.instantiate_connected(
            Nand2::new(self.params.nand),
            NodeBundle::<Gate2Io> {
                vdd: io.vdd,
                vss: io.vss,
                a: io.a,
                b: io.b,
                y: io.yb,
            },
        );
        cell.instantiate_connected(
            FoldedInv::new(self.params.inv),
            NodeBundle::<InvIo> {
                vdd: io.vdd,
                vss: io.vss,
                a: io.yb,
                y: io.y,
            },
        );
        Ok(())
    }
}

impl Block for And3 {
    type Io = And3Io;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("and3")
    }

    fn io(&self) -> Self::Io {
        Default::default()
    }
}

impl Schematic for And3 {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        cell.instantiate_connected(
            Nand3::new(self.params.nand),
            NodeBundle::<Gate3Io> {
                vdd: io.vdd,
                vss: io.vss,
                a: io.a,
                b: io.b,
                c: io.c,
                y: io.yb,
            },
        );
        cell.instantiate_connected(
            FoldedInv::new(self.params.inv),
            NodeBundle::<InvIo> {
                vdd: io.vdd,
                vss: io.vss,
                a: io.yb,
                y: io.y,
            },
        );
        Ok(())
    }
}

impl Block for Gate {
    type Io = GateIo;

    fn name(&self) -> arcstr::ArcStr {
        match self {
            Gate::And2(g) => Block::name(g),
            Gate::And3(g) => Block::name(g),
            Gate::Inv(g) => Block::name(g),
            Gate::FoldedInv(g) => Block::name(g),
            Gate::MultiFingerInv(g) => Block::name(g),
            Gate::Nand2(g) => Block::name(g),
            Gate::Nand3(g) => Block::name(g),
            Gate::Nor2(g) => Block::name(g),
        }
    }

    fn io(&self) -> Self::Io {
        self.params().gate_io()
    }
}

impl Schematic for Gate {
    type Schema = Sky130;
    type NestedData = ();

    /// Generates the gate's devices directly in this cell (without an extra level of
    /// hierarchy), matching the structure of the gate layouts.
    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let (vdd, vss, y) = (io.vdd, io.vss, io.y);
        let inputs = &io.inputs;
        match self {
            Gate::And2(g) => g.schematic(
                &NodeBundle::<And2Io> {
                    vdd,
                    vss,
                    a: inputs[0],
                    b: inputs[1],
                    y,
                    yb: io.yb[0],
                },
                cell,
            ),
            Gate::And3(g) => g.schematic(
                &NodeBundle::<And3Io> {
                    vdd,
                    vss,
                    a: inputs[0],
                    b: inputs[1],
                    c: inputs[2],
                    y,
                    yb: io.yb[0],
                },
                cell,
            ),
            Gate::Inv(g) => g.schematic(
                &NodeBundle::<InvIo> {
                    vdd,
                    vss,
                    a: inputs[0],
                    y,
                },
                cell,
            ),
            Gate::FoldedInv(g) => g.schematic(
                &NodeBundle::<InvIo> {
                    vdd,
                    vss,
                    a: inputs[0],
                    y,
                },
                cell,
            ),
            Gate::MultiFingerInv(g) => g.schematic(
                &NodeBundle::<InvIo> {
                    vdd,
                    vss,
                    a: inputs[0],
                    y,
                },
                cell,
            ),
            Gate::Nand2(g) => g.schematic(
                &NodeBundle::<Gate2Io> {
                    vdd,
                    vss,
                    a: inputs[0],
                    b: inputs[1],
                    y,
                },
                cell,
            ),
            Gate::Nand3(g) => g.schematic(
                &NodeBundle::<Gate3Io> {
                    vdd,
                    vss,
                    a: inputs[0],
                    b: inputs[1],
                    c: inputs[2],
                    y,
                },
                cell,
            ),
            Gate::Nor2(g) => g.schematic(
                &NodeBundle::<Gate2Io> {
                    vdd,
                    vss,
                    a: inputs[0],
                    b: inputs[1],
                    y,
                },
                cell,
            ),
        }
    }
}

impl Block for TappedGate {
    type Io = GateIo;

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tapped_gate")
    }

    fn io(&self) -> Self::Io {
        self.params.gate_io()
    }
}

impl Schematic for TappedGate {
    type Schema = Sky130;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        cell.instantiate_connected_named(Gate::new(self.params), io, "gate");
        Ok(())
    }
}
