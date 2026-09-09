use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::PathBuf;

use arcstr::ArcStr;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::schematic::Signal;
use crate::sim::blocks::Iac;
use crate::sim::blocks::Resistor;
use crate::sim::blocks::Vdc;
use crate::sim::blocks::Vpulse;
use crate::sim::run::SimulationTestbench as Testbench;
use crate::sim::run::{
    AcAnalysis, Analysis, ComplexSignal, RealSignal, Save, SweepMode, TranAnalysis,
};
use rust_decimal::Decimal;
use substrate::simulation::waveform::{EdgeDir, TimeWaveform};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum TransitionTbNode {
    Vdd,
    Vss,
    // Node to be measured.
    Vmeas,
    // Node to apply stimulus.
    Vstim,
    Floating,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionTimes {
    pub tr: f64,
    pub tf: f64,
}

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(derive(Debug))]
pub struct TransitionTbParams<T> {
    /// Supply voltage.
    pub vdd: f64,
    pub delay: f64,
    pub width: f64,
    pub fall: f64,
    pub rise: f64,
    pub upper_threshold: f64,
    pub lower_threshold: f64,
    pub dut: T,
    pub pex_netlist: Option<PathBuf>,
    pub connections: HashMap<ArcStr, Vec<TransitionTbNode>>,
}

impl<T: Clone> TransitionTbParams<T> {
    #[inline]
    pub fn builder() -> TransitionTbParamsBuilder<T> {
        TransitionTbParamsBuilder::default()
    }
}

pub struct TransitionTestbench<T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>> {
    params: TransitionTbParams<T::Params>,
}

