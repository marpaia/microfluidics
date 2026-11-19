//! How a chip is run, and what it must then do.

use std::collections::BTreeMap;

use serde::Serialize;

/// What is applied at an access point, in SI units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Drive {
    Pressure(f64),
    FlowRate(f64),
}

/// How the chip is run.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Operating {
    /// In pascal seconds.
    pub viscosity: f64,
    /// What is applied at each access point, by its vertex.
    pub drives: BTreeMap<String, Drive>,
    /// What enters at each access point: the fraction of each species.
    pub feeds: BTreeMap<String, BTreeMap<String, f64>>,
}

/// Something measured of the flow.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    /// The flow rate through a edge, in cubic metres a second.
    Flow,
    /// The pressure at a vertex, in pascals.
    Pressure,
    /// The pressure from one vertex to another.
    Drop,
    /// The mean speed in the narrowest duct of a edge, in metres a second.
    Velocity,
    /// The shear stress on the wall of the narrowest duct of a edge, in pascals.
    Shear,
    /// The Reynolds number in the narrowest duct of a edge.
    Reynolds,
    /// How long fluid takes to pass through a edge, in seconds.
    Residence,
    /// The fraction of one species at a vertex.
    Fraction { species: String },
    /// The fraction of chips, made within the process's tolerances, that meet every other requirement.
    Yield,
}

impl Measure {
    /// The unit a measure is reported in, and how many of them make one SI unit.
    pub fn reported(&self) -> (&'static str, f64) {
        match self {
            Measure::Flow => ("ul_min", 6e10),
            Measure::Pressure | Measure::Drop => ("mbar", 0.01),
            Measure::Velocity => ("mm_s", 1e3),
            Measure::Shear => ("Pa", 1.0),
            Measure::Reynolds => ("", 1.0),
            Measure::Residence => ("s", 1.0),
            Measure::Fraction { .. } | Measure::Yield => ("%", 100.0),
        }
    }
}

/// What a requirement asks of the values it measures, in SI units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bound {
    /// Every value within this fraction of their mean.
    EqualWithin(f64),
    /// Every value within `within` of `value`; within that fraction of it when `relative`.
    Is {
        value: f64,
        within: f64,
        relative: bool,
    },
    AtMost(f64),
    AtLeast(f64),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Requirement {
    pub name: String,
    /// The requirement in the researcher's words.
    pub statement: String,
    pub measure: Measure,
    /// Where it is measured: edges or vertices, as the measure takes. A pressure drop takes pairs.
    pub subjects: Vec<String>,
    pub bound: Bound,
}
