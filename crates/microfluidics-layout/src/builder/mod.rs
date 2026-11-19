//! The builder: the one place a design is put together.
//!
//! A design is built by calls, in order: rectangles, ports and edges into a region's body, regions
//! placed into the body that holds them, and a design's own facts on top. Things are known here
//! by path (`feed.trunk_width`, `lane[2].left[17]`) and by the source line that asked for
//! them. The Python package derives those paths from the attributes things are assigned to and
//! calls this; no rule of what a design may be lives anywhere else.

mod body;
mod function;
mod model;
mod place;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use serde::Serialize;

use microfluidics_core::Quantity;
use microfluidics_core::model::{LayerSpec, LayerStack};

use crate::builder::model::{Built, Parameter};
use crate::chip::Chip;
use crate::geometry::TOLERANCE;

/// How deep regions may nest: a region that contains itself is stopped here.
const MAX_DEPTH: usize = 32;

/// Where in a design's source something was asked for.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Origin {
    pub file: String,
    pub line: u32,
}

impl Origin {
    pub fn new(file: &str, line: u32) -> Self {
        Self {
            file: file.to_owned(),
            line,
        }
    }
}

pub use crate::builder::model::{Elaborated, Instance, Overrides, Tunable};

/// A fault in a design, with the line that caused it when that is known.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildError {
    pub message: String,
    pub origin: Option<Origin>,
}

impl BuildError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            origin: None,
        }
    }

    pub fn at(origin: &Origin, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            origin: Some(origin.clone()),
        }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.origin {
            Some(origin) => write!(f, "{}:{}: {}", origin.file, origin.line, self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for BuildError {}

/// A design that can be made again with other values: what closure, and everything else that
/// analyses a design, is given.
pub type Build<'a> = &'a dyn Fn(&Overrides) -> Result<Elaborated, BuildError>;

/// One parameter of a region: its default, and the range it is held to.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSpec {
    pub name: String,
    pub default: Quantity,
    pub range: Option<(Quantity, Quantity)>,
    /// Whether it is a whole number of things.
    pub count: bool,
}

/// What a region states about itself before it is built.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RegionSpec {
    pub name: String,
    /// What it is, in a sentence or two.
    pub doc: String,
    /// The file it is written in.
    pub source: String,
    pub params: Vec<ParamSpec>,
}

/// Where a region goes in the body that holds it.
#[derive(Debug, Clone, PartialEq)]
pub enum Place {
    /// Its origin at a point, turned counterclockwise by whole quarter turns.
    At { point: crate::geometry::Point, turned: u8 },
    /// Its port `port` meeting the port `to` of the body, facing it, with a gap of `offset`
    /// along that port's direction.
    Attach { port: String, to: String, offset: f64 },
}

/// One body being built: a design, or one instance of a region.
pub struct Builder {
    /// The design's name, or the region's.
    name: String,
    /// The path of this body from the design, with a trailing dot; empty for the design.
    path: String,
    depth: usize,
    overrides: Rc<Overrides>,
    /// The height of each of the design's layers.
    layers: Rc<BTreeMap<String, f64>>,
    built: Built,
    /// Where each named thing was asked for, by its path within this body.
    origins: BTreeMap<String, Origin>,
    /// Every name given in this body.
    named: BTreeSet<String>,
    /// The name of the region this body is an instance of; empty for the design.
    region: String,
    /// The instances placed in this body and those within them, by their path within it.
    instances: Vec<Instance>,
    /// This instance's own parameters, as they were settled.
    settled: Vec<(String, Parameter)>,
    /// Where this instance was asked for.
    origin: Origin,
    /// The path of every parameter in this body and in what is placed in it, as an override
    /// names it.
    params: BTreeSet<String>,
    // A design's own facts.
    slab: Option<[f64; 3]>,
    glass: Option<[f64; 3]>,
    stack: Vec<LayerSpec>,
    process: Option<String>,
    /// Whether `operate` and `require` take effect: off while a benchmark's candidate is built.
    own_conditions: bool,
}

