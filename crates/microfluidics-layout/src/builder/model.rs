//! What the builder makes and what steers it: an elaborated design, the values that replace
//! parameters, and the parameters a design allows to be tuned.

use std::collections::BTreeMap;

use serde::Serialize;

use microfluidics_core::Quantity;
use microfluidics_core::model::{EdgeSpec, Operating, Requirement};

use crate::builder::Origin;
use crate::chip::Chip;
use crate::plan::Layout;

/// Values that replace those written in source, by path: `feed.trunk_width`, or `pitch` for a
/// design's own parameter.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Overrides(pub BTreeMap<String, Quantity>);

impl Overrides {
    /// Read overrides from the text of the file named `file`: a line for each, as
    /// `"feed.trunk_width" = "412.6um"`.
    pub fn parse(file: &str, text: &str) -> Result<Self, String> {
        let written: BTreeMap<String, String> = toml::from_str(text).map_err(|error| format!("{file}: {error}"))?;
        let read = |(path, value): (String, String)| match Quantity::parse(&value) {
            Some(quantity) => Ok((path, quantity)),
            None => Err(format!(
                "{file}: the value of {path}, {value:?}, is not a quantity such as \"412.6um\""
            )),
        };
        written.into_iter().map(read).collect::<Result<_, _>>().map(Self)
    }

    /// The overrides as the text of a file [`Overrides::parse`] reads.
    pub fn toml(&self) -> String {
        self.0
            .iter()
            .map(|(path, value)| format!("{path:?} = {:?}\n", value.written()))
            .collect()
    }
}

/// A parameter that may be changed to meet the requirements, with the range it must stay in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tunable {
    /// Its path, as an override names it: `feed.trunk_width`, or `pitch` for the design's own.
    pub path: String,
    pub value: Quantity,
    pub low: Quantity,
    pub high: Quantity,
    /// Whether it is a whole number of things.
    pub count: bool,
}

/// A parameter as elaboration settled it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parameter {
    pub(crate) value: Quantity,
    pub(crate) range: Option<(Quantity, Quantity)>,
    pub(crate) count: bool,
}

/// One placed instance of a region: where it is in the design and what it was built with.
#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    /// Its path from the design: `feed`, `lane[2]`, `feed.trunk`.
    pub path: String,
    /// The name of the region it is an instance of.
    pub region: String,
    /// Its parameters as they were settled, in the order the region states them.
    pub parameters: Vec<(String, Quantity)>,
}

impl Instance {
    /// Whether the design itself placed it, and not a region the design placed.
    pub fn of_design(&self) -> bool {
        !self.path.contains('.')
    }
}

/// A design with every instance placed and every number final.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Elaborated {
    pub name: String,
    pub chip: Chip,
    /// The processes the chip is made in, as the design names them.
    pub processes: Vec<String>,
    /// What regions state about themselves, by path: `lane[0].observation_sites`.
    pub quantities: BTreeMap<String, f64>,
    /// How the chip is run, when the design says.
    pub operating: Option<Operating>,
    /// What the chip must do.
    pub requirements: Vec<Requirement>,
    /// The parameters the design allows to be changed.
    pub tunables: Vec<Tunable>,
    /// Where each named thing was asked for in the design's source, by path. Not part of what
    /// the design is: moving a line changes nothing that is made.
    #[serde(skip)]
    pub origins: BTreeMap<String, Origin>,
    /// The hash of each source file the design was built from, by its name.
    #[serde(skip)]
    pub sources: BTreeMap<String, String>,
    /// Every placed instance, those within others after the one that holds them. Not part of
    /// what the design is: the chip already holds everything they made.
    #[serde(skip)]
    pub instances: Vec<Instance>,
}

/// What one body has had built into it.
#[derive(Default)]
pub(crate) struct Built {
    pub(crate) layout: Layout,
    pub(crate) quantities: BTreeMap<String, f64>,
    pub(crate) edges: Vec<EdgeSpec>,
    pub(crate) joints: Vec<(String, String)>,
    pub(crate) operating: Option<Operating>,
    pub(crate) requirements: Vec<Requirement>,
    /// Every parameter of the design and of each instance, by path.
    pub(crate) parameters: BTreeMap<String, Parameter>,
    pub(crate) tunables: Vec<Tunable>,
}
