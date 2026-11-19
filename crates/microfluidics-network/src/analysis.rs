//! The pipeline, once: a design elaborated, checked, and its network solved.

use std::collections::BTreeMap;

use microfluidics_core::{Check, Waiver};
use microfluidics_layout::{Build, BuildError, Elaborated, Features, Net, Overrides, Process, check, connectivity};

use crate::error::NetworkError;
use crate::graph::extract;
use crate::requirement::values;
use crate::variation::Variation;

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error(transparent)]
    Design(#[from] BuildError),
    #[error(transparent)]
    Graph(#[from] NetworkError),
}

/// A design elaborated and checked as drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    pub elaborated: Elaborated,
    /// The ports placed together: the vertices of the network.
    pub nets: Vec<Net>,
    /// `rules` and `connectivity`.
    pub checks: Vec<Check>,
}

/// Build the design and check it against `process` and its own stated connections.
pub fn analyse(
    build: Build<'_>,
    overrides: &Overrides,
    process: Option<&Process>,
    waivers: &[Waiver],
) -> Result<Analysis, BuildError> {
    let elaborated = build(overrides)?;
    let rules = process.map_or(&[][..], |process| &process.rules);
    let drawn = check(&Features::drawn(&elaborated.chip), rules, waivers);
    let connected = connectivity(&elaborated.chip, waivers);
    Ok(Analysis {
        elaborated,
        nets: connected.nets,
        checks: vec![drawn, connected.check],
    })
}

impl Analysis {
    /// Whether the design is sound as drawn: within its rules, and connected as it says.
    pub fn sound(&self) -> bool {
        self.checks.iter().all(Check::passes)
    }

    /// The value of each requirement's measure at each of its subjects, in SI units, were the
    /// chip made as `variation` says. Nothing is drawn again: the network is extracted and solved.
    pub fn values_under(&self, variation: &Variation) -> Result<BTreeMap<String, Vec<f64>>, NetworkError> {
        let Some(operating) = &self.elaborated.operating else {
            return Ok(BTreeMap::new());
        };
        let network = extract(&self.elaborated.chip, &self.nets, variation)?;
        let solution = network.solve(operating)?;
        let measured = |requirement: &microfluidics_core::model::Requirement| {
            let numbers = values(requirement, &solution)?
                .into_iter()
                .map(|(_, value)| value)
                .collect();
            Ok((requirement.name.clone(), numbers))
        };
        // Yield is of the whole chip: it is found from these values, not among them.
        let placed = self
            .elaborated
            .requirements
            .iter()
            .filter(|requirement| requirement.measure != microfluidics_core::model::Measure::Yield);
        placed.map(measured).collect()
    }
}
