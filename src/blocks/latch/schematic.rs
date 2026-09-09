use crate::blocks::gate::FoldedInv;
use anyhow::Result;

use super::DiffLatch;

impl DiffLatch {
    pub(crate) fn build_schematic(&self, ctx: &mut crate::schematic::CircuitBuilder) -> Result<()> {
        let vdd = ctx.port("vdd", crate::schematic::Direction::InOut);
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let din1 = ctx.port("din1", crate::schematic::Direction::Input);
        let din2 = ctx.port("din2", crate::schematic::Direction::Input);
        let dout1 = ctx.port("dout1", crate::schematic::Direction::Output);
        let dout2 = ctx.port("dout2", crate::schematic::Direction::Output);

        let [rst, set, q, qb] = ctx.signals(["rst", "set", "q", "qb"]);
        for (din, dout, suffix) in [(&din1, &rst, "1"), (&din2, &set, "2")] {
            let mut buf = ctx.instantiate::<FoldedInv>(&self.params.inv_in)?;
            buf.connect_all([("vdd", &vdd), ("vss", &vss), ("a", din), ("y", dout)]);
            buf.set_name(format!("inbuf_{suffix}"));
            ctx.add_instance(buf);
        }

        for (din, dout, suffix) in [(&q, &dout2, "1"), (&qb, &dout1, "2")] {
            let mut buf = ctx.instantiate::<FoldedInv>(&self.params.inv_out)?;
            buf.connect_all([("vdd", &vdd), ("vss", &vss), ("a", din), ("y", dout)]);
            buf.set_name(format!("outbuf_{suffix}"));
            ctx.add_instance(buf);
        }
        for (din, dout, suffix) in [(&q, &qb, "1"), (&qb, &q, "2")] {
            let mut buf = ctx.instantiate::<FoldedInv>(&self.params.invq)?;
            buf.connect_all([("vdd", &vdd), ("vss", &vss), ("a", din), ("y", dout)]);
            buf.set_name(format!("invq_{suffix}"));
            ctx.add_instance(buf);
        }

        for (d, g, suffix) in [(&q, &rst, "1"), (&qb, &set, "2")] {
            for i in 0..2 {
                let mut mn = ctx.instantiate::<sky130::mos::Nfet01v8>(&(
                    self.params.nwidth / 2,
                    self.params.lch,
                ))?;
                mn.connect_all([("d", d), ("g", g), ("s", &vss), ("b", &vss)]);
                mn.set_name(format!("MN{suffix}{i}"));
                ctx.add_instance(mn);
            }
        }

        Ok(())
    }
}
