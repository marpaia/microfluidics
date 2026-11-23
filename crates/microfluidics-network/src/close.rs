//! Closure: changing what a design allows to be changed until its requirements hold.
//!
//! Each trial is the whole design made again with other values: elaborated, checked against the
//! rules of every process it names and its own connections, its network extracted and solved. A trial that breaks a rule
//! or cannot be made is never moved to. Source is not rewritten; the result is a set of
//! overrides.

use std::collections::BTreeMap;

use microfluidics_core::model::{Bound, Measure, Requirement};
use microfluidics_core::{Dim, Quantity};
use microfluidics_core::{Finding, Waiver};
use microfluidics_layout::{Build, Overrides, Process, Tunable};

use crate::analysis::{AnalysisError, analyse};
use crate::optimise::least_squares;
use crate::requirement::distances;
use crate::variation::Variation;

/// The most times the search may move.
const ITERATIONS: usize = 60;
/// How far inside a limit, in units of its tolerance, the search goes before it stops pulling.
const MARGIN: f64 = 0.5;
/// The grid masks are written on, in metres.
const GRID: f64 = 1e-9;
/// How strongly the search prefers to stay near where it began: the smallest change wins.
const STAY: f64 = 0.02;

/// One parameter the closure changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub path: String,
    pub from: Quantity,
    pub to: Quantity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    /// The overrides under which the design was left: those it began with, and what was tuned.
    pub overrides: Overrides,
    pub changes: Vec<Change>,
    /// Whether every requirement now holds.
    pub converged: bool,
    pub iterations: usize,
    /// How far each requirement is from its bound, in units of its tolerance: at most 1 holds.
    pub distances: BTreeMap<String, f64>,
    /// When not converged: the requirement furthest from holding, and what each tunable does to it.
    pub limiting: Option<Finding>,
}

/// The value of tunable `tunable` at `at` within 0..1.
fn value_at(tunable: &Tunable, at: f64) -> Quantity {
    let si = tunable.low.si + at.clamp(0.0, 1.0) * (tunable.high.si - tunable.low.si);
    // A count is whole, and a length lies on the masks' grid.
    let si = match tunable.value.dim {
        _ if tunable.count => si.round(),
        Dim::LENGTH => (si / GRID).round() * GRID,
        _ => si,
    };
    // The value is the one its written form reads back as, so that the tuned file holds exactly
    // what was checked.
    let value = Quantity {
        si,
        dim: tunable.value.dim,
    };
    Quantity::parse(&value.written()).unwrap_or(value)
}

