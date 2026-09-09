use super::Precharge;

impl Precharge {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let length = self.params.length;

        let vdd = ctx.port("vdd", crate::schematic::Direction::InOut);
        let bl = ctx.port("bl", crate::schematic::Direction::InOut);
        let br = ctx.port("br", crate::schematic::Direction::InOut);
        let en_b = ctx.port("en_b", crate::schematic::Direction::Input);

        let mut bl_pull_up =
            ctx.instantiate::<sky130::mos::Pfet01v8>(&(self.params.pull_up_width, length))?;
        bl_pull_up.connect_all([("d", &bl), ("g", &en_b), ("s", &vdd), ("b", &vdd)]);
        bl_pull_up.set_name("bl_pull_up");
        ctx.add_instance(bl_pull_up);

        let mut br_pull_up =
            ctx.instantiate::<sky130::mos::Pfet01v8>(&(self.params.pull_up_width, length))?;
        br_pull_up.connect_all([("d", &br), ("g", &en_b), ("s", &vdd), ("b", &vdd)]);
        br_pull_up.set_name("br_pull_up");
        ctx.add_instance(br_pull_up);

        let mut equalizer =
            ctx.instantiate::<sky130::mos::Pfet01v8>(&(self.params.equalizer_width, length))?;
        equalizer.connect_all([("d", &bl), ("g", &en_b), ("s", &br), ("b", &vdd)]);
        equalizer.set_name("equalizer");
        ctx.add_instance(equalizer);

        Ok(())
    }
}
