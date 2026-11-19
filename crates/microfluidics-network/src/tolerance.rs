//! A design across the chips a process makes: at its corners, and over its spread.
//!
//! A made chip differs from its drawing in ways shared across the chip (a layer spun thick, every
//! feature of a mask a little wide) and in ways each feature has alone. The first are taken at
//! their extremes together, as corners; all are taken at random, for the yield.

use std::collections::BTreeMap;

use microfluidics_core::model::{Bound, Measure, Requirement};
use microfluidics_core::{Check, Finding};
use microfluidics_layout::Process;

use crate::analysis::Analysis;
use crate::requirement::{distances, tidy, wanted};
use crate::variation::{Variation, normal};

/// How many standard deviations a corner stands from nominal.
const CORNER: f64 = 3.0;

fn placed(analysis: &Analysis) -> Vec<&Requirement> {
    analysis
        .elaborated
        .requirements
        .iter()
        .filter(|requirement| requirement.measure != Measure::Yield)
        .collect()
}

/// How far the worst-placed value of each requirement is from holding under `variation`.
fn worst(analysis: &Analysis, variation: &Variation) -> Option<BTreeMap<String, f64>> {
    let values = analysis.values_under(variation).ok()?;
    let each = |requirement: &&Requirement| {
        Some((
            requirement.name.clone(),
            distances(&requirement.bound, values.get(&requirement.name)?)
                .into_iter()
                .fold(0.0, f64::max),
        ))
    };
    placed(analysis).iter().map(each).collect()
}

