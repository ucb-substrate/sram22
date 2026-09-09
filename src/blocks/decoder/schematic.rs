use crate::schematic::Direction;
use crate::script::DesignContext;
use itertools::Itertools;
use subgeom::bbox::BoundBox;

use crate::schematic::Slice;

use super::{
    Decoder, DecoderParams, DecoderStage, DecoderStageParams, DecoderStagePhysicalDesignScript,
};
use crate::blocks::decoder::{base_indices, DecoderStagePhysicalDesign, RoutingStyle};
use crate::blocks::gate::{Gate, GateParams};

impl Decoder {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let vdd = ctx.port("vdd", crate::schematic::Direction::InOut);
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let mut node = &self.params.tree.root;

        let mut invs = vec![];

        let num_children = node.children.len();
        if num_children == 1 {
            while let GateParams::Inv(params) | GateParams::FoldedInv(params) = node.gate {
                invs.push(params);
                node = &node.children[0];
            }
        }
        invs.reverse();
        let child_sizes = if node.children.is_empty() {
            (0..node.num.ilog2()).map(|_| 2).collect()
        } else {
            node.children.iter().map(|n| n.num).collect()
        };
        let params = DecoderStageParams {
            pd: self.params.pd,
            routing_style: RoutingStyle::Decoder,
            max_width: self.params.max_width,
            gate: node.gate,
            invs,
            num: node.num,
            use_multi_finger_invs: self.params.use_multi_finger_invs,
            dont_connect_outputs: true,
            child_sizes,
        };
        let mut inst = ctx
            .instantiate::<DecoderStage>(&params)?
            .with_connections([("vdd", vdd), ("vss", vss)]);
        let layout_inst = crate::layout_ctx().instantiate_layout::<DecoderStage>(&params)?;
        ctx.bubble_filter_map(&mut inst, |port| {
            port.name().starts_with("y").then_some(port.name().into())
        });
        if node.children.is_empty() {
            ctx.bubble_filter_map(&mut inst, |port| {
                port.name()
                    .starts_with("predecode")
                    .then_some(port.name().into())
            });
        }

        let mut next_addr = (0, 0);
        for (i, node) in node.children.iter().enumerate() {
            let mut child = ctx
                .instantiate::<Decoder>(&DecoderParams {
                    pd: self.params.pd,
                    max_width: Some(
                        self.params
                            .max_width
                            .map(|width| width / num_children as i64)
                            .unwrap_or_else(|| layout_inst.brect().width() / num_children as i64),
                    ),
                    tree: super::DecoderTree { root: node.clone() },
                    use_multi_finger_invs: false,
                })?
                .with_connections([("vdd", vdd), ("vss", vss)]);

            let ports = child.ports().cloned().collect_vec();
            for child_port in ports
                .into_iter()
                .filter_map(|port| {
                    if port.name().starts_with("predecode") {
                        Some(port)
                    } else {
                        None
                    }
                })
                .sorted_unstable_by(|a, b| a.name().cmp(b.name()))
            {
                let port = ctx.port(
                    format!("predecode_{}_{}", next_addr.0, next_addr.1),
                    crate::schematic::Direction::Input,
                );
                child.connect(child_port.name().clone(), port);
                if next_addr.1 > 0 {
                    next_addr = (next_addr.0 + 1, 0);
                } else {
                    next_addr = (next_addr.0, 1);
                }
            }

            let conn = ctx.bus(format!("child_conn_{i}"), node.num);
            let noconn = ctx.bus(format!("child_noconn_{i}"), node.num);

            child.connect("y", conn);
            if child.port("y_b").is_ok() {
                child.connect("y_b", noconn);
            }
            for j in 0..node.num {
                inst.connect(format!("predecode_{i}_{j}"), conn.index(j));
            }
            ctx.add_instance(child);
        }
        ctx.add_instance(inst);

        Ok(())
    }
}

impl DecoderStage {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let DecoderStagePhysicalDesign {
            gate_params,
            folding_factors,
            ..
        } = &*ctx
            .inner()
            .run_script::<DecoderStagePhysicalDesignScript>(&self.params)?;
        let num_stages = gate_params.len();
        let vdd = ctx.port("vdd", crate::schematic::Direction::InOut);
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let y = ctx.bus_port("y", self.params.num, crate::schematic::Direction::Output);
        let y_b = if num_stages > 1 || gate_params[0].gate_type().is_and() {
            Some(ctx.bus_port("y_b", self.params.num, crate::schematic::Direction::Output))
        } else {
            None
        };

