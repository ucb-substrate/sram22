use crate::sim::waveform::{DigitalTrace as Waveform, DigitalWaveform};
use std::sync::Arc;

use super::{Decoder, DecoderParams, DecoderPhysicalDesignParams, DecoderStyle, DecoderTree};
use crate::blocks::sram::WORDLINE_CAP_PER_CELL;
use serde::{Deserialize, Serialize};
use subgeom::Dir;

use crate::sim::blocks::Vdc;
use crate::sim::blocks::Vpwl;
use crate::sim::run::SimulationTestbench as Testbench;

use crate::sim::run::TranAnalysis;

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DecoderCriticalPathTbParams {
    bits: usize,
    scale: i64,
    vdd: f64,
    period: f64,
    tr: f64,
    tf: f64,
}

pub struct DecoderCriticalPathTb {
    params: DecoderCriticalPathTbParams,
}

impl DecoderCriticalPathTb {
    // Float bit patterns give block identity a reflexive Eq and matching Hash, including NaNs.
    fn cache_key(&self) -> impl std::hash::Hash + Eq + '_ {
        let p = &self.params;
        (
            [
                p.vdd.to_bits(),
                p.period.to_bits(),
                p.tr.to_bits(),
                p.tf.to_bits(),
            ],
            &p.bits,
            &p.scale,
        )
    }
}

impl std::hash::Hash for DecoderCriticalPathTb {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash(&self.cache_key(), h)
    }
}
impl PartialEq for DecoderCriticalPathTb {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key() == other.cache_key()
    }
}
impl Eq for DecoderCriticalPathTb {}
impl crate::schematic::FromParams for DecoderCriticalPathTb {
    type Params = DecoderCriticalPathTbParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self { params: *params })
    }
}
impl substrate::block::Block for DecoderCriticalPathTb {
    type Io = substrate::types::TestbenchIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("decodercriticalpathtb")
    }
    fn io(&self) -> Self::Io {
        Default::default()
    }
}
impl substrate::schematic::Schematic for DecoderCriticalPathTb {
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
impl DecoderCriticalPathTb {
    fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder<crate::sim::Simulator>,
    ) -> anyhow::Result<()> {
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let vdd = ctx.signal("vdd");

        let params = &self.params;
        let addr = ctx.bus("addr", params.bits);
        let addr_b = ctx.bus("addr_b", params.bits);
        let decode = ctx.bus("decode", 2usize.pow(params.bits as u32));
        let decode_b = ctx.bus("decode_b", 2usize.pow(params.bits as u32));

        let vsupply = crate::sim::dec(params.vdd);
        ctx.instantiate::<Vdc>(&vsupply)?
            .named("Vdd")
            .with_connections([("p", vdd), ("n", vss)])
            .add_to(ctx);

        let tree = DecoderTree::new(params.bits, 64. * WORDLINE_CAP_PER_CELL);
        let decoder_params = DecoderParams {
            pd: DecoderPhysicalDesignParams {
                style: DecoderStyle::RowMatched,
                dir: Dir::Horiz,
            },
            max_width: None,
            tree,
            use_multi_finger_invs: true,
        };
        let mut dut = ctx
            .instantiate::<Decoder>(&decoder_params)?
            .with_connections([("vdd", vdd), ("vss", vss), ("y", decode)])
            .named("dut");
        if dut.port("y_b").is_ok() {
            dut.connect("y_b", decode_b);
        }
        for i in 0..params.bits {
            dut.connect(format!("predecode_{i}_0"), addr_b.index(i));
            dut.connect(format!("predecode_{i}_1"), addr.index(i));
        }
        dut.add_to(ctx);

        let waveforms = self.waveforms();

        for i in 0..self.params.bits {
            ctx.instantiate::<Vpwl>(&waveforms.addr[i])?
                .named(format!("Vaddr[{i}]"))
                .with_connections([("p", addr.index(i)), ("n", vss)])
                .add_to(ctx);
            ctx.instantiate::<Vpwl>(&waveforms.addr_b[i])?
                .named(format!("Vaddr_b[{i}]"))
                .with_connections([("p", addr_b.index(i)), ("n", vss)])
                .add_to(ctx);
        }
        Ok(())
    }
}

impl Testbench for DecoderCriticalPathTb {
    type Output = ();
    fn setup(&self, ctx: &mut crate::sim::run::SimulationPlan) -> anyhow::Result<()> {
        let tran = TranAnalysis::builder()
            .start(0.0)
            .stop(self.t_stop())
            .step(self.params.period / 50.0)
            .build()
            .unwrap();
        ctx.add_analysis(tran);
        Ok(())
    }

    fn measure(&self, _ctx: &crate::sim::run::SimulationResults) -> anyhow::Result<Self::Output> {
        Ok(())
    }
}

struct Waveforms {
    addr_b: Vec<Arc<Waveform>>,
    addr: Vec<Arc<Waveform>>,
}

impl DecoderCriticalPathTb {
    fn waveforms(&self) -> Waveforms {
        let params = &self.params;
        let n = params.bits;

        let mut out = Waveforms {
            addr: Vec::with_capacity(n),
            addr_b: Vec::with_capacity(n),
        };

        let t_stop = self.t_stop();
        for i in 0..n {
            let mut addr = Waveform::with_initial_value(0.0);
            let mut addr_b = Waveform::with_initial_value(params.vdd);

            let t_start = params.period / 4.0 + i as f64 * params.period;
            let t_end = t_start + params.period / 2.0;

            addr.push_low(t_start, params.vdd, params.tf);
            addr.push_high(t_end, params.vdd, params.tr);
            addr.push_low(t_stop, params.vdd, params.tf);

            addr_b.push_high(t_start, params.vdd, params.tr);
            addr_b.push_low(t_end, params.vdd, params.tf);
            addr_b.push_high(t_stop, params.vdd, params.tr);

            out.addr.push(Arc::new(addr));
            out.addr_b.push(Arc::new(addr_b));
        }

        assert_eq!(out.addr.len(), n);
        assert_eq!(out.addr_b.len(), n);

        out
    }

    #[inline]
    fn t_stop(&self) -> f64 {
        self.params.period * self.params.bits as f64
    }
}

#[cfg(test)]
mod tests {

    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    #[ignore = "slow"]
    fn test_decoder_critical_path_5bit() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_decoder_critical_path_5bit");

        let params = DecoderCriticalPathTbParams {
            bits: 5,
            scale: 1,
            vdd: 1.8,
            period: 20e-9,
            tr: 5e-12,
            tf: 5e-12,
        };

        crate::sim::run::<DecoderCriticalPathTb>(&ctx, &params, &work_dir)
            .expect("failed to run simulation");
    }
}
