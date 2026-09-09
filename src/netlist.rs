//! SPICE netlist export and extracted-netlist wrappers.

use std::path::{Path, PathBuf};

use arcstr::ArcStr;
use sky130::Sky130;
use spice::netlist::NetlistOptions;
use spice::{Primitive, Spice};
use substrate::block::Block;
use substrate::context::Context;
use substrate::error::Result;
use substrate::schematic::{CellBuilder, PrimitiveBinding, Schematic};
use substrate::types::schematic::IoNodeBundle;
use substrate::types::{Flatten, HasBundleKind, HasNameTree, NameBuf, NameFragment};

use crate::sim::PdkSchema;

/// Constructs and exports a circuit with pin labels matching the layout.
pub fn write_schematic<B: Schematic<Schema = Sky130> + crate::schematic::FromParams>(
    ctx: &Context,
    params: &B::Params,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    write_verification_netlist(ctx, B::from_params(params)?, path)?;
    Ok(())
}

/// Writes the SPICE netlist of `block` to `path`.
///
/// Bus ports are named `bus_0`, `bus_1`, ... following Substrate conventions.
pub fn write_netlist<B: Schematic<Schema = Sky130>>(
    ctx: &Context,
    block: B,
    path: impl AsRef<Path>,
) -> Result<()> {
    write_library(&spice_library(ctx, block)?, path.as_ref())
}

fn spice_library<B: Schematic<Schema = Sky130>>(
    ctx: &Context,
    block: B,
) -> Result<scir::Library<Spice>> {
    // Convert the generated library itself so conversion does not insert wrapper cells.
    let lib = ctx.export_scir(block)?;
    let lib = lib
        .scir
        .convert_schema::<PdkSchema>()
        .map_err(|_| substrate::error::Error::UnsupportedPrimitive)?
        .build()
        .map_err(substrate::schematic::conv::ConvError::from)?;
    let lib = lib
        .convert_schema::<Spice>()
        .map_err(|_| substrate::error::Error::UnsupportedPrimitive)?
        .build()
        .map_err(substrate::schematic::conv::ConvError::from)?;
    Ok(lib)
}

fn write_library(lib: &scir::Library<Spice>, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(std::sync::Arc::new)?;
    }
    let mut file = std::fs::File::create(path).map_err(std::sync::Arc::new)?;
    spice::netlist::NetlisterInstance::new(&Spice, lib, &mut file, NetlistOptions::default())
        .export()?;
    Ok(())
}

/// Renders a flattened node name using SPICE-style bus bit names (`bus[0]`).
///
/// This is the convention used by the SRAM22 layouts, LEF, and Verilog models.
pub fn bracket_name(name: &NameBuf) -> String {
    let mut name = name.clone();
    let mut fragments = Vec::new();
    while let Some(f) = name.pop() {
        fragments.push(f);
    }
    fragments.reverse();
    let mut out = String::new();
    for (i, f) in fragments.iter().enumerate() {
        match f {
            NameFragment::Str(s) => {
                if i > 0 {
                    out.push('_');
                }
                out.push_str(s);
            }
            NameFragment::Idx(idx) => {
                out.push_str(&format!("[{idx}]"));
            }
        }
    }
    out
}

/// Returns the flattened native port names and the corresponding layout labels.
/// Buses use brackets; gate input arrays and standard-cell power bundles use their
/// a/b/c/d and supply pin labels.
pub fn port_names<B: Block>(block: &B) -> Vec<(String, String)> {
    block
        .io()
        .kind()
        .flat_names(None)
        .into_iter()
        .map(|n| {
            let native = n.to_string();
            let physical = if let Some(i) = native
                .strip_prefix("inputs_")
                .and_then(|s| s.parse::<usize>().ok())
            {
                // The parameterized gate uses an input array; layout labels use a/b/c/d.
                ["a", "b", "c", "d"]
                    .get(i)
                    .map(|s| (*s).to_owned())
                    .unwrap_or_else(|| bracket_name(&n))
            } else if native == "yb_0" {
                "yb".to_owned()
            } else if let Some(power) = native.strip_prefix("pwr_") {
                power.to_owned()
            } else {
                bracket_name(&n)
            };
            (native, physical)
        })
        .collect()
}

/// Writes the SPICE netlist of `block` to `path` for physical verification.
///
/// The top-level cell's bus ports are renamed from Substrate's `bus_0` convention to the
/// `bus[0]` convention used by the SRAM22 layout pin labels so that LVS and PEX can match
/// the netlist against the layout.
pub fn write_verification_netlist<B: Schematic<Schema = Sky130>>(
    ctx: &Context,
    block: B,
    path: impl AsRef<Path>,
) -> Result<()> {
    let ports = port_names(&block);
    let lib = rename_top_ports(spice_library(ctx, block)?, &ports)?;
    write_library(&lib, path.as_ref())
}