/// Tune `design` from the overrides `start` until its requirements hold, or as near as it gets.
pub fn close(
    build: Build<'_>,
    start: &Overrides,
    processes: &[Process],
    waivers: &[Waiver],
) -> Result<Closure, AnalysisError> {
    let first = analyse(build, start, processes, waivers)?;
    let tunables = first.elaborated.tunables.clone();
    let width = |tunable: &Tunable| (tunable.high.si - tunable.low.si).max(f64::MIN_POSITIVE);
    let begun: Vec<f64> = tunables
        .iter()
        .map(|tunable| ((tunable.value.si - tunable.low.si) / width(tunable)).clamp(0.0, 1.0))
        .collect();
    // A count moves by whole steps; anything else by a thousandth of its range.
    let steps: Vec<f64> = tunables
        .iter()
        .map(|tunable| if tunable.count { 1.0 / width(tunable) } else { 1e-3 })
        .collect();
    let overrides_at = |x: &[f64]| {
        let mut overrides = start.clone();
        overrides.0.extend(
            tunables
                .iter()
                .zip(x)
                .map(|(tunable, at)| (tunable.path.clone(), value_at(tunable, *at))),
        );
        overrides
    };
    // How far each requirement is from holding at `x`; `None` where the design cannot be made.
    let measure = |x: &[f64]| -> Option<BTreeMap<String, (Bound, f64)>> {
        let analysis = analyse(build, &overrides_at(x), processes, waivers)
            .ok()
            .filter(|analysis| analysis.sound())?;
        let values = analysis.values_under(&Variation::nominal()).ok()?;
        let placed = analysis
            .elaborated
            .requirements
            .iter()
            .filter(|requirement| requirement.measure != Measure::Yield);
        let worst = |requirement: &Requirement| {
            let each = distances(&requirement.bound, &values[&requirement.name]);
            (
                requirement.name.clone(),
                (requirement.bound, each.into_iter().fold(0.0, f64::max)),
            )
        };
        Some(placed.map(worst).collect())
    };
    // A value to be met pulls all the way to it. A limit, or a spread to stay within, pulls only
    // until it holds with margin, so that the design is changed no more than it needs.
    let pull = |(bound, worst): (Bound, f64)| match bound {
        Bound::Is { .. } => worst,
        Bound::EqualWithin(_) | Bound::AtMost(_) | Bound::AtLeast(_) => (worst - MARGIN).max(0.0),
    };
    let mut residuals = |x: &[f64]| {
        let mut all: Vec<f64> = measure(x)?.into_values().map(pull).collect();
        all.extend(x.iter().zip(&begun).map(|(now, then)| STAY * (now - then)));
        Some(all)
    };

    let fit = least_squares(&begun, &steps, &mut residuals, ITERATIONS);
    let (x, iterations) = fit.map_or((begun.clone(), 0), |fit| (fit.x, fit.iterations));
    let overrides = overrides_at(&x);
    let analysis = analyse(build, &overrides, processes, waivers)?;
    let values = analysis.values_under(&Variation::nominal())?;
    let mut held = BTreeMap::new();
    for requirement in analysis
        .elaborated
        .requirements
        .iter()
        .filter(|requirement| requirement.measure != Measure::Yield)
    {
        let worst = distances(&requirement.bound, &values[&requirement.name])
            .into_iter()
            .fold(0.0, f64::max);
        held.insert(requirement.name.clone(), worst);
    }
    let converged = analysis.sound() && held.values().all(|distance| *distance <= 1.0 + 1e-9);
    let changes = (tunables.iter().zip(&x))
        .map(|(tunable, at)| Change {
            path: tunable.path.clone(),
            from: tunable.value,
            to: value_at(tunable, *at),
        })
        .filter(|change| (change.to.si - change.from.si).abs() > 1e-12 * change.from.si.abs().max(1e-12))
        .collect();

    let limiting = (!converged).then(|| {
        // A design that breaks a rule or a connection as it stands has nowhere to tune from.
        let broken = analysis
            .checks
            .iter()
            .filter(|check| !check.passes())
            .flat_map(|check| &check.findings)
            .find(|finding| finding.waived.is_none());
        if let Some(broken) = broken {
            let message = format!(
                "nothing can be tuned while the design breaks a rule or a connection: {}",
                broken.message
            );
            return Finding {
                message,
                ..broken.clone()
            };
        }
        let worst = held
            .iter()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(name, distance)| (name.clone(), *distance));
        let Some((name, distance)) = worst else {
            let message =
                "the design breaks a rule or a connection as it stands, so no requirement could be tuned toward"
                    .to_owned();
            return Finding {
                rule: "closure".to_owned(),
                subject: analysis.elaborated.name.clone(),
                measured: None,
                limit: None,
                message,
                waived: None,
            };
        };
        // Why the design cannot be made at `x`: the fault, or the first rule or connection broken.
        let stopped = |x: &[f64]| match analyse(build, &overrides_at(x), processes, waivers) {
            Err(fault) => fault.to_string(),
            Ok(there) => {
                let found = there
                    .checks
                    .iter()
                    .flat_map(|check| &check.findings)
                    .find(|finding| finding.waived.is_none());
                found.map_or_else(
                    || "its flow cannot be solved".to_owned(),
                    |finding| format!("{} (rule `{}`)", finding.message, finding.rule),
                )
            }
        };
        // What moving each tunable one step, each way, does to that requirement.
        let mut effects = Vec::new();
        for (at, tunable) in tunables.iter().enumerate() {
            let mut helps = None;
            let mut blocked = Vec::new();
            for (way, step) in [("raising", steps[at]), ("lowering", -steps[at])] {
                let mut moved = x.clone();
                moved[at] += step;
                if !(0.0..=1.0).contains(&moved[at]) {
                    blocked.push(format!(
                        "it is at the {} of its range",
                        if step > 0.0 { "top" } else { "bottom" }
                    ));
                    continue;
                }
                match measure(&moved).and_then(|there| Some(there.get(&name)?.1 - distance)) {
                    Some(effect) if effect < -1e-9 => helps = helps.or(Some(way)),
                    Some(_) => {}
                    None => blocked.push(format!("{way} it breaks the design: {}", stopped(&moved))),
                }
            }
            effects.push(match (helps, blocked.is_empty()) {
                (Some(way), _) => format!("{way} {} would help, but not enough alone", tunable.path),
                (None, true) => format!("{} does nothing for it either way", tunable.path),
                (None, false) => format!("{} cannot help: {}", tunable.path, blocked.join(", and ")),
            });
        }
        let tuned = if effects.is_empty() {
            "the design names nothing to tune".to_owned()
        } else {
            effects.join("; ")
        };
        let message = format!(
            "`{name}` is {distance:.2} times its tolerance from holding, and tuning cannot bring it nearer: {tuned}"
        );
        Finding {
            rule: name,
            subject: analysis.elaborated.name.clone(),
            measured: Some(distance),
            limit: Some(1.0),
            message,
            waived: None,
        }
    });
    Ok(Closure {
        overrides,
        changes,
        converged,
        iterations,
        distances: held,
        limiting,
    })
}
