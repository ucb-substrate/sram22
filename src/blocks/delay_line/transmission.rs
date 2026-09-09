use crate::blocks::gate::PrimitiveGateParams;

#[derive(Hash, PartialEq, Eq)]
pub struct TransmissionGate {
    params: PrimitiveGateParams,
}

impl crate::schematic::FromParams for TransmissionGate {
    type Params = PrimitiveGateParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self { params: *params })
    }
}
impl substrate::block::Block for TransmissionGate {
    type Io = crate::schematic::NamedIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::format!("transmission_gate")
    }
    fn io(&self) -> Self::Io {
        crate::schematic::NamedIo::new([
            ("din", 1, crate::schematic::Direction::Input),
            ("en", 1, crate::schematic::Direction::Input),
            ("en_b", 1, crate::schematic::Direction::Input),
            ("dout", 1, crate::schematic::Direction::Output),
            ("vdd", 1, crate::schematic::Direction::InOut),
            ("vss", 1, crate::schematic::Direction::InOut),
        ])
    }
}
impl substrate::schematic::Schematic for TransmissionGate {
    type Schema = sky130::Sky130;
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
impl TransmissionGate {
    fn build_schematic(&self, ctx: &mut crate::schematic::CircuitBuilder) -> anyhow::Result<()> {
        let [din, en, en_b] = ctx.ports(["din", "en", "en_b"], crate::schematic::Direction::Input);
        let dout = ctx.port("dout", crate::schematic::Direction::Output);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], crate::schematic::Direction::InOut);

        ctx.instantiate::<sky130::mos::Nfet01v8>(&(self.params.nwidth, self.params.length))?
            .named("npass")
            .with_connections([("d", din), ("g", en), ("s", dout), ("b", vss)])
            .add_to(ctx);

        ctx.instantiate::<sky130::mos::Pfet01v8>(&(self.params.pwidth, self.params.length))?
            .named("ppass")
            .with_connections([("d", dout), ("g", en_b), ("s", din), ("b", vdd)])
            .add_to(ctx);

        Ok(())
    }
}
crate::impl_sky130_build!(TransmissionGate);