/// Rename only the top cell's signal objects. Editing the SCIR keeps instance names,
/// child pin names, model names and parameter values independent of layout pin labels.
fn rename_top_ports(
    lib: scir::Library<Spice>,
    names: &[(String, String)],
) -> Result<scir::Library<Spice>> {
    use scir::IndexOwned;
    let id = lib.top_cell().expect("generated library has a top cell");
    let original = lib.cell(id);
    let mut cell = scir::Cell::new(original.name());
    let mut signals = std::collections::HashMap::new();
    // Keep allocation deterministic despite SCIR storing signals in a HashMap.
    let mut ordered: Vec<_> = original.signals().collect();
    ordered.sort_by_key(|(id, _)| *id);
    for (id, signal) in ordered {
        let name = names
            .iter()
            .find(|(from, _)| signal.is_port() && from == signal.name.as_str())
            .map(|(_, to)| to.as_str())
            .unwrap_or(&signal.name);
        let new = match signal.width {
            Some(width) => cell.add_bus(name, width),
            None => cell.add_node(name).into(),
        };
        signals.insert(id, new);
    }
    for port in original.ports() {
        cell.expose_port(signals[&port.signal()].signal(), port.direction());
    }
    for (_, original) in original.instances() {
        let mut instance = original.clone();
        for connection in instance.connections_mut().values_mut() {
            *connection = scir::Concat::new(
                connection
                    .parts()
                    .map(|part| {
                        let signal = signals[&part.signal()];
                        match part.range() {
                            Some(range) => signal.index(range.start()..range.end()),
                            None => signal,
                        }
                    })
                    .collect(),
            );
        }
        cell.add_instance(instance);
    }
    let mut lib = lib.into_builder();
    lib.overwrite_cell_with_id(id, cell);
    lib.build()
        .map_err(substrate::schematic::conv::ConvError::from)
        .map_err(Into::into)
}

/// An extracted (PEX) view of block `T`, read from a SPICE netlist on disk.
///
/// The extracted subcircuit must have the same name as `T` and ports named with the
/// bracketed bus convention written by [`write_verification_netlist`].
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Pex<T> {
    /// The block whose extracted view this is.
    pub block: T,
    /// The path to the extracted netlist.
    pub netlist: PathBuf,
}

impl<T> Pex<T> {
    /// Creates a new extracted view of `block`.
    pub fn new(block: T, netlist: impl Into<PathBuf>) -> Self {
        Self {
            block,
            netlist: netlist.into(),
        }
    }
}

impl<T: Block> Block for Pex<T> {
    type Io = T::Io;

    fn name(&self) -> ArcStr {
        arcstr::format!("{}_pex", self.block.name())
    }

    fn io(&self) -> Self::Io {
        self.block.io()
    }
}

impl<T: Block<Io: HasBundleKind<BundleKind: substrate::types::schematic::SchematicBundleKind>>>
    Schematic for Pex<T>
{
    type Schema = Spice;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let expected: Vec<ArcStr> = port_names(&self.block)
            .into_iter()
            .map(|(_, bracket)| ArcStr::from(bracket))
            .collect();
        let nodes = io.flatten_vec();
        assert_eq!(expected.len(), nodes.len());
        let netlist = self.netlist.canonicalize().map_err(std::sync::Arc::new)?;
        let ports = extracted_ports(&netlist, &self.block.name())
            .map_err(|e| substrate::error::Error::Anyhow(std::sync::Arc::new(e)))?;
        if ports.len() != expected.len()
            || ports
                .iter()
                .any(|p| !expected.iter().any(|e| e.eq_ignore_ascii_case(p)))
        {
            return Err(substrate::error::Error::Anyhow(std::sync::Arc::new(
                anyhow::anyhow!(
                    "extracted ports {ports:?} do not match the block interface {expected:?}"
                ),
            )));
        }
        let mut prim = PrimitiveBinding::new(Primitive::RawInstanceWithInclude {
            cell: self.block.name(),
            netlist,
            ports: ports.clone(),
        });
        for port in ports {
            let index = expected
                .iter()
                .position(|e| e.eq_ignore_ascii_case(&port))
                .unwrap();
            prim.connect(port, nodes[index]);
        }
        cell.set_primitive(prim);
        Ok(())
    }
}

