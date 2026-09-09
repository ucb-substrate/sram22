use std::collections::HashMap;
use std::path::PathBuf;

use arcstr::ArcStr;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::schematic::Signal;
use crate::sim::blocks::Idc;
use crate::sim::blocks::Vdc;
use crate::sim::run::SimulationTestbench as Testbench;
use crate::sim::run::{Analysis, Save, TranAnalysis};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum TbNode {
    Vdd,
    Vss,
    // Node to be measured.
    Vmeas,
    Floating,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeCap {
    pub cnode: f64,
}

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(derive(Debug))]
pub struct TbParams<T> {
    /// Current source value in nano amperes.
    pub idc: i64,
    /// Supply voltage.
    pub vdd: f64,
    pub dut: T,
    pub pex_netlist: Option<PathBuf>,
    pub connections: HashMap<ArcStr, Vec<TbNode>>,
}

impl<T: Clone> TbParams<T> {
    #[inline]
    pub fn builder() -> TbParamsBuilder<T> {
        TbParamsBuilder::default()
    }
}

pub struct CapTestbench<T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>> {
    params: TbParams<T::Params>,
}

impl<T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>> CapTestbench<T> {
    fn cache_key(&self) -> impl std::hash::Hash + Eq + '_ {
        let p = &self.params;
        let mut connections: Vec<_> = p.connections.iter().collect();
        connections.sort_by(|a, b| a.0.cmp(b.0));
        (
            [p.vdd.to_bits()],
            &p.idc,
            &p.dut,
            &p.pex_netlist,
            connections,
        )
    }
}

impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > std::hash::Hash for CapTestbench<T>
{
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash(&self.cache_key(), h)
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > PartialEq for CapTestbench<T>
{
    fn eq(&self, other: &Self) -> bool {
        self.cache_key() == other.cache_key()
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > Eq for CapTestbench<T>
{
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > crate::schematic::FromParams for CapTestbench<T>
{
    type Params = TbParams<T::Params>;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > substrate::block::Block for CapTestbench<T>
{
    type Io = substrate::types::TestbenchIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("cap_testbench")
    }
    fn io(&self) -> Self::Io {
        Default::default()
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > substrate::schematic::Schematic for CapTestbench<T>
{
    type Schema = crate::sim::Simulator;
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
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > CapTestbench<T>
{
    fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder<crate::sim::Simulator>,
    ) -> anyhow::Result<()> {
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let vdd = ctx.signal("vdd");
        let vmeas = ctx.signal("vmeas");
        let mut ctr = 0;

        let mut connections = Vec::new();

        for (k, nodes) in &self.params.connections {
            let mut signal = Vec::new();
            for node in nodes {
                signal.push(match node {
                    TbNode::Vdd => vdd,
                    TbNode::Vss => vss,
                    TbNode::Vmeas => vmeas,
                    TbNode::Floating => {
                        ctr += 1;
                        ctx.signal(format!("floating{ctr}"))
                    }
                });
            }
            connections.push((k.clone(), Signal::new(signal)));
        }

        if let Some(netlist) = &self.params.pex_netlist {
            ctx.instantiate_pex::<T>(&self.params.dut, netlist)?
                .with_connections(connections)
                .named("dut")
                .add_to(ctx);
        } else {
            ctx.instantiate::<T>(&self.params.dut)?
                .with_connections(connections)
                .named("dut")
                .add_to(ctx);
        }

        ctx.instantiate::<Vdc>(&crate::sim::dec(self.params.vdd))?
            .with_connections([("p", vdd), ("n", vss)])
            .named("Vdd")
            .add_to(ctx);

        let mut idc = ctx.instantiate::<Idc>(&crate::sim::dec((self.params.idc) as f64 * 1e-9))?;
        idc.connect_all([("p", vss), ("n", vmeas)]);
        idc.set_name("iin");
        ctx.add_instance(idc);

        Ok(())
    }
}

impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > Testbench for CapTestbench<T>
{
    type Output = NodeCap;

    fn setup(&self, ctx: &mut crate::sim::run::SimulationPlan) -> anyhow::Result<()> {
        if let Some(ref netlist) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        ctx.set_ic("vmeas", rust_decimal::Decimal::ZERO);
        ctx.add_analysis(Analysis::Tran(
            TranAnalysis::builder()
                .stop(6e-6)
                .start(0.0)
                .step(1e-9)
                .build()
                .unwrap(),
        ))
        .save(Save::Signals(["vmeas".into()].into_iter().collect()));
        Ok(())
    }

    fn measure(&self, ctx: &crate::sim::run::SimulationResults) -> anyhow::Result<Self::Output> {
        let data = ctx.data[0].tran();
        let sig = &data.data["vmeas"];
        let (idx1, v1) = sig
            .values
            .iter()
            .enumerate()
            .find(|(_i, &x)| x > 0.1)
            .unwrap();
        let (idx2, v2) = sig
            .values
            .iter()
            .enumerate()
            .find(|(_i, &x)| x > 0.5)
            .unwrap();

        let t1 = data.time.values[idx1];
        let t2 = data.time.values[idx2];

        assert!(v2 > v1);
        assert!(idx2 > idx1);
        assert!(t2 > t1);

        let cnode = self.params.idc as f64 * 1e-9 * (t2 - t1) / (v2 - v1);

        Ok(NodeCap { cnode })
    }
}