/// How one requirement fares across the corners of a process. Values are in SI units.
#[derive(Debug, Clone, PartialEq)]
pub struct Across {
    /// Its worst distance from holding, in its own tolerances: at most 1 where it holds at
    /// every corner.
    pub distance: f64,
    /// The corner it is worst at, and the value furthest from holding there.
    pub corner: String,
    pub reached: f64,
    /// The least and greatest values over every corner, and the corner of each. Values that
    /// must be equal are each taken as their share of their own corner's mean, of the mean as
    /// drawn, so that what is compared is what the requirement compares.
    pub low: f64,
    pub low_corner: String,
    pub high: f64,
    pub high_corner: String,
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

/// Each requirement across every corner of the process: each layer at its highest and lowest,
/// with every feature at its widest and narrowest. An error says why it could not be found.
pub fn across(analysis: &Analysis, process: &Process) -> Result<BTreeMap<String, Across>, String> {
    let Some(tolerances) = process.tolerances else {
        return Err("the process states no tolerances".to_owned());
    };
    if placed(analysis).is_empty() || analysis.elaborated.operating.is_none() {
        return Err("the design states no requirement on its flow".to_owned());
    }
    let drawn = analysis
        .values_under(&Variation::nominal())
        .map_err(|error| format!("the network could not be solved as drawn: {error}"))?;
    let layers: Vec<String> = analysis
        .elaborated
        .chip
        .layers
        .iter()
        .map(|layer| layer.name.clone())
        .collect();
    // One bit for each layer's height, and one for the width of everything.
    let mut held: BTreeMap<String, Across> = BTreeMap::new();
    for corner in 0..1_u32 << (layers.len() + 1) {
        let sign = |bit: usize| if corner >> bit & 1 == 0 { -1.0 } else { 1.0 };
        let bias = sign(layers.len()) * CORNER * tolerances.width_bias_sigma;
        let variation = Variation {
            height_scale: layers
                .iter()
                .enumerate()
                .map(|(at, layer)| {
                    (
                        layer.clone(),
                        1.0 + sign(at) * CORNER * tolerances.height_fraction_sigma,
                    )
                })
                .collect(),
            width_bias: layers.iter().map(|layer| (layer.clone(), bias)).collect(),
            mismatch: None,
        };
        let described = |bit: usize, high: &str, low: &str| {
            if sign(bit) > 0.0 {
                high.to_owned()
            } else {
                low.to_owned()
            }
        };
        let mut named: Vec<String> = layers
            .iter()
            .enumerate()
            .map(|(at, layer)| format!("{layer} {}", described(at, "thick", "thin")))
            .collect();
        named.push(format!("features {}", described(layers.len(), "wide", "narrow")));
        let named = named.join(", ");
        let Ok(values) = analysis.values_under(&variation) else {
            return Err(format!("the network could not be solved with {named}"));
        };
        for requirement in placed(analysis) {
            let Some(found) = values.get(&requirement.name).filter(|found| !found.is_empty()) else {
                return Err(format!("`{}` could not be measured with {named}", requirement.name));
            };
            let apart = distances(&requirement.bound, found);
            // Values that must be equal are compared with their own mean, which moves with the corner.
            let scale = match (requirement.bound, drawn.get(&requirement.name)) {
                (Bound::EqualWithin(_), Some(drawn)) if mean(found) != 0.0 => mean(drawn) / mean(found),
                _ => 1.0,
            };
            let (mut distance, mut reached) = (0.0, found[0] * scale);
            for (value, apart) in found.iter().zip(&apart) {
                if *apart > distance {
                    (distance, reached) = (*apart, value * scale);
                }
            }
            let low = found
                .iter()
                .fold(f64::INFINITY, |least, value| least.min(value * scale));
            let high = found
                .iter()
                .fold(f64::NEG_INFINITY, |most, value| most.max(value * scale));
            let entry = held.entry(requirement.name.clone()).or_insert_with(|| Across {
                distance,
                corner: named.clone(),
                reached,
                low,
                low_corner: named.clone(),
                high,
                high_corner: named.clone(),
            });
            if distance > entry.distance {
                (entry.distance, entry.reached, entry.corner) = (distance, reached, named.clone());
            }
            if low < entry.low {
                (entry.low, entry.low_corner) = (low, named.clone());
            }
            if high > entry.high {
                (entry.high, entry.high_corner) = (high, named.clone());
            }
        }
    }
    Ok(held)
}

/// Whether every requirement holds at every corner of the process, from how each fares across them.
pub fn corners_of(analysis: &Analysis, found: &Result<BTreeMap<String, Across>, String>) -> Check {
    let found = match found {
        Ok(found) => found,
        Err(why) => return Check::not_run("corners", why),
    };
    let drawn = analysis.values_under(&Variation::nominal()).unwrap_or_default();
    let mut findings = Vec::new();
    let mut metrics = BTreeMap::new();
    for requirement in placed(analysis) {
        let Some(across) = found.get(&requirement.name) else {
            continue;
        };
        let (unit, per_si) = requirement.measure.reported();
        let name = &requirement.name;
        metrics.insert(name.clone(), across.distance);
        metrics.insert(format!("{name}.low"), across.low * per_si);
        metrics.insert(format!("{name}.high"), across.high * per_si);
        if across.distance > 1.0 + 1e-9 {
            let numbers = drawn.get(name).cloned().unwrap_or_default();
            let message = format!(
                "`{name}` reaches {}{unit} with {}, and must be {}",
                tidy(across.reached * per_si),
                across.corner,
                wanted(requirement, &numbers)
            );
            findings.push(Finding {
                rule: name.clone(),
                subject: across.corner.clone(),
                measured: Some(across.distance),
                limit: Some(1.0),
                message,
                waived: None,
            });
        }
    }
    let mut check = Check::from_findings("corners", findings, metrics, &[]);
    check.note = Some(format!(
        "each requirement at every corner of the process: each layer {CORNER} standard deviations thick and thin, with every feature as much wide and narrow"
    ));
    check
}

/// Whether every requirement holds at every corner of the process: each layer at its highest
/// and lowest, with every feature at its widest and narrowest.
pub fn corners(analysis: &Analysis, process: &Process) -> Check {
    corners_of(analysis, &across(analysis, process))
}

/// The fraction of chips, made with every tolerance at random, that meet every requirement.
pub fn yield_(analysis: &Analysis, process: &Process, samples: usize, seed: u64) -> Check {
    let Some(tolerances) = process.tolerances else {
        return Check::not_run("yield", "the process states no tolerances");
    };
    if placed(analysis).is_empty() || analysis.elaborated.operating.is_none() {
        return Check::not_run("yield", "the design states no requirement on its flow");
    }
    let layers: Vec<String> = analysis
        .elaborated
        .chip
        .layers
        .iter()
        .map(|layer| layer.name.clone())
        .collect();
    let mut good = 0;
    for sample in 0..samples as u64 {
        // Each chip draws its own values: two for each layer, and a seed for its features.
        let draw = |index: u64| normal(seed, sample * 1000 + index);
        let variation = Variation {
            height_scale: layers
                .iter()
                .enumerate()
                .map(|(at, layer)| (layer.clone(), 1.0 + tolerances.height_fraction_sigma * draw(at as u64)))
                .collect(),
            width_bias: layers
                .iter()
                .enumerate()
                .map(|(at, layer)| (layer.clone(), tolerances.width_bias_sigma * draw(100 + at as u64)))
                .collect(),
            mismatch: Some((
                tolerances.width_sigma,
                seed.wrapping_add(sample).wrapping_mul(0x9E37_79B9),
            )),
        };
        good += usize::from(
            worst(analysis, &variation).is_some_and(|found| found.values().all(|distance| *distance <= 1.0 + 1e-9)),
        );
    }
    let fraction = good as f64 / samples.max(1) as f64;
    let wanted = analysis
        .elaborated
        .requirements
        .iter()
        .find(|requirement| requirement.measure == Measure::Yield);
    let mut findings = Vec::new();
    if let Some((requirement, Bound::AtLeast(least))) = wanted.map(|requirement| (requirement, requirement.bound))
        && fraction < least
    {
        let message = format!(
            "{}: {:.1}% of chips meet every requirement, and {:.1}% must",
            requirement.statement,
            fraction * 100.0,
            least * 100.0
        );
        findings.push(Finding {
            rule: requirement.name.clone(),
            subject: "chip".to_owned(),
            measured: Some(fraction * 100.0),
            limit: Some(least * 100.0),
            message,
            waived: None,
        });
    }
    let metrics = [
        ("yield_percent".to_owned(), fraction * 100.0),
        ("chips".to_owned(), samples as f64),
    ]
    .into();
    let mut check = Check::from_findings("yield", findings, metrics, &[]);
    check.note = Some("the share of chips meeting every requirement, with each layer's height, each mask's widths and each feature's width at random".to_owned());
    check
}
