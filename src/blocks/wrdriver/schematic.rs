use crate::blocks::delay_line::tristate::TristateInv;
use crate::blocks::gate::PrimitiveGateParams;

use super::WriteDriver;

impl WriteDriver {
    pub(crate) fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder,
    ) -> anyhow::Result<()> {
        let en = ctx.port("en", crate::schematic::Direction::Input);
        let en_b = ctx.port("en_b", crate::schematic::Direction::Input);
        let data = ctx.port("data", crate::schematic::Direction::Input);
        let data_b = ctx.port("data_b", crate::schematic::Direction::Input);
        let bl = ctx.port("bl", crate::schematic::Direction::InOut);
        let br = ctx.port("br", crate::schematic::Direction::InOut);
        let vdd = ctx.port("vdd", crate::schematic::Direction::InOut);
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);

        ctx.instantiate::<TristateInv>(&PrimitiveGateParams {
            pwidth: self.params.pwidth_driver,
            nwidth: self.params.nwidth_driver,
            length: self.params.length,
        })?
        .with_connections([
            ("vdd", vdd),
            ("din", data_b),
            ("en", en),
            ("en_b", en_b),
            ("din_b", bl),
            ("vss", vss),
        ])
        .named("bldriver")
        .add_to(ctx);

        ctx.instantiate::<TristateInv>(&PrimitiveGateParams {
            pwidth: self.params.pwidth_driver,
            nwidth: self.params.nwidth_driver,
            length: self.params.length,
        })?
        .with_connections([
            ("vdd", vdd),
            ("din", data),
            ("en", en),
            ("en_b", en_b),
            ("din_b", br),
            ("vss", vss),
        ])
        .named("brdriver")
        .add_to(ctx);

        Ok(())
    }
}
