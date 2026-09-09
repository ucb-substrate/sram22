#![allow(unexpected_cfgs)]

use std::path::PathBuf;

pub use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use sky130::Sky130;
#[cfg(feature = "commercial")]
use sky130_commercial_pdk::Sky130CommercialPdk;
#[cfg(not(feature = "commercial"))]
use sky130_open_pdk::Sky130OpenPdk;
use substrate::context::Context;
use substrate1::data::{SubstrateConfig, SubstrateCtx};
#[cfg(not(feature = "commercial"))]
use substrate1::pdk::PdkParams;
use tera::Tera;

pub mod abs;
pub mod bits;
pub mod blocks;
pub mod cli;
#[cfg(feature = "commercial")]
pub mod liberate;
pub mod liberty;
pub mod logic;
pub mod measure;
pub mod netlist;
pub mod paths;
pub mod pex;
pub mod plan;
pub mod schematic;
pub mod script;
pub mod sim;
pub mod tech;
pub mod verification;
pub mod verilog;

pub const BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");
pub const LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lib");
pub const SKY130_OPEN_PDK_ROOT: &str = env!("SKY130_OPEN_PDK_ROOT");
#[cfg(feature = "commercial")]
pub const SKY130_COMMERCIAL_PDK_ROOT: &str = env!("SKY130_COMMERCIAL_PDK_ROOT");

lazy_static! {
    pub static ref TEMPLATES: Tera =
        match Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/*")) {
            Ok(t) => t,
            Err(e) => panic!("Error parsing templates: {e}"),
        };
}

pub fn bus_bit(name: &str, index: usize) -> String {
    format!("{name}[{index}]")
}

thread_local! {
    static LAYOUT_DESIGN_CONTEXTS: std::cell::RefCell<Vec<(SubstrateCtx, Context)>> = const { std::cell::RefCell::new(Vec::new()) };
    static LAYOUT_CONTEXT: std::cell::RefCell<Option<SubstrateCtx>> = const { std::cell::RefCell::new(None) };
}

/// Runs a design script against the layout context currently generating geometry.
pub(crate) fn with_layout_context<T>(layout: &SubstrateCtx, f: impl FnOnce(&Context) -> T) -> T {
    struct Restore(Option<SubstrateCtx>);
    impl Drop for Restore {
        fn drop(&mut self) {
            LAYOUT_CONTEXT.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let previous = LAYOUT_CONTEXT.with(|slot| slot.replace(Some(layout.clone())));
    let _restore = Restore(previous);
    let design = LAYOUT_DESIGN_CONTEXTS.with(|contexts| {
        let mut contexts = contexts.borrow_mut();
        if let Some((_, design)) = contexts
            .iter()
            .find(|(other, _)| std::sync::Arc::ptr_eq(&other.pdk(), &layout.pdk()))
        {
            return design.clone();
        }
        let design = setup_ctx();
        contexts.push((layout.clone(), design.clone()));
        design
    });
    f(&design)
}

/// Creates the Substrate 1 context used only for layout generation.
pub fn setup_layout_ctx() -> SubstrateCtx {
    let mut builder = SubstrateConfig::builder();

    #[cfg(feature = "commercial")]
    let builder = builder.pdk(
        Sky130CommercialPdk::new(
            PathBuf::from(SKY130_COMMERCIAL_PDK_ROOT),
            PathBuf::from(SKY130_OPEN_PDK_ROOT),
        )
        .unwrap(),
    );
    #[cfg(not(feature = "commercial"))]
    let builder = builder.pdk(
        Sky130OpenPdk::new(&PdkParams {
            pdk_root: PathBuf::from(SKY130_OPEN_PDK_ROOT),
        })
        .unwrap(),
    );

    let cfg = builder.build();

    SubstrateCtx::from_config(cfg).unwrap()
}

/// Creates the Substrate 2 context used for schematic generation, netlisting, and simulation.
///
/// Legacy layout generation uses a separate thread-local context.
pub fn setup_ctx() -> Context {
    let mut builder = Context::builder();
    #[cfg(not(feature = "spectre"))]
    {
        builder.install(ngspice::Ngspice::default());
        builder.install(crate::sim::ac::NgspiceAc);
    }
    #[cfg(feature = "spectre")]
    builder.install(spectre::Spectre::default());

    #[cfg(not(feature = "commercial"))]
    {
        builder.install(Sky130::open(SKY130_OPEN_PDK_ROOT));
    }

    #[cfg(feature = "commercial")]
    {
        builder.install(Sky130::src_nda(
            SKY130_OPEN_PDK_ROOT,
            SKY130_COMMERCIAL_PDK_ROOT,
        ));
    }

    builder.build()
}

/// Returns this thread's layout context. Legacy layout state never crosses threads.
pub fn layout_ctx() -> SubstrateCtx {
    LAYOUT_CONTEXT.with(|slot| {
        if slot.borrow().is_none() {
            *slot.borrow_mut() = Some(setup_layout_ctx());
        }
        slot.borrow().as_ref().unwrap().clone()
    })
}

#[cfg(test)]
pub mod tests {

    use std::path::PathBuf;

    use super::BUILD_PATH;

    pub(crate) fn test_work_dir(name: &str) -> PathBuf {
        PathBuf::from(BUILD_PATH).join(name)
    }
}