impl<T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>> TransitionTestbench<T> {
    fn cache_key(&self) -> impl std::hash::Hash + Eq + '_ {
        let p = &self.params;
        let mut connections: Vec<_> = p.connections.iter().collect();
        connections.sort_by(|a, b| a.0.cmp(b.0));
        (
            [
                p.vdd.to_bits(),
                p.delay.to_bits(),
                p.width.to_bits(),
                p.fall.to_bits(),
                p.rise.to_bits(),
                p.upper_threshold.to_bits(),
                p.lower_threshold.to_bits(),
            ],
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
    > std::hash::Hash for TransitionTestbench<T>
{
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash(&self.cache_key(), h)
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > PartialEq for TransitionTestbench<T>
{
    fn eq(&self, other: &Self) -> bool {
        self.cache_key() == other.cache_key()
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > Eq for TransitionTestbench<T>
{
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > crate::schematic::FromParams for TransitionTestbench<T>
{
    type Params = TransitionTbParams<T::Params>;
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
    > substrate::block::Block for TransitionTestbench<T>
{
    type Io = substrate::types::TestbenchIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("transition_testbench")
    }
    fn io(&self) -> Self::Io {
        Default::default()
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > substrate::schematic::Schematic for TransitionTestbench<T>
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
    > TransitionTestbench<T>
{
    fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder<crate::sim::Simulator>,
    ) -> anyhow::Result<()> {
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let vdd = ctx.signal("vdd");
        let vmeas = ctx.signal("vmeas");
        let vstim = ctx.signal("vstim");
        let mut ctr = 0;

        let mut connections = Vec::new();

        for (k, nodes) in &self.params.connections {
            let mut signal = Vec::new();
            for node in nodes {
                signal.push(match node {
                    TransitionTbNode::Vdd => vdd,
                    TransitionTbNode::Vss => vss,
                    TransitionTbNode::Vmeas => vmeas,
                    TransitionTbNode::Vstim => vstim,
                    TransitionTbNode::Floating => {
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

        ctx.instantiate::<Vpulse>(&Vpulse {
            val0: Decimal::ZERO,
            val1: crate::sim::dec(self.params.vdd),
            delay: crate::sim::dec(self.params.delay),
            rise: crate::sim::dec(self.params.rise),
            fall: crate::sim::dec(self.params.fall),
            width: crate::sim::dec(self.params.width),
            period: Decimal::ZERO,
        })?
        .with_connections([("p", vstim), ("n", vss)])
        .named("Vstim")
        .add_to(ctx);

        Ok(())
    }
}

impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > Testbench for TransitionTestbench<T>
{
    type Output = TransitionTimes;

    fn setup(&self, ctx: &mut crate::sim::run::SimulationPlan) -> anyhow::Result<()> {
        if let Some(ref netlist) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        let duration = (self.params.delay + self.params.width) * 2.;
        ctx.add_analysis(Analysis::Tran(
            TranAnalysis::builder()
                .stop(duration)
                .start(0.0)
                .step(duration / 1e5)
                .build()
                .unwrap(),
        ))
        .save(Save::All);
        Ok(())
    }

    fn measure(&self, ctx: &crate::sim::run::SimulationResults) -> anyhow::Result<Self::Output> {
        let data = ctx.data[0].tran();
        let sig = data.waveform("vmeas").expect("missing vmeas waveform");
        let transitions = sig
            .transitions(
                self.params.lower_threshold * self.params.vdd,
                self.params.upper_threshold * self.params.vdd,
            )
            .collect::<Vec<_>>();

        let t1 = transitions[0];
        let t2 = transitions[1];

        let (tr, tf) = match t1.dir() {
            EdgeDir::Rising => (t1.duration(), t2.duration()),
            EdgeDir::Falling => (t2.duration(), t1.duration()),
        };

        Ok(TransitionTimes { tr, tf })
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum AcImpedanceTbNode {
    Vdd,
    Vss,
    Vcustom(Decimal),
    // Node to be measured.
    Vmeas,
    Floating,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcImpedance {
    pub vmeas: ComplexSignal,
    pub freq: RealSignal,
}

impl AcImpedance {
    /// Returns the capacitance at high frequency from the reactance.
    pub fn max_freq_cap(&self) -> f64 {
        -1. / self.vmeas.imag.last().unwrap() / self.freq.values.last().unwrap() / 2. / PI
    }

    /// Returns the resistance at low frequency.
    pub fn min_freq_res(&self) -> f64 {
        *self.vmeas.real.first().unwrap()
    }
}

#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(derive(Debug))]
pub struct AcImpedanceTbParams<T> {
    /// Supply voltage.
    pub vdd: f64,
    pub fstart: f64,
    pub fstop: f64,
    pub points: usize,
    pub dut: T,
    pub pex_netlist: Option<PathBuf>,
    pub vmeas_conn: AcImpedanceTbNode,
    pub connections: HashMap<ArcStr, Vec<AcImpedanceTbNode>>,
}

impl<T: Clone> AcImpedanceTbParams<T> {
    #[inline]
    pub fn builder() -> AcImpedanceTbParamsBuilder<T> {
        AcImpedanceTbParamsBuilder::default()
    }
}

pub struct AcImpedanceTestbench<T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>> {
    params: AcImpedanceTbParams<T::Params>,
}

impl<T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>> AcImpedanceTestbench<T> {
    fn cache_key(&self) -> impl std::hash::Hash + Eq + '_ {
        let p = &self.params;
        let mut connections: Vec<_> = p.connections.iter().collect();
        connections.sort_by(|a, b| a.0.cmp(b.0));
        (
            [p.vdd.to_bits(), p.fstart.to_bits(), p.fstop.to_bits()],
            &p.points,
            &p.dut,
            &p.pex_netlist,
            &p.vmeas_conn,
            connections,
        )
    }
}

impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > std::hash::Hash for AcImpedanceTestbench<T>
{
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash(&self.cache_key(), h)
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > PartialEq for AcImpedanceTestbench<T>
{
    fn eq(&self, other: &Self) -> bool {
        self.cache_key() == other.cache_key()
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > Eq for AcImpedanceTestbench<T>
{
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > crate::schematic::FromParams for AcImpedanceTestbench<T>
{
    type Params = AcImpedanceTbParams<T::Params>;
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
    > substrate::block::Block for AcImpedanceTestbench<T>
{
    type Io = substrate::types::TestbenchIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("transition_testbench")
    }
    fn io(&self) -> Self::Io {
        Default::default()
    }
}
impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > substrate::schematic::Schematic for AcImpedanceTestbench<T>
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
    > AcImpedanceTestbench<T>
{
    fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder<crate::sim::Simulator>,
    ) -> anyhow::Result<()> {
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let vdd = ctx.signal("vdd");
        let vmeas = ctx.signal("vmeas");
        let mut ctr = 0;

        match self.params.vmeas_conn {
            AcImpedanceTbNode::Vdd => {
                ctx.instantiate::<Resistor>(&crate::sim::dec((100) as f64 * 1e6))?
                    .with_connections([("p", vmeas), ("n", vdd)])
                    .named("Vmeas")
                    .add_to(ctx);
            }
            AcImpedanceTbNode::Vss => {
                ctx.instantiate::<Resistor>(&crate::sim::dec((100) as f64 * 1e6))?
                    .with_connections([("p", vmeas), ("n", vss)])
                    .named("Vmeas")
                    .add_to(ctx);
            }
            AcImpedanceTbNode::Vcustom(val) => {
                ctr += 1;
                let vcustom = ctx.signal(format!("custom{ctr}"));
                ctx.instantiate::<Vdc>(&val)?
                    .with_connections([("p", vcustom), ("n", vss)])
                    .named("Vmeas")
                    .add_to(ctx);
                ctx.instantiate::<Resistor>(&crate::sim::dec((100) as f64 * 1e6))?
                    .with_connections([("p", vmeas), ("n", vcustom)])
                    .named("Vmeas")
                    .add_to(ctx);
            }
            _ => {}
        };

        let mut connections = Vec::new();

        for (k, nodes) in &self.params.connections {
            let mut signal = Vec::new();
            for node in nodes {
                signal.push(match node {
                    AcImpedanceTbNode::Vdd => vdd,
                    AcImpedanceTbNode::Vss => vss,
                    AcImpedanceTbNode::Vmeas => vmeas,
                    AcImpedanceTbNode::Vcustom(val) => {
                        ctr += 1;
                        let vcustom = ctx.signal(format!("custom{ctr}"));
                        ctx.instantiate::<Vdc>(val)?
                            .with_connections([("p", vcustom), ("n", vss)])
                            .named(format!("Vcustom{ctr}"))
                            .add_to(ctx);
                        vcustom
                    }
                    AcImpedanceTbNode::Floating => {
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

        ctx.instantiate::<Iac>(&crate::sim::dec(1.))?
            .with_connections([("p", vss), ("n", vmeas)])
            .named("iin")
            .add_to(ctx);

        Ok(())
    }
}

impl<
        T: crate::schematic::FromParams<Params: std::hash::Hash + Eq>
            + substrate::schematic::Schematic<Schema = sky130::Sky130>
            + crate::schematic::BuildIn<crate::sim::Simulator>,
    > Testbench for AcImpedanceTestbench<T>
{
    type Output = AcImpedance;

    fn setup(&self, ctx: &mut crate::sim::run::SimulationPlan) -> anyhow::Result<()> {
        if let Some(ref netlist) = self.params.pex_netlist {
            ctx.include(netlist);
        }
        ctx.add_analysis(Analysis::Ac(
            AcAnalysis::builder()
                .fstop(self.params.fstop)
                .fstart(self.params.fstart)
                .points(self.params.points)
                .sweep(SweepMode::Lin)
                .build()
                .unwrap(),
        ))
        .save(Save::Signals(["vmeas".into()].into_iter().collect()));
        Ok(())
    }

    fn measure(&self, ctx: &crate::sim::run::SimulationResults) -> anyhow::Result<Self::Output> {
        let data = ctx.data[0].ac();

        Ok(AcImpedance {
            vmeas: data.data["vmeas"].clone(),
            freq: data.freq.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::gate::{Inv, PrimitiveGateParams};
    use crate::schematic::FromParams;
    use TransitionTbNode as N;

    fn inverter_params() -> TransitionTbParams<PrimitiveGateParams> {
        TransitionTbParams {
            vdd: 1.8,
            delay: 1e-9,
            width: 2e-9,
            fall: 20e-12,
            rise: 20e-12,
            upper_threshold: 0.8,
            lower_threshold: 0.2,
            dut: PrimitiveGateParams {
                nwidth: 1000,
                pwidth: 2000,
                length: 150,
            },
            pex_netlist: None,
            connections: [
                ("vdd".into(), vec![N::Vdd]),
                ("vss".into(), vec![N::Vss]),
                ("a".into(), vec![N::Vstim]),
                ("y".into(), vec![N::Vmeas]),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn circuit_cache_uses_parameters_and_ignores_connection_insertion_order() {
        let ctx = crate::setup_ctx();
        let params = inverter_params();
        let first = ctx
            .generate_schematic(TransitionTestbench::<Inv>::from_params(&params).unwrap())
            .raw();
        let mut reordered = params.clone();
        reordered.connections = params
            .connections
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let second = ctx
            .generate_schematic(TransitionTestbench::<Inv>::from_params(&reordered).unwrap())
            .raw();
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        reordered.vdd = 1.7;
        let changed = ctx
            .generate_schematic(TransitionTestbench::<Inv>::from_params(&reordered).unwrap())
            .raw();
        assert!(!std::sync::Arc::ptr_eq(&first, &changed));
    }

    #[test]
    #[ignore = "requires the selected simulator and SKY130 models"]
    fn inverter_transition_measurement() {
        let dir = tempfile::tempdir().unwrap();
        let params = inverter_params();
        let out =
            crate::sim::run::<TransitionTestbench<Inv>>(&crate::setup_ctx(), &params, dir.path())
                .unwrap();
        assert!(out.tr > 1e-13 && out.tr < 1e-9, "{out:?}");
        assert!(out.tf > 1e-13 && out.tf < 1e-9, "{out:?}");
    }
}
