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
    /// `connectivity`, then `rules` against each process.
    pub checks: Vec<Check>,
}

/// The waivers that speak of findings made against `process`.
pub fn waived_at(waivers: &[Waiver], process: Option<&str>) -> Vec<Waiver> {
    let applying = waivers.iter().filter(|waiver| waiver.applies(process));
    applying.cloned().collect()
}

/// The design's rules as drawn, checked against `process`; with none, against no rule.
pub fn drawn(elaborated: &Elaborated, process: Option<&Process>, waivers: &[Waiver]) -> Check {
    let name = process.map(|process| process.name.as_str());
    let rules = process.map_or(&[][..], |process| &process.rules);
    let features = Features::drawn(&elaborated.chip);
    let mut found = check(&features, rules, &waived_at(waivers, name)).for_process(name);
    found.note = process.and_then(Process::caveat);
    found
}

/// Each of `processes` in turn, or once with none when there are none.
pub fn each(processes: &[Process]) -> Vec<Option<&Process>> {
    if processes.is_empty() {
        vec![None]
    } else {
        processes.iter().map(Some).collect()
    }
}

/// Build the design and check it against its own stated connections and every one of
/// `processes`.
pub fn analyse(
    build: Build<'_>,
    overrides: &Overrides,
    processes: &[Process],
    waivers: &[Waiver],
) -> Result<Analysis, BuildError> {
    let elaborated = build(overrides)?;
    let connected = connectivity(&elaborated.chip, &waived_at(waivers, None));
    let mut checks = vec![connected.check];
    checks.extend(
        each(processes)
            .into_iter()
            .map(|process| drawn(&elaborated, process, waivers)),
    );
    Ok(Analysis {
        elaborated,
        nets: connected.nets,
        checks,
    })
}

impl Analysis {
    /// Whether the design is sound as drawn: connected as it says, and within the rules of every
    /// process.
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
