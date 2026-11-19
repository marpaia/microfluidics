//! Whether a solved network does what a requirement asks. This is the only place a bound is
//! interpreted.

use std::collections::BTreeMap;

use microfluidics_core::model::{Bound, Measure, Requirement};
use microfluidics_core::{Check, Finding};

use crate::error::NetworkError;
use crate::solve::Solution;

/// The value of a requirement's measure at each of its subjects, in SI units.
pub fn values(requirement: &Requirement, solution: &Solution) -> Result<Vec<(String, f64)>, NetworkError> {
    if requirement.measure == Measure::Drop {
        let pairs = requirement.subjects.chunks(2).filter(|pair| pair.len() == 2);
        return pairs
            .map(|pair| {
                Ok((
                    format!("{} to {}", pair[0], pair[1]),
                    solution.drop(&pair[0], &pair[1])?,
                ))
            })
            .collect();
    }
    (requirement.subjects.iter())
        .map(|subject| Ok((subject.clone(), solution.quantity(&requirement.measure, subject)?)))
        .collect()
}

/// How far each value is from what the bound allows, in units of the bound's own tolerance: at
/// most 1 where the bound holds. A bound with no tolerance counts each unit of its limit as one.
pub fn distances(bound: &Bound, values: &[f64]) -> Vec<f64> {
    let mean = values.iter().sum::<f64>() / values.len().max(1) as f64;
    let each = |value: &f64| match *bound {
        Bound::EqualWithin(within) => (value - mean).abs() / (within * mean.abs()).max(f64::MIN_POSITIVE),
        Bound::Is {
            value: wanted,
            within,
            relative,
        } => {
            let allowed = if relative { within * wanted.abs() } else { within };
            (value - wanted).abs() / allowed.max(f64::MIN_POSITIVE)
        }
        Bound::AtMost(most) => value / most.abs().max(f64::MIN_POSITIVE),
        Bound::AtLeast(least) => 2.0 - value / least.abs().max(f64::MIN_POSITIVE),
    };
    values.iter().map(each).collect()
}

/// A number as a person reads it: four significant figures, with no zeros trailing.
pub fn tidy(value: f64) -> String {
    if value == 0.0 || !value.is_finite() {
        return format!("{value}");
    }
    let digits = (3 - value.abs().log10().floor() as i32).clamp(0, 12) as usize;
    let written = format!("{value:.digits$}");
    if written.contains('.') {
        written.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        written
    }
}

/// What a requirement asks of its values, in words, given the values as drawn in SI units.
pub fn wanted(requirement: &Requirement, numbers: &[f64]) -> String {
    let (unit, per_si) = requirement.measure.reported();
    let shown = |value: f64| format!("{}{unit}", tidy(value * per_si));
    match requirement.bound {
        Bound::EqualWithin(within) => format!(
            "within {}% of their mean, {}",
            tidy(within * 100.0),
            shown(numbers.iter().sum::<f64>() / numbers.len().max(1) as f64)
        ),
        Bound::Is {
            value,
            within,
            relative: true,
        } => format!("{} within {}%", shown(value), tidy(within * 100.0)),
        Bound::Is {
            value,
            within,
            relative: false,
        } => format!("{} within {}", shown(value), shown(within)),
        Bound::AtMost(most) => format!("at most {}", shown(most)),
        Bound::AtLeast(least) => format!("at least {}", shown(least)),
    }
}

/// Check one requirement against a solution.
pub fn evaluate(requirement: &Requirement, solution: &Solution) -> Check {
    let id = format!("requirement/{}", requirement.name);
    let measured = match values(requirement, solution) {
        Ok(measured) => measured,
        Err(error) => return Check::not_run(&id, &format!("{}: {error}", requirement.statement)),
    };
    let (unit, per_si) = requirement.measure.reported();
    let numbers: Vec<f64> = measured.iter().map(|(_, value)| *value).collect();
    let shown = |value: f64| format!("{}{unit}", tidy(value * per_si));
    let wanted = wanted(requirement, &numbers);
    let mut findings = Vec::new();
    let mut metrics: BTreeMap<String, f64> = BTreeMap::new();
    for ((subject, value), distance) in measured.iter().zip(distances(&requirement.bound, &numbers)) {
        metrics.insert(subject.clone(), value * per_si);
        if distance > 1.0 + 1e-9 {
            findings.push(Finding {
                rule: requirement.name.clone(),
                subject: subject.clone(),
                measured: Some(value * per_si),
                limit: None,
                message: format!(
                    "{}: {subject} is {}, and must be {wanted}",
                    requirement.statement,
                    shown(*value)
                ),
                waived: None,
            });
        }
    }
    let mut check = Check::from_findings(&id, findings, metrics, &[]);
    check.note = Some(format!(
        "{}; values in {}",
        requirement.statement,
        if unit.is_empty() { "plain numbers" } else { unit }
    ));
    check
}
