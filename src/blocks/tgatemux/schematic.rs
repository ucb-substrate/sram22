use super::*;

impl TGateMux {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let length = self.params.length;

        let sel_b = ctx.port("sel_b", crate::schematic::Direction::Input);
        let sel = ctx.port("sel", crate::schematic::Direction::Input);
        let bl = ctx.port("bl", crate::schematic::Direction::InOut);
        let br = ctx.port("br", crate::schematic::Direction::InOut);
        let bl_out = ctx.port("bl_out", crate::schematic::Direction::InOut);
        let br_out = ctx.port("br_out", crate::schematic::Direction::InOut);
        let vdd = ctx.port("vdd", crate::schematic::Direction::InOut);
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);

        let mut mpbl = ctx.instantiate::<sky130::mos::Pfet01v8>(&(self.params.pwidth, length))?;
        mpbl.connect_all([("d", &bl_out), ("g", &sel_b), ("s", &bl), ("b", &vdd)]);
        mpbl.set_name("MPBL");
        ctx.add_instance(mpbl);

        let mut mpbr = ctx.instantiate::<sky130::mos::Pfet01v8>(&(self.params.pwidth, length))?;
        mpbr.connect_all([("d", &br_out), ("g", &sel_b), ("s", &br), ("b", &vdd)]);
        mpbr.set_name("MPBR");
        ctx.add_instance(mpbr);

        let mut mnbl = ctx.instantiate::<sky130::mos::Nfet01v8>(&(self.params.nwidth, length))?;
        mnbl.connect_all([("d", &bl_out), ("g", &sel), ("s", &bl), ("b", &vss)]);
        mnbl.set_name("MNBL");
        ctx.add_instance(mnbl);

        let mut mnbr = ctx.instantiate::<sky130::mos::Nfet01v8>(&(self.params.nwidth, length))?;
        mnbr.connect_all([("d", &br_out), ("g", &sel), ("s", &br), ("b", &vss)]);
        mnbr.set_name("MNBR");
        ctx.add_instance(mnbr);

        Ok(())
    }
}