impl Builder {
    /// A builder for the design `name`, with `overrides` replacing parameters by path.
    pub fn design(name: &str, overrides: &Overrides) -> Self {
        Self {
            name: name.to_owned(),
            path: String::new(),
            depth: 0,
            overrides: Rc::new(overrides.clone()),
            layers: Rc::new(BTreeMap::new()),
            built: Built::default(),
            origins: BTreeMap::new(),
            named: BTreeSet::new(),
            region: String::new(),
            instances: Vec::new(),
            settled: Vec::new(),
            origin: Origin::default(),
            params: BTreeSet::new(),
            slab: None,
            glass: None,
            stack: Vec::new(),
            process: None,
            own_conditions: true,
        }
    }

    fn is_design(&self) -> bool {
        self.depth == 0
    }

    /// The instance this body is, for a message: ` (in feed)`; nothing for the design.
    fn whose(&self) -> String {
        match self.path.trim_end_matches('.') {
            "" => String::new(),
            path => format!(" (in {path})"),
        }
    }

    /// A fault a region finds in itself, such as parameters that do not fit together, naming
    /// the instance it was found in.
    pub fn fault(&self, at: &Origin, message: &str) -> BuildError {
        BuildError::at(at, format!("{message}{}", self.whose()))
    }

    fn design_only(&self, at: &Origin, what: &str) -> Result<(), BuildError> {
        if self.is_design() {
            Ok(())
        } else {
            Err(BuildError::at(at, format!("only a design {what}")))
        }
    }

    /// Take `name` for something made in this body.
    fn name(&mut self, name: &str, at: &Origin) -> Result<(), BuildError> {
        if name.is_empty() {
            return Err(BuildError::at(at, "something here has no name"));
        }
        if !self.named.insert(name.to_owned()) {
            return Err(BuildError::at(
                at,
                format!("`{name}` is already the name of something here"),
            ));
        }
        self.origins.insert(name.to_owned(), at.clone());
        Ok(())
    }

