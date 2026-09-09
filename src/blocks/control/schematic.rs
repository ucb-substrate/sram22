use crate::blocks::macros::{SvtInv2, SvtInv4};

use super::{ControlLogicReplicaV2, EdgeDetector, InvChain, SrLatch, SvtInvChain};

impl ControlLogicReplicaV2 {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        // PORTS
        let [clk, ce, we, rstb, rbl] = ctx.ports(
            ["clk", "ce", "we", "rstb", "rbl"],
            crate::schematic::Direction::Input,
        );
        let [saen, pc_b, rwl, wlen, wrdrven] = ctx.ports(
            ["saen", "pc_b", "rwl", "wlen", "wrdrven"],
            crate::schematic::Direction::Output,
        );
        let [vdd, vss] = ctx.ports(["vdd", "vss"], crate::schematic::Direction::InOut);

        // SIGNALS
        let [clkd, clk_buf, clkp0, clkp, clkp_b, clkpd, clkpd_b, clkpdd, clkp_grst_b] = ctx
            .signals([
                "clkd",
                "clk_buf",
                "clkp0",
                "clkp",
                "clkp_b",
                "clkpd",
                "clkpd_b",
                "clkpdd",
                "clkp_grst_b",
            ]);
        let [decrepstart, decrepend] = ctx.signals(["decrepstart", "decrepend"]);
        let [wlen_grst_b, wlen_rst_decoderd, wlen_b, wlen_q, wlend] = ctx.signals([
            "wlen_grst_b",
            "wlen_rst_decoderd",
            "wlen_b",
            "wlen_q",
            "wlend",
        ]);
        let [saen_set_b, saen_b] = ctx.signals(["saen_set_b", "saen_b"]);
        let [wrdrven_set_b0, wrdrven_set_b, wrdrven_grst_b, wrdrven_b] = ctx.signals([
            "wrdrven_set_b0",
            "wrdrven_set_b",
            "wrdrven_grst_b",
            "wrdrven_b",
        ]);
        let [reset, we_b, pc, pc_set_b, pc_b0, rbl_b] =
            ctx.signals(["reset", "we_b", "pc", "pc_set_b", "pc_b0", "rbl_b"]);

        // STANDARD CELLS