        enum DecoderIO {
            Decoder { predecode: Vec<Vec<Slice>> },
            Driver { wl_en: Slice, inn: Slice },
        }
        let io = match self.params.routing_style {
            RoutingStyle::Decoder => {
                let mut predecode = Vec::new();
                for (i, s) in self.params.child_sizes.iter().copied().enumerate() {
                    predecode.push(Vec::new());
                    for j in 0..s {
                        predecode.last_mut().unwrap().push(ctx.port(
                            arcstr::format!("predecode_{i}_{j}"),
                            crate::schematic::Direction::Input,
                        ));
                    }
                }
                DecoderIO::Decoder { predecode }
            }
            RoutingStyle::Driver => DecoderIO::Driver {
                wl_en: ctx.port("wl_en", crate::schematic::Direction::Input),
                inn: ctx.bus_port("in", self.params.num, crate::schematic::Direction::Input),
            },
        };
        let x: Vec<_> = (0..num_stages - 1)
            .map(|i| ctx.bus(format!("x_{i}"), self.params.num))
            .collect();

        for (stage, (gate, &folding_factor)) in
            gate_params.iter().zip(folding_factors.iter()).enumerate()
        {
            let gate_params = gate.scale(1. / (folding_factor as f64));

            for i in 0..self.params.num {
                for j in 0..folding_factor {
                    let mut gate = ctx
                        .instantiate::<Gate>(&gate_params)?
                        .with_connections([("vdd", vdd), ("vss", vss)])
                        .named(format!("gate_{}_{}_{}", stage, i, j));

                    if num_stages > 1 {
                        if stage == num_stages - 2 {
                            gate.connect("y", y_b.unwrap().index(i));
                        } else if stage == num_stages - 1 {
                            gate.connect("y", y.index(i));
                        } else if stage < num_stages - 1 {
                            gate.connect("y", x[stage].index(i));
                        }
                        if gate_params.gate_type().is_and() {
                            gate.connect("yb", ctx.signal(format!("y_b_noconn_{stage}_{i}_{j}")));
                        }
                    } else {
                        if gate_params.gate_type().is_and() {
                            gate.connect("yb", y_b.unwrap().index(i));
                        }
                        gate.connect("y", y.index(i));
                    }
                    if stage == 0 {
                        match &io {
                            DecoderIO::Decoder { predecode } => {
                                let idxs = base_indices(i, &self.params.child_sizes);
                                gate.connect(
                                    "inputs",
                                    crate::schematic::Signal::new(
                                        idxs.into_iter().enumerate().map(|(i, j)| predecode[i][j]),
                                    ),
                                );
                            }
                            DecoderIO::Driver { wl_en, inn } => {
                                gate.connect(
                                    "inputs",
                                    crate::schematic::Signal::new([*wl_en, inn.index(i)]),
                                );
                            }
                        }
                    } else if stage == num_stages - 1 {
                        gate.connect("inputs", y_b.unwrap().index(i));
                    } else {
                        gate.connect("inputs", x[stage - 1].index(i));
                    }
                    gate.add_to(ctx);
                }
            }
        }
        Ok(())
    }
}

impl Decoder {
    pub(crate) fn schematic_io(&self) -> crate::schematic::NamedIo {
        use crate::schematic::{NamedIo, Port};
        let mut io = NamedIo::new([
            ("vdd", 1, Direction::InOut),
            ("vss", 1, Direction::InOut),
            ("y", self.params.tree.root.num, Direction::Output),
        ]);
        let mut node = &self.params.tree.root;
        let mut stages = 1;
        while let GateParams::Inv(_) | GateParams::FoldedInv(_) = node.gate {
            if node.children.len() != 1 {
                break;
            }
            stages += 1;
            node = &node.children[0];
        }
        if stages > 1 || node.gate.gate_type().is_and() {
            io.0.push(Port {
                name: "y_b".into(),
                width: self.params.tree.root.num,
                direction: Direction::Output,
            });
        }
        for i in 0..self.params.tree.root.num.ilog2() {
            for j in 0..2 {
                io.0.push(Port {
                    name: arcstr::format!("predecode_{i}_{j}"),
                    width: 1,
                    direction: Direction::Input,
                });
            }
        }
        io
    }
}
impl DecoderStage {
    pub(crate) fn schematic_io(&self) -> crate::schematic::NamedIo {
        use crate::schematic::{NamedIo, Port};
        let mut io = NamedIo::new([
            ("vdd", 1, Direction::InOut),
            ("vss", 1, Direction::InOut),
            ("y", self.params.num, Direction::Output),
        ]);
        if !self.params.invs.is_empty() || self.params.gate.gate_type().is_and() {
            io.0.push(Port {
                name: "y_b".into(),
                width: self.params.num,
                direction: Direction::Output,
            });
        }
        match self.params.routing_style {
            RoutingStyle::Decoder => {
                for (i, &n) in self.params.child_sizes.iter().enumerate() {
                    for j in 0..n {
                        io.0.push(Port {
                            name: arcstr::format!("predecode_{i}_{j}"),
                            width: 1,
                            direction: Direction::Input,
                        });
                    }
                }
            }
            RoutingStyle::Driver => {
                io.0.push(Port {
                    name: "wl_en".into(),
                    width: 1,
                    direction: Direction::Input,
                });
                io.0.push(Port {
                    name: "in".into(),
                    width: self.params.num,
                    direction: Direction::Input,
                });
            }
        }
        io
    }
}