    /// The layer `name`, which the design must have.
    fn layer_named<'a>(&self, name: &'a str, at: &Origin) -> Result<&'a str, BuildError> {
        if self.layers.contains_key(name) {
            return Ok(name);
        }
        let known = listed(self.layers.keys().map(String::as_str));
        Err(BuildError::at(
            at,
            format!("the design has no layer `{name}`; its layers are {known}"),
        ))
    }

    /// The path of this body from the design, with a trailing dot; empty for the design.
    pub fn prefix(&self) -> &str {
        &self.path
    }

    /// The height of the layer `name`, when the design has it.
    pub fn height(&self, name: &str) -> Option<f64> {
        self.layers.get(name).copied()
    }

    /// The value this body's parameter `name` was settled at.
    pub fn settled(&self, name: &str) -> Option<Quantity> {
        let found = self.settled.iter().find(|(held, _)| held == name);
        found
            .map(|(_, parameter)| parameter.value)
            .or_else(|| self.built.parameters.get(name).map(|held| held.value))
    }

    /// State the chip's size.
    pub fn slab(&mut self, size: [f64; 3], at: &Origin) -> Result<(), BuildError> {
        self.design_only(at, "states a slab")?;
        if size.iter().any(|side| *side <= 0.0) || self.slab.is_some() {
            return Err(BuildError::at(
                at,
                "a design states one slab, with every size above zero",
            ));
        }
        self.slab = Some(size);
        Ok(())
    }

    /// State the glass the slab is bonded to: its width, depth and thickness.
    pub fn glass(&mut self, size: [f64; 3], at: &Origin) -> Result<(), BuildError> {
        self.design_only(at, "states its glass")?;
        if size.iter().any(|side| *side <= 0.0) || self.glass.is_some() {
            return Err(BuildError::at(
                at,
                "a design states one glass, with every size above zero",
            ));
        }
        self.glass = Some(size);
        Ok(())
    }

    /// State one of the chip's layers. Layers are stated before anything is drawn on them.
    pub fn layer(&mut self, spec: LayerSpec, at: &Origin) -> Result<(), BuildError> {
        self.design_only(at, "states layers")?;
        if spec.height <= 0.0 {
            return Err(BuildError::at(
                at,
                format!("layer `{}` must have a height above zero", spec.name),
            ));
        }
        if self.built.layout.feature_count() > 0 || !self.built.layout.ports.is_empty() {
            return Err(BuildError::at(
                at,
                "a design states its layers before anything is drawn",
            ));
        }
        let mut layers = (*self.layers).clone();
        layers.insert(spec.name.clone(), spec.height);
        self.layers = Rc::new(layers);
        self.stack.push(spec);
        Ok(())
    }

    /// Name the process the chip is made in.
    pub fn process(&mut self, name: &str, at: &Origin) -> Result<(), BuildError> {
        self.design_only(at, "names a process")?;
        self.process = Some(name.to_owned());
        Ok(())
    }

    /// One of the design's own parameters: `value`, or its override, held to `range`.
    pub fn param(
        &mut self,
        name: &str,
        value: Quantity,
        range: Option<(Quantity, Quantity)>,
        count: bool,
        at: &Origin,
    ) -> Result<Quantity, BuildError> {
        self.design_only(at, "has parameters of its own; a region's are stated with the region")?;
        self.name(name, at)?;
        self.params.insert(name.to_owned());
        let settled = self.overrides.0.get(name).copied().unwrap_or(value);
        if settled.dim != value.dim {
            let (given, wanted) = (settled.dim.name(), value.dim.name());
            return Err(BuildError::at(
                at,
                format!("the value given for `{name}` is {given}, but `{name}` is {wanted}"),
            ));
        }
        if count && (settled.si.fract() != 0.0 || settled.si < 0.0) {
            return Err(BuildError::at(
                at,
                format!("`{name}` is a whole number, not {}", settled.si),
            ));
        }
        if let Some((low, high)) = range
            && (settled.si < low.si - TOLERANCE || settled.si > high.si + TOLERANCE)
        {
            let (is, low, high) = (settled.written(), low.written(), high.written());
            return Err(BuildError::at(at, format!("`{name}` is {is}, outside {low}..{high}")));
        }
        self.built.parameters.insert(
            name.to_owned(),
            Parameter {
                value: settled,
                range,
                count,
            },
        );
        Ok(settled)
    }

    /// Whether the design's own `operate` and requirements take effect. A benchmark turns them
    /// off while its candidate is built, then on again to state its own.
    pub fn own_conditions(&mut self, on: bool) {
        self.own_conditions = on;
    }

    /// The finished design: the chip made and checked as a whole.
    pub fn finish(self) -> Result<Elaborated, BuildError> {
        let fault = |message: String| BuildError { message, origin: None };
        let slab = self
            .slab
            .ok_or_else(|| fault("a design states its slab: its width, depth and thickness".to_owned()))?;
        let glass = self.glass.ok_or_else(|| {
            fault("a design states the glass its slab is bonded to: its width, depth and thickness".to_owned())
        })?;
        if self.stack.is_empty() {
            return Err(fault(
                "a design states its layers, each with a height and a GDSII number".to_owned(),
            ));
        }
        let stack = LayerStack::new(self.stack).map_err(fault)?;
        if let Some(unknown) = self.overrides.0.keys().find(|path| !self.params.contains(*path)) {
            return Err(fault(format!(
                "a value is given for `{unknown}`, and the design has no such parameter; its parameters are {}",
                listed(self.params.iter().map(String::as_str))
            )));
        }
        let built = self.built;
        let mut chip = Chip::new(slab, glass, stack, built.layout).map_err(|error| fault(error.to_string()))?;
        (chip.edges, chip.joints) = (built.edges, built.joints);
        Ok(Elaborated {
            name: self.name,
            chip,
            process: self.process,
            quantities: built.quantities,
            operating: built.operating,
            requirements: built.requirements,
            tunables: built.tunables,
            origins: self.origins,
            sources: BTreeMap::new(),
            instances: self.instances,
        })
    }
}

/// Names set out for a message: `a, b, c`, or `none`.
fn listed<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = names.collect();
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}
