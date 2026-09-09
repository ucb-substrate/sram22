//! Open-PDK conversion preserving SRAM22's parallel transistor multiplicity.
//!
//! The upstream SKY130 importer combines SPICE `m` and `mult` into `MosParams::nf`.
//! The NDA exporter emits that count as `mult`, but its open exporter emits BSIM `nf`,
//! which divides the supplied width among fingers. SRAM22's imported macros specify
//! per-device widths, so the open models must receive the count as `m` instead.
use scir::schema::{FromSchema, Schema};
use std::convert::Infallible;

#[derive(Debug, Clone)]
pub struct OpenPdkSchema;
impl Schema for OpenPdkSchema {
    type Primitive = sky130::Primitive;
}
impl FromSchema<sky130::Sky130> for OpenPdkSchema {
    type Error = Infallible;
    fn convert_primitive(p: sky130::Primitive) -> Result<sky130::Primitive, Infallible> {
        Ok(p)
    }
    fn convert_instance(_: &mut scir::Instance, _: &sky130::Primitive) -> Result<(), Infallible> {
        Ok(())
    }
}
impl FromSchema<OpenPdkSchema> for spice::Spice {
    type Error = Infallible;
    fn convert_primitive(p: sky130::Primitive) -> Result<spice::Primitive, Infallible> {
        let is_mos = matches!(p, sky130::Primitive::Mos { .. });
        let mut p = <spice::Spice as FromSchema<sky130::Sky130OpenSchema>>::convert_primitive(p)?;
        if is_mos {
            if let spice::Primitive::RawInstance { params, .. } = &mut p {
                let nf = params.remove(&arcstr::literal!("nf").into()).unwrap();
                params.insert(arcstr::literal!("m").into(), nf);
            }
        }
        Ok(p)
    }
    fn convert_instance(i: &mut scir::Instance, p: &sky130::Primitive) -> Result<(), Infallible> {
        <spice::Spice as FromSchema<sky130::Sky130OpenSchema>>::convert_instance(i, p)
    }
}
impl FromSchema<OpenPdkSchema> for ngspice::Ngspice {
    type Error = Infallible;
    fn convert_primitive(p: sky130::Primitive) -> Result<ngspice::Primitive, Infallible> {
        Ok(ngspice::Primitive::Spice(<spice::Spice as FromSchema<
            OpenPdkSchema,
        >>::convert_primitive(p)?))
    }
    fn convert_instance(i: &mut scir::Instance, p: &sky130::Primitive) -> Result<(), Infallible> {
        <spice::Spice as FromSchema<OpenPdkSchema>>::convert_instance(i, p)
    }
}
#[cfg(feature = "spectre")]
impl FromSchema<OpenPdkSchema> for spectre::Spectre {
    type Error = Infallible;
    fn convert_primitive(p: sky130::Primitive) -> Result<spectre::Primitive, Infallible> {
        let is_mos = matches!(p, sky130::Primitive::Mos { .. });
        let mut p =
            <spectre::Spectre as FromSchema<sky130::Sky130OpenSchema>>::convert_primitive(p)?;
        if is_mos {
            if let spectre::Primitive::RawInstance { params, .. } = &mut p {
                for (name, _) in params {
                    if name == "nf" {
                        *name = "m".into();
                    }
                }
            }
        }
        Ok(p)
    }
    fn convert_instance(i: &mut scir::Instance, p: &sky130::Primitive) -> Result<(), Infallible> {
        <spectre::Spectre as FromSchema<sky130::Sky130OpenSchema>>::convert_instance(i, p)
    }
}