        ctx.instantiate::<crate::blocks::stdcells::HsInv>(&16)?
            .with_connections([
                ("A", rstb),
                ("Y", reset),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("reset_inv")
            .add_to(ctx);

        // CLK LOGIC
        ctx.instantiate::<InvChain>(&12)?
            .with_connections([("din", clk), ("dout", clkd), ("vdd", vdd), ("vss", vss)])
            .named("clk_delay")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsAnd2>(&4)?
            .with_connections([
                ("A", clkd),
                ("B", ce),
                ("X", clk_buf),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("clk_gate")
            .add_to(ctx);
        ctx.instantiate::<EdgeDetector>(&crate::schematic::NoParams)?
            .with_connections([
                ("din", clk_buf),
                ("dout", clkp0),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("clk_pulse")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsBuf>(&16)?
            .with_connections([
                ("A", clkp0),
                ("X", clkp),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("clk_pulse_buf")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsInv>(&16)?
            .with_connections([
                ("A", clkp),
                ("Y", clkp_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("clk_pulse_inv")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&3)?
            .with_connections([("din", clkp_b), ("dout", clkpd), ("vdd", vdd), ("vss", vss)])
            .named("clkp_delay")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsInv>(&2)?
            .with_connections([
                ("A", clkpd),
                ("Y", clkpd_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("clkpd_inv")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&self.params.wlen_pulse_invs)?
            .with_connections([
                ("din", clkpd_b),
                ("dout", clkpdd),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("clkpd_delay")
            .add_to(ctx);

        // REPLICA LOGIC
        //
        // Turn on wordlines at start of cycle.
        // Turn them off when replica bitline drops low enough to flip an inverter.
        ctx.instantiate::<crate::blocks::stdcells::HsMux2>(&4)?
            .with_connections([
                ("A0", rbl_b),
                ("A1", clkpdd),
                ("S", we),
                ("X", decrepstart),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("mux_wlen_rst")
            .add_to(ctx);
        ctx.instantiate::<SvtInvChain>(&self.params.decoder_delay_invs)?
            .with_connections([
                ("din", decrepstart),
                ("dout", decrepend),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("decoder_replica")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&self.params.pc_set_delay_invs)?
            .with_connections([
                ("din", decrepend),
                ("dout", wlen_rst_decoderd),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("decoder_replica_delay")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsInv>(&2)?
            .with_connections([
                ("A", we),
                ("Y", we_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("inv_we")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsInv>(&2)?
            .with_connections([
                ("A", rbl),
                ("Y", rbl_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("inv_rbl")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsNor2>(&4)?
            .with_connections([
                ("A", decrepstart),
                ("B", reset),
                ("Y", wlen_grst_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("wlen_grst")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsNor2>(&4)?
            .with_connections([
                ("A", wlen_rst_decoderd),
                ("B", reset),
                ("Y", pc_set_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("pc_set")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsNor2>(&4)?
            .with_connections([
                ("A", decrepend),
                ("B", reset),
                ("Y", wrdrven_grst_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("wrdrven_grst")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsNor2>(&4)?
            .with_connections([
                ("A", clkp),
                ("B", reset),
                ("Y", clkp_grst_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("clkp_grst")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsNand2>(&8)?
            .with_connections([
                ("A", we_b),
                ("B", decrepend),
                ("Y", saen_set_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("nand_sense_en")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsNand2>(&8)?
            .with_connections([
                ("A", rbl_b),
                ("B", we_b),
                ("Y", wlend),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("nand_wlendb_web")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsAnd2>(&4)?
            .with_connections([
                ("A", wlen_q),
                ("B", wlend),
                ("X", wlen),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("and_wlen")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsBuf>(&16)?
            .with_connections([
                ("A", wlen_q),
                ("X", rwl),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("rwl_buf")
            .add_to(ctx);

        // CONTROL LATCHES
        ctx.instantiate::<SrLatch>(&crate::schematic::NoParams)?
            .with_connections([
                ("sb", clkpd_b),
                ("rb", wlen_grst_b),
                ("q", wlen_q),
                ("qb", wlen_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wl_ctl")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&crate::schematic::NoParams)?
            .with_connections([
                ("sb", saen_set_b),
                ("rb", clkp_grst_b),
                ("q", saen),
                ("qb", saen_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("saen_ctl")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&crate::schematic::NoParams)?
            .with_connections([
                ("sb", pc_set_b),
                ("rb", clkp_b),
                ("q", pc),
                ("qb", pc_b0),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("pc_ctl")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsBuf>(&16)?
            .with_connections([
                ("A", pc_b0),
                ("X", pc_b),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("pc_b_buf")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsNand2>(&8)?
            .with_connections([
                ("A", clkpd),
                ("B", we),
                ("Y", wrdrven_set_b0),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("wrdrven_set")
            .add_to(ctx);
        ctx.instantiate::<InvChain>(&self.params.wrdrven_set_delay_invs)?
            .with_connections([
                ("din", wrdrven_set_b0),
                ("dout", wrdrven_set_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wrdrven_set_delay")
            .add_to(ctx);
        ctx.instantiate::<SrLatch>(&crate::schematic::NoParams)?
            .with_connections([
                ("sb", wrdrven_set_b),
                ("rb", wrdrven_grst_b),
                ("q", wrdrven),
                ("qb", wrdrven_b),
                ("vdd", vdd),
                ("vss", vss),
            ])
            .named("wrdrven_ctl")
            .add_to(ctx);

        Ok(())
    }
}

impl SrLatch {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let [sb, rb] = ctx.ports(["sb", "rb"], crate::schematic::Direction::Input);
        let [q, qb] = ctx.ports(["q", "qb"], crate::schematic::Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], crate::schematic::Direction::InOut);

        let [q0, q0b] = ctx.signals(["q0", "q0b"]);

        let mut nand_set = ctx.instantiate::<crate::blocks::stdcells::HsNand2>(&8)?;
        let mut nand_reset = nand_set.clone();

        nand_set.connect_all([
            ("A", q0b),
            ("B", sb),
            ("Y", q0),
            ("pwr_vpwr", vdd),
            ("pwr_vpb", vdd),
            ("pwr_vgnd", vss),
            ("pwr_vnb", vss),
        ]);
        nand_set.set_name("nand_set");
        ctx.add_instance(nand_set);

        nand_reset.connect_all([
            ("A", q0),
            ("B", rb),
            ("Y", q0b),
            ("pwr_vpwr", vdd),
            ("pwr_vpb", vdd),
            ("pwr_vgnd", vss),
            ("pwr_vnb", vss),
        ]);
        nand_reset.set_name("nand_reset");
        ctx.add_instance(nand_reset);

        ctx.instantiate::<crate::blocks::stdcells::HsInv>(&2)?
            .with_connections([
                ("A", q0),
                ("Y", qb),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("qb_inv")
            .add_to(ctx);
        ctx.instantiate::<crate::blocks::stdcells::HsInv>(&2)?
            .with_connections([
                ("A", q0b),
                ("Y", q),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("q_inv")
            .add_to(ctx);

        Ok(())
    }
}

impl InvChain {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let din = ctx.port("din", crate::schematic::Direction::Input);
        let dout = ctx.port("dout", crate::schematic::Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], crate::schematic::Direction::InOut);
        let x = ctx.bus("x", self.n - 1);

        for i in 0..self.n {
            ctx.instantiate::<crate::blocks::stdcells::HsInv>(&if i == self.n - 1 {
                4
            } else {
                2
            })?
            .with_connections([
                ("A", if i == 0 { din } else { x.index(i - 1) }),
                ("Y", if i == self.n - 1 { dout } else { x.index(i) }),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named(format!("inv{i}"))
            .add_to(ctx);
        }
        Ok(())
    }
}

impl SvtInvChain {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let din = ctx.port("din", crate::schematic::Direction::Input);
        let dout = ctx.port("dout", crate::schematic::Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], crate::schematic::Direction::InOut);
        let x = ctx.bus("x", self.n - 1);

        for i in 0..self.n {
            if i == self.n - 1 {
                ctx.instantiate::<SvtInv4>(&crate::schematic::NoParams)?
            } else {
                ctx.instantiate::<SvtInv2>(&crate::schematic::NoParams)?
            }
            .with_connections([
                ("A", if i == 0 { din } else { x.index(i - 1) }),
                ("Y", if i == self.n - 1 { dout } else { x.index(i) }),
                ("vpwr", vdd),
                ("vpb", vdd),
                ("vgnd", vss),
                ("vnb", vss),
            ])
            .named(format!("inv{i}"))
            .add_to(ctx);
        }
        Ok(())
    }
}

impl EdgeDetector {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let din = ctx.port("din", crate::schematic::Direction::Input);
        let dout = ctx.port("dout", crate::schematic::Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], crate::schematic::Direction::InOut);
        let delayed = ctx.signal("delayed");

        ctx.instantiate::<InvChain>(&self.invs)?
            .with_connections([("din", din), ("dout", delayed), ("vdd", vdd), ("vss", vss)])
            .named("delay_chain")
            .add_to(ctx);

        ctx.instantiate::<crate::blocks::stdcells::HsAnd2>(&4)?
            .with_connections([
                ("A", din),
                ("B", delayed),
                ("X", dout),
                ("pwr_vpwr", vdd),
                ("pwr_vpb", vdd),
                ("pwr_vgnd", vss),
                ("pwr_vnb", vss),
            ])
            .named("and")
            .add_to(ctx);
        Ok(())
    }
}
