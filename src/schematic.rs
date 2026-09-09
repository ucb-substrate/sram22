//! Named circuit construction on Substrate 2.
//!
//! SRAMs and decoders have parameter-dependent interfaces. These helpers retain named
//! ports while using Substrate's node unification, hierarchy, schema conversion and cache.

use arcstr::ArcStr;
use std::collections::HashMap;
use std::ops::{Range, RangeFrom, RangeFull, RangeTo};
use std::sync::Arc;
use substrate::block::Block;
use substrate::context::Context;
use substrate::schematic::{CellBuilder, HasNestedView, InstancePath, NestedView, Schematic};
use substrate::types::schematic::*;
pub use substrate::types::Direction;
use substrate::types::{FlatLen, Flatten, HasBundleKind, HasNameTree, NameTree, Unflatten};
pub type Result<T> = anyhow::Result<T>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub name: ArcStr,
    pub width: usize,
    pub direction: Direction,
}
impl Port {
    pub fn name(&self) -> &ArcStr {
        &self.name
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn direction(&self) -> Direction {
        self.direction
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NamedIo(pub Vec<Port>);
impl NamedIo {
    pub fn new(ports: impl IntoIterator<Item = (impl Into<ArcStr>, usize, Direction)>) -> Self {
        Self(
            ports
                .into_iter()
                .map(|(name, width, direction)| Port {
                    name: name.into(),
                    width,
                    direction,
                })
                .collect(),
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedKind(Vec<(ArcStr, usize)>);
#[derive(Debug, Clone)]
pub struct NamedBundle<T> {
    kind: NamedKind,
    pub values: Vec<T>,
}
impl FlatLen for NamedIo {
    fn len(&self) -> usize {
        self.0.iter().map(|p| p.width).sum()
    }
}
impl Flatten<Direction> for NamedIo {
    fn flatten<E: Extend<Direction>>(&self, out: &mut E) {
        for p in &self.0 {
            out.extend(std::iter::repeat_n(p.direction, p.width));
        }
    }
}
impl HasBundleKind for NamedIo {
    type BundleKind = NamedKind;
    fn kind(&self) -> NamedKind {
        NamedKind(self.0.iter().map(|p| (p.name.clone(), p.width)).collect())
    }
}
impl FlatLen for NamedKind {
    fn len(&self) -> usize {
        self.0.iter().map(|p| p.1).sum()
    }
}
impl HasBundleKind for NamedKind {
    type BundleKind = Self;
    fn kind(&self) -> Self {
        self.clone()
    }
}
impl HasNameTree for NamedKind {
    fn names(&self) -> Option<Vec<NameTree>> {
        if self.len() == 0 {
            return None;
        }
        Some(
            self.0
                .iter()
                .filter(|p| p.1 > 0)
                .map(|(name, width)| {
                    NameTree::new(
                        name.clone(),
                        if *width == 1 {
                            vec![]
                        } else {
                            (0..*width).map(|i| NameTree::new(i, vec![])).collect()
                        },
                    )
                })
                .collect(),
        )
    }
}
impl<T: Send + Sync> HasBundleKind for NamedBundle<T> {
    type BundleKind = NamedKind;
    fn kind(&self) -> NamedKind {
        self.kind.clone()
    }
}
impl<T> FlatLen for NamedBundle<T> {
    fn len(&self) -> usize {
        self.values.len()
    }
}
impl<T: Clone> Flatten<T> for NamedBundle<T> {
    fn flatten<E: Extend<T>>(&self, out: &mut E) {
        out.extend(self.values.iter().cloned());
    }
}
impl Flatten<Node> for NamedBundle<Terminal> {
    fn flatten<E: Extend<Node>>(&self, out: &mut E) {
        out.extend(self.values.iter().map(|t| **t));
    }
}
impl<T> Unflatten<NamedKind, T> for NamedBundle<T> {
    fn unflatten<I: Iterator<Item = T>>(kind: &NamedKind, src: &mut I) -> Option<Self> {
        let values: Vec<_> = src.take(kind.len()).collect();
        (values.len() == kind.len()).then(|| Self {
            kind: kind.clone(),
            values,
        })
    }
}
impl<T: HasNestedView> HasNestedView for NamedBundle<T> {
    type NestedView = NamedBundle<NestedView<T>>;
    fn nested_view(&self, parent: &InstancePath) -> Self::NestedView {
        NamedBundle {
            kind: self.kind.clone(),
            values: self.values.iter().map(|v| v.nested_view(parent)).collect(),
        }
    }
}
impl HasNodeBundle for NamedKind {
    type NodeBundle = NamedBundle<Node>;
}
impl HasTerminalBundle for NamedKind {
    type TerminalBundle = NamedBundle<Terminal>;
}
impl SchematicBundleKind for NamedKind {
    fn terminal_view(
        cell: substrate::schematic::CellId,
        cell_io: &NamedBundle<Node>,
        instance: substrate::schematic::InstanceId,
        instance_io: &NamedBundle<Node>,
    ) -> NamedBundle<Terminal> {
        NamedBundle {
            kind: cell_io.kind.clone(),
            values: cell_io
                .values
                .iter()
                .zip(&instance_io.values)
                .map(|(c, i)| substrate::types::Signal::terminal_view(cell, c, instance, i))
                .collect(),
        }
    }
}

/// Constructors shared by schematics, simulations and design scripts, independent of layout.
pub trait FromParams: Sized {
    type Params: Clone + serde::Serialize + Send + Sync + 'static;
    fn from_params(params: &Self::Params) -> Result<Self>;
}
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct NoParams;

/// A contiguous range in a circuit's node arena.
#[derive(Debug, Clone, Copy)]
pub struct Slice {
    start: usize,
    width: usize,
}
impl Slice {
    pub fn index<I>(&self, i: I) -> <Self as IndexOwned<I>>::Output
    where
        Self: IndexOwned<I>,
    {
        IndexOwned::index(self, i)
    }
    pub fn width(&self) -> usize {
        self.width
    }
}
pub trait IndexOwned<I> {
    type Output;
    fn index(&self, index: I) -> Self::Output;
}
impl IndexOwned<usize> for Slice {
    type Output = Slice;
    fn index(&self, i: usize) -> Slice {
        assert!(i < self.width);
        Slice {
            start: self.start + i,
            width: 1,
        }
    }
}
impl IndexOwned<Range<usize>> for Slice {
    type Output = Slice;
    fn index(&self, i: Range<usize>) -> Slice {
        assert!(i.start <= i.end && i.end <= self.width);
        Slice {
            start: self.start + i.start,
            width: i.end - i.start,
        }
    }
}
impl IndexOwned<RangeFrom<usize>> for Slice {
    type Output = Slice;
    fn index(&self, i: RangeFrom<usize>) -> Slice {
        self.index(i.start..self.width)
    }
}
impl IndexOwned<RangeTo<usize>> for Slice {
    type Output = Slice;
    fn index(&self, i: RangeTo<usize>) -> Slice {
        self.index(0..i.end)
    }
}
impl IndexOwned<RangeFull> for Slice {
    type Output = Slice;
    fn index(&self, _: RangeFull) -> Slice {
        *self
    }
}
#[derive(Debug, Clone)]
pub struct Signal(Vec<Slice>);
impl Signal {
    pub fn new(s: impl IntoIterator<Item = Slice>) -> Self {
        Self(s.into_iter().collect())
    }
}
impl From<Slice> for Signal {
    fn from(s: Slice) -> Self {
        Self(vec![s])
    }
}
impl From<&Slice> for Signal {
    fn from(s: &Slice) -> Self {
        (*s).into()
    }
}
impl From<&Signal> for Signal {
    fn from(s: &Signal) -> Self {
        s.clone()
    }
}

/// A deferred instance: connections are resolved when it is added to its parent cell.
type InstanceInserter<S> =
    dyn Fn(&mut CellBuilder<S>, Option<ArcStr>) -> HashMap<ArcStr, Vec<Node>> + Send + Sync;

/// An instance and its named connections, ready to insert into a parent cell.
#[derive(Clone)]
pub struct CircuitInstance<S: substrate::schematic::schema::Schema> {
    ports: Vec<Port>,
    name: Option<ArcStr>,
    connections: HashMap<ArcStr, Signal>,
    insert: Arc<InstanceInserter<S>>,
}
impl<S: substrate::schematic::schema::Schema> CircuitInstance<S> {
    pub fn connect(&mut self, name: impl Into<ArcStr>, signal: impl Into<Signal>) {
        self.connections
            .insert(name.into().to_ascii_lowercase().into(), signal.into());
    }
    pub fn connect_all<N: Into<ArcStr>, C: Into<Signal>>(
        &mut self,
        conns: impl IntoIterator<Item = (N, C)>,
    ) {
        for (n, c) in conns {
            self.connect(n, c)
        }
    }
    pub fn with_connection(mut self, name: impl Into<ArcStr>, signal: impl Into<Signal>) -> Self {
        self.connect(name, signal);
        self
    }
    pub fn with_connections<N: Into<ArcStr>, C: Into<Signal>>(
        mut self,
        conns: impl IntoIterator<Item = (N, C)>,
    ) -> Self {
        self.connect_all(conns);
        self
    }
    pub fn set_name(&mut self, name: impl Into<ArcStr>) {
        self.name = Some(name.into());
    }
    pub fn named(mut self, name: impl Into<ArcStr>) -> Self {
        self.set_name(name);
        self
    }
    pub fn ports(&self) -> impl Iterator<Item = &Port> {
        self.ports.iter()
    }
    pub fn port(&self, name: &str) -> Result<&Port> {
        self.ports
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("unknown port {name}"))
    }
    pub fn add_to(self, ctx: &mut CircuitBuilder<S>) {
        ctx.add_instance(self)
    }
}

pub trait BuildIn<S: substrate::schematic::schema::Schema>: FromParams {
    fn build_in(self) -> CircuitInstance<S>;
}
/// Creates a named instance from any compatible native Substrate block.
pub fn native_instance<S, B>(block: B) -> CircuitInstance<S>
where
    S: substrate::schematic::schema::Schema,
    B: Schematic<Schema = S>,
{
    let block = Arc::new(block);
    let io = block.io();
    let names = io.kind().flat_names(None);
    let dirs = io.flatten_vec();
    let mut ports: Vec<Port> = Vec::new();
    for (name, dir) in names.iter().zip(dirs) {
        let (name, _) = split_name(name);
        if let Some(p) = ports.iter_mut().find(|p| p.name == name) {
            p.width += 1;
        } else {
            ports.push(Port {
                name,
                width: 1,
                direction: dir,
            });
        }
    }
    CircuitInstance {
        ports,
        name: None,
        connections: HashMap::new(),
        insert: Arc::new(move |cell, name| {
            let inst = match name {
                Some(name) => cell.instantiate_named(block.clone(), name),
                None => cell.instantiate(block.clone()),
            };
            let nodes = Flatten::<Node>::flatten_vec(inst.io());
            let mut map: HashMap<ArcStr, Vec<Node>> = HashMap::new();
            for (name, node) in names.iter().zip(nodes) {
                let (name, _) = split_name(name);
                map.entry(name).or_default().push(node);
            }
            map
        }),
    }
}
fn split_name(name: &substrate::types::NameBuf) -> (ArcStr, Option<usize>) {
    let mut base = name.clone();
    if let Some(substrate::types::NameFragment::Idx(i)) = base.pop() {
        return (base.to_string().to_ascii_lowercase().into(), Some(i));
    }
    (name.to_string().to_ascii_lowercase().into(), None)
}

pub struct CircuitBuilder<'a, S: substrate::schematic::schema::Schema = sky130::Sky130> {
    cell: &'a mut CellBuilder<S>,
    nodes: Vec<Node>,
    ports: HashMap<ArcStr, Slice>,
    directions: HashMap<ArcStr, Direction>,
    instance_names: std::collections::HashSet<ArcStr>,
}
impl<'a, S: substrate::schematic::schema::Schema> CircuitBuilder<'a, S> {
    pub fn new<I: substrate::types::Io>(
        io: &I,
        nodes: impl Flatten<Node>,
        cell: &'a mut CellBuilder<S>,
    ) -> Self {
        let names = io.kind().flat_names(None);
        let nodes = nodes.flatten_vec();
        let mut ports: HashMap<ArcStr, Slice> = HashMap::new();
        let mut directions = HashMap::new();
        let io_directions = io.flatten_vec();
        for (i, name) in names.iter().enumerate() {
            let (name, _) = split_name(name);
            if let Some(previous) = directions.insert(name.clone(), io_directions[i]) {
                assert_eq!(
                    previous, io_directions[i],
                    "mixed directions in port {name}"
                );
            }
            ports
                .entry(name)
                .and_modify(|p| p.width += 1)
                .or_insert(Slice { start: i, width: 1 });
        }
        Self {
            cell,
            nodes,
            ports,
            directions,
            instance_names: Default::default(),
        }
    }
    pub fn inner(&self) -> &Context {
        self.cell.ctx()
    }
    pub fn port(&mut self, name: impl Into<ArcStr>, dir: Direction) -> Slice {
        let name = name.into().to_ascii_lowercase();
        let port = *self
            .ports
            .get(name.as_str())
            .unwrap_or_else(|| panic!("undeclared IO port {name}"));
        assert_eq!(
            self.directions[name.as_str()],
            dir,
            "incorrect direction for port {name}"
        );
        port
    }
    pub fn bus_port(&mut self, name: impl Into<ArcStr>, width: usize, dir: Direction) -> Slice {
        let p = self.port(name, dir);
        assert_eq!(p.width, width);
        p
    }
    pub fn ports<N: Into<ArcStr>, const K: usize>(
        &mut self,
        names: [N; K],
        dir: Direction,
    ) -> [Slice; K] {
        names.map(|n| self.port(n, dir))
    }
    pub fn signal(&mut self, name: impl Into<ArcStr>) -> Slice {
        self.bus(name, 1)
    }
    pub fn signals<N: Into<ArcStr>, const K: usize>(&mut self, names: [N; K]) -> [Slice; K] {
        names.map(|n| self.signal(n))
    }
    pub fn bus(&mut self, name: impl Into<ArcStr>, width: usize) -> Slice {
        let p = Slice {
            start: self.nodes.len(),
            width,
        };
        // Explicit bracket names preserve layout, extracted-netlist and testbench paths.
        let name = name.into();
        for i in 0..width {
            let n = if width == 1 {
                name.clone()
            } else {
                arcstr::format!("{name}[{i}]")
            };
            self.nodes
                .push(self.cell.signal(n, substrate::types::Signal));
        }
        p
    }
    pub fn buses<N: Into<ArcStr>, const K: usize>(
        &mut self,
        names: [N; K],
        width: usize,
    ) -> [Slice; K] {
        names.map(|n| self.bus(n, width))
    }
    pub fn instantiate<B: BuildIn<S>>(&self, params: &B::Params) -> Result<CircuitInstance<S>> {
        Ok(B::from_params(params)?.build_in())
    }
    pub fn add_instance(&mut self, inst: CircuitInstance<S>) {
        let base = inst.name.unwrap_or_else(|| arcstr::literal!("0"));
        let mut name = base.clone();
        let mut suffix = 1;
        while !self.instance_names.insert(name.to_ascii_lowercase().into()) {
            name = arcstr::format!("{base}_{suffix}");
            suffix += 1;
        }
        let ports = (inst.insert)(self.cell, Some(name));
        for (name, signal) in inst.connections {
            let nodes: Vec<_> = signal
                .0
                .into_iter()
                .flat_map(|s| self.nodes[s.start..s.start + s.width].iter().copied())
                .collect();
            let target = ports.get(&name).unwrap_or_else(|| {
                panic!(
                    "unknown instance port {name}; available: {:?}",
                    ports.keys()
                )
            });
            assert_eq!(target.len(), nodes.len(), "port width mismatch on {name}");
            for (a, b) in target.iter().zip(nodes) {
                self.cell.connect(*a, b);
            }
        }
    }
    pub fn bubble_all_ports(&mut self, inst: &mut CircuitInstance<S>) {
        self.bubble_filter_map(inst, |p| Some(p.name.clone()));
    }
    pub fn bubble_filter_map(
        &mut self,
        inst: &mut CircuitInstance<S>,
        f: impl Fn(&Port) -> Option<ArcStr>,
    ) {
        for p in inst.ports.clone() {
            if let Some(name) = f(&p) {
                let signal = self.bus_port(name, p.width, p.direction);
                inst.connect(p.name, signal);
            }
        }
    }
}

/// Implements schema conversion for a SKY130 block in both circuit and testbench builders.
#[macro_export]
macro_rules! impl_sky130_build {
    ($typ:ty) => {
        impl $crate::schematic::BuildIn<sky130::Sky130> for $typ {
            fn build_in(self) -> $crate::schematic::CircuitInstance<sky130::Sky130> {
                $crate::schematic::native_instance(self)
            }
        }
        impl $crate::schematic::BuildIn<$crate::sim::Simulator> for $typ {
            fn build_in(self) -> $crate::schematic::CircuitInstance<$crate::sim::Simulator> {
                $crate::schematic::native_instance(substrate::schematic::ConvertSchema::<
                    _,
                    $crate::sim::Simulator,
                >::new(
                    substrate::schematic::ConvertSchema::<_, $crate::sim::PdkSchema>::new(self),
                ))
            }
        }
    };
}

impl FromParams for sky130::mos::Nfet01v8 {
    type Params = (i64, i64);
    fn from_params(p: &Self::Params) -> Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(sky130::mos::Nfet01v8);

impl FromParams for sky130::mos::Pfet01v8 {
    type Params = (i64, i64);
    fn from_params(p: &Self::Params) -> Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(sky130::mos::Pfet01v8);

impl CircuitBuilder<'_, crate::sim::Simulator> {
    pub fn instantiate_pex<B>(
        &self,
        params: &B::Params,
        path: impl AsRef<std::path::Path>,
    ) -> Result<CircuitInstance<crate::sim::Simulator>>
    where
        B: FromParams + Schematic<Schema = sky130::Sky130>,
    {
        let pex = crate::netlist::Pex::new(B::from_params(params)?, path.as_ref());
        Ok(native_instance(substrate::schematic::ConvertSchema::<
            _,
            crate::sim::Simulator,
        >::new(pex)))
    }
}