/// Reads only the extracted interface; device statements may use vendor-specific syntax.
fn extracted_ports(path: &Path, cell: &str) -> anyhow::Result<Vec<ArcStr>> {
    let text = std::fs::read_to_string(path)?;
    let mut header = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        if let Some(ports) = header.as_mut() {
            if let Some(rest) = line.strip_prefix('+') {
                let ports: &mut String = ports;
                ports.push(' ');
                ports.push_str(rest);
                continue;
            }
            break;
        }
        let mut words = line.split_whitespace();
        if words
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case(".subckt"))
            && words.next().is_some_and(|s| s.eq_ignore_ascii_case(cell))
        {
            header = Some(words.collect::<Vec<_>>().join(" "));
        }
    }
    let header =
        header.ok_or_else(|| anyhow::anyhow!("no .subckt {cell} in {}", path.display()))?;
    let ports: Vec<ArcStr> = header
        .split_whitespace()
        .take_while(|s| {
            !s.eq_ignore_ascii_case("params:") && !s.contains('=') && !s.starts_with('$')
        })
        .map(ArcStr::from)
        .collect();
    let unique: std::collections::HashSet<_> =
        ports.iter().map(|s| s.to_ascii_lowercase()).collect();
    anyhow::ensure!(
        unique.len() == ports.len(),
        "duplicate ports in extracted subcircuit {cell}"
    );
    Ok(ports)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Clone, Hash, PartialEq, Eq)]
    struct PexFixture;
    impl Block for PexFixture {
        type Io = crate::schematic::NamedIo;
        fn name(&self) -> ArcStr {
            "pex_fixture".into()
        }
        fn io(&self) -> Self::Io {
            use substrate::types::Direction;
            Self::Io::new([("a", 2, Direction::Input), ("y", 1, Direction::Output)])
        }
    }

    #[derive(Clone, Hash, PartialEq, Eq)]
    struct PexParent(PathBuf);
    impl Block for PexParent {
        type Io = crate::schematic::NamedIo;
        fn name(&self) -> ArcStr {
            "pex_parent".into()
        }
        fn io(&self) -> Self::Io {
            PexFixture.io()
        }
    }
    impl Schematic for PexParent {
        type Schema = Spice;
        type NestedData = ();
        fn schematic(&self, io: &IoNodeBundle<Self>, cell: &mut CellBuilder<Spice>) -> Result<()> {
            let dut = cell.instantiate_named(Pex::new(PexFixture, &self.0), "dut");
            cell.connect(io, dut.io());
            Ok(())
        }
    }

    #[test]
    fn extracted_interface_uses_header_order() {
        use substrate::schematic::netlist::ConvertibleNetlister;
        let dir = tempfile::tempdir_in(".").unwrap();
        let input = dir.path().join("extracted.spice");
        let output = dir.path().join("wrapper.spice");
        std::fs::write(
            &input,
            "* extracted view\n.SUBCKT pex_fixture Y\n+ a[1] a[0]\nR0 Y a[0] 1000\n.ENDS\n",
        )
        .unwrap();
        let absolute_input = input.canonicalize().unwrap();
        let ctx = Context::builder().build();
        Spice
            .write_netlist_to_file(&ctx, PexParent(input), &output, NetlistOptions::default())
            .unwrap();
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.contains(absolute_input.to_str().unwrap()));
        let instance = text
            .lines()
            .find(|line| line.trim_end().ends_with(" pex_fixture"))
            .unwrap();
        assert!(instance.contains(" y a_1 a_0 pex_fixture"), "{instance}");
    }

    #[test]
    fn extracted_interface_rejects_duplicate_pins() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("extracted.spice");
        std::fs::write(&path, ".subckt dut p P\n.ends\n").unwrap();
        assert!(extracted_ports(&path, "dut").is_err());
        assert!(extracted_ports(&path, "missing").is_err());
    }

    #[test]
    fn parameterized_gate_export_matches_layout_pin_names() {
        use crate::blocks::gate::{Gate, GateParams, PrimitiveGateParams};
        let gate = Gate::new(GateParams::Nand2(PrimitiveGateParams {
            nwidth: 1000,
            pwidth: 1000,
            length: 150,
        }));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gate.spice");
        write_verification_netlist(&crate::setup_ctx(), gate, &path).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains(".SUBCKT nand2 vdd vss a b y"), "{text}");
    }

    #[test]
    fn layout_pin_names_do_not_rename_instances_or_child_ports() {
        let mut lib = scir::LibraryBuilder::<Spice>::new();
        let mut child = scir::Cell::new("addr_0");
        let input = child.add_node("addr_0");
        child.expose_port(input, scir::Direction::Input);
        let child_id = lib.add_cell(child);
        let mut top = scir::Cell::new("top");
        let input = top.add_node("addr_0");
        top.expose_port(input, scir::Direction::Input);
        let mut instance = scir::Instance::new("addr_0", child_id);
        instance.connect("addr_0", input);
        top.add_instance(instance);
        let top_id = lib.add_cell(top);
        lib.set_top(top_id);
        let lib =
            rename_top_ports(lib.build().unwrap(), &[("addr_0".into(), "addr[0]".into())]).unwrap();
        let top = lib.cell(top_id);
        let instance = top.instance_named("addr_0");
        assert_eq!(top.signal(top.port("addr[0]").signal()).name, "addr[0]");
        assert_eq!(
            instance
                .connection("addr_0")
                .parts()
                .next()
                .unwrap()
                .signal(),
            top.port("addr[0]").signal()
        );
        assert_eq!(lib.cell(child_id).name(), "addr_0");
        assert_eq!(lib.cell(child_id).ports().count(), 1);
        assert_eq!(
            lib.cell(child_id)
                .signal(lib.cell(child_id).port("addr_0").signal())
                .name,
            "addr_0"
        );
    }
}
