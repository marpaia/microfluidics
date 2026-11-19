//! Whether a mask prints what was drawn.
//!
//! The exposure a mask gives is taken as the mask blurred by a Gaussian, and resist as staying
//! where the exposure passes a threshold. For rectangles the blur has a closed form, so a
//! feature's printed width is found along a line across it, with its neighbours' light
//! included. Only features narrow enough for the blur to matter are looked at.

use std::collections::BTreeMap;

use microfluidics_core::{Check, Finding};

use crate::chip::Chip;
use crate::geometry::{Rect, TOLERANCE};
use crate::mask::{Mask, MaskSet};
use crate::maskspec::{PrintModel, Tone};
use crate::process::Process;

/// A feature narrower than this many blurs is looked at.
const CRITICAL: f64 = 8.0;
/// Light from further than this many blurs is nothing.
const REACH: f64 = 4.0;
/// How many of the worst features a check names.
const NAMED: usize = 20;

/// The error function, to within 1.5e-7.
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let series =
        t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    (1.0 - series * (-x * x).exp()).copysign(x)
}

/// The share of full exposure a blurred edge-to-edge opening `min..max` gives at `at`.
fn opening(at: f64, min: f64, max: f64, blur: f64) -> f64 {
    let scale = blur * std::f64::consts::SQRT_2;
    0.5 * (erf((at - min) / scale) - erf((at - max) / scale))
}

/// The exposure at a point from every rectangle of `lit`, as a share of full exposure.
fn exposure(x: f64, y: f64, lit: &[Rect], blur: f64) -> f64 {
    let each = |rect: &Rect| opening(x, rect.min.x, rect.max.x, blur) * opening(y, rect.min.y, rect.max.y, blur);
    lit.iter().map(each).sum::<f64>().min(1.0)
}

/// What one feature prints as.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Printed {
    /// Its width across its narrow way, and how far its end prints short of the mask's.
    Width { width: f64, pullback: f64 },
    /// Nothing prints.
    Vanished,
    /// It prints joined to a feature it is drawn apart from.
    Bridged,
    /// It is drawn joined to another feature across its narrow way, so it has no width of its own.
    Joined,
}

/// Where the exposure along a line crosses `level`, between `inside` (above it) and `outside`.
fn crossing(level: f64, inside: f64, outside: f64, exposure_at: &dyn Fn(f64) -> f64) -> Option<f64> {
    if exposure_at(outside) >= level {
        return None;
    }
    let (mut high, mut low) = (inside, outside);
    for _ in 0..60 {
        let middle = (high + low) / 2.0;
        if exposure_at(middle) >= level {
            high = middle;
        } else {
            low = middle;
        }
    }
    Some((high + low) / 2.0)
}

/// What `feature` prints as beside `near`, the other features within reach of it.
fn printed(feature: &Rect, near: &[Rect], model: &PrintModel) -> Printed {
    let mut lit = vec![*feature];
    lit.extend_from_slice(near);
    let upright = feature.width() <= feature.height();
    let (middle_x, middle_y) = (
        (feature.min.x + feature.max.x) / 2.0,
        (feature.min.y + feature.max.y) / 2.0,
    );
    // Across the narrow way through the middle, and along the long way from the middle to an end.
    let across = |at: f64| {
        if upright {
            exposure(at, middle_y, &lit, model.blur)
        } else {
            exposure(middle_x, at, &lit, model.blur)
        }
    };
    let along = |at: f64| {
        if upright {
            exposure(middle_x, at, &lit, model.blur)
        } else {
            exposure(at, middle_y, &lit, model.blur)
        }
    };
    let (low, high, middle) = if upright {
        (feature.min.x, feature.max.x, middle_x)
    } else {
        (feature.min.y, feature.max.y, middle_y)
    };
    let (end, long_middle) = if upright {
        (feature.max.y, middle_y)
    } else {
        (feature.max.x, middle_x)
    };
    if across(middle) < model.threshold {
        return Printed::Vanished;
    }
    let reach = REACH * model.blur;
    let edges = (
        crossing(model.threshold, middle, low - reach, &across),
        crossing(model.threshold, middle, high + reach, &across),
    );
    let (Some(left), Some(right)) = edges else {
        // Light all the way out: a neighbour it touches there, or one it prints into.
        let touching = near.iter().any(|other| {
            let (gap_x, gap_y) = (
                (other.min.x - feature.max.x).max(feature.min.x - other.max.x),
                (other.min.y - feature.max.y).max(feature.min.y - other.max.y),
            );
            gap_x <= TOLERANCE && gap_y <= TOLERANCE
        });
        return if touching { Printed::Joined } else { Printed::Bridged };
    };
    // The printed edges reach a feature it is drawn apart from: the two print as one.
    let apart = |other: &&Rect| {
        let gap_x = (other.min.x - feature.max.x).max(feature.min.x - other.max.x);
        let gap_y = (other.min.y - feature.max.y).max(feature.min.y - other.max.y);
        gap_x > TOLERANCE || gap_y > TOLERANCE
    };
    let reached = |other: &Rect| {
        let (across_low, across_high, along_low, along_high) = if upright {
            (other.min.x, other.max.x, other.min.y, other.max.y)
        } else {
            (other.min.y, other.max.y, other.min.x, other.max.x)
        };
        along_low <= long_middle && long_middle <= along_high && across_low < right && across_high > left
    };
    if near.iter().filter(apart).any(reached) {
        return Printed::Bridged;
    }
    let pullback = crossing(model.threshold, long_middle, end + reach, &along).map_or(0.0, |printed| end - printed);
    Printed::Width {
        width: right - left,
        pullback,
    }
}

/// The rectangles of one mask, and for each narrow one those within reach of it.
fn narrow_with_neighbours(rects: &[Rect], model: &PrintModel) -> Vec<(usize, Vec<Rect>)> {
    let reach = REACH * model.blur;
    let narrow = |rect: &Rect| rect.width().min(rect.height());
    // Small features are found by a grid; the few large ones are looked at every time.
    let small = 64.0 * CRITICAL * model.blur;
    let structure = small + 2.0 * reach;
    let key = |rect: &Rect| {
        (
            ((rect.min.x + rect.max.x) / 2.0 / structure).floor() as i64,
            ((rect.min.y + rect.max.y) / 2.0 / structure).floor() as i64,
        )
    };
    let is_small = |rect: &Rect| rect.width().max(rect.height()) <= small;
    let mut grid: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
    let mut large = Vec::new();
    for (at, rect) in rects.iter().enumerate() {
        if is_small(rect) {
            grid.entry(key(rect)).or_default().push(at);
        } else {
            large.push(at);
        }
    }
    let within = |a: &Rect, b: &Rect| {
        b.min.x - a.max.x <= reach
            && a.min.x - b.max.x <= reach
            && b.min.y - a.max.y <= reach
            && a.min.y - b.max.y <= reach
    };
    let mut found = Vec::new();
    for (at, rect) in rects
        .iter()
        .enumerate()
        .filter(|(_, rect)| narrow(rect) < CRITICAL * model.blur)
    {
        let mut candidates: Vec<usize> = large.clone();
        if is_small(rect) {
            let (column, row) = key(rect);
            for (dx, dy) in (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (dx, dy))) {
                candidates.extend(grid.get(&(column + dx, row + dy)).into_iter().flatten());
            }
        } else {
            candidates.extend(grid.values().flatten());
        }
        let near = candidates
            .into_iter()
            .filter(|other| *other != at && within(rect, &rects[*other]))
            .map(|other| rects[other])
            .collect();
        found.push((at, near));
    }
    found
}

/// One mask's narrow features as they print: the findings, and what was measured.
fn mask_check(
    masks: &MaskSet,
    mask: &Mask,
    model: &PrintModel,
    findings: &mut Vec<Finding>,
    metrics: &mut BTreeMap<String, f64>,
) {
    let rectangles = masks.flattened(mask);
    let rects: Vec<Rect> = rectangles.iter().map(|rectangle| rectangle.bounds()).collect();
    // A dark mask shadows its features: resist stays where the light, not the shadow, falls short.
    let model = &PrintModel {
        threshold: if mask.spec.tone == Tone::Dark {
            1.0 - model.threshold
        } else {
            model.threshold
        },
        ..*model
    };
    let nm = |metres: f64| (metres * 1e9).round() as i64;
    let mut known: BTreeMap<Vec<i64>, Printed> = BTreeMap::new();
    let mut failing: Vec<(f64, Finding)> = Vec::new();
    let (mut checked, mut worst, mut narrowest): (usize, f64, Option<(f64, f64, f64)>) = (0, 0.0, None);
    for (at, near) in narrow_with_neighbours(&rects, model) {
        let rect = rects[at];
        // Features alike, with neighbours alike, print alike.
        let mut alike: Vec<[i64; 4]> = near
            .iter()
            .map(|other| {
                [
                    nm(other.min.x - rect.min.x),
                    nm(other.min.y - rect.min.y),
                    nm(other.max.x - rect.min.x),
                    nm(other.max.y - rect.min.y),
                ]
            })
            .collect();
        alike.sort_unstable();
        let mut signature = vec![nm(rect.width()), nm(rect.height())];
        signature.extend(alike.into_iter().flatten());
        let outcome = *known.entry(signature).or_insert_with(|| printed(&rect, &near, model));
        let drawn = rect.width().min(rect.height()) - 2.0 * mask.spec.bias;
        let fault = |message: String, measured: Option<f64>| Finding {
            rule: format!("print.{}", mask.layer.name),
            subject: rectangles[at].name.clone(),
            measured,
            limit: Some(drawn * 1e6),
            message,
            waived: None,
        };
        checked += 1;
        let name = &rectangles[at].name;
        match outcome {
            Printed::Joined => checked -= 1,
            Printed::Vanished => failing.push((
                f64::INFINITY,
                fault(
                    format!("{name}, drawn {:.2} um wide, does not print at all", drawn * 1e6),
                    Some(0.0),
                ),
            )),
            Printed::Bridged => failing.push((
                f64::INFINITY,
                fault(
                    format!("{name} prints joined to a feature it is drawn apart from"),
                    None,
                ),
            )),
            Printed::Width { width, pullback } => {
                let error = width - drawn;
                worst = if error.abs() > worst.abs() { error } else { worst };
                if narrowest.is_none_or(|(least, _, _)| drawn < least) {
                    narrowest = Some((drawn, width, pullback));
                }
                if error.abs() > model.tolerance {
                    let message = format!(
                        "{name} prints {:.2} um wide, and is drawn {:.2} um; the process holds {:.2} um",
                        width * 1e6,
                        drawn * 1e6,
                        model.tolerance * 1e6
                    );
                    failing.push((error.abs(), fault(message, Some(width * 1e6))));
                }
            }
        }
    }
    let layer = &mask.layer.name;
    metrics.insert(format!("{layer}.checked"), checked as f64);
    metrics.insert(format!("{layer}.failing"), failing.len() as f64);
    metrics.insert(format!("{layer}.worst_error_um"), worst * 1e6);
    if let Some((drawn, width, pullback)) = narrowest {
        metrics.insert(format!("{layer}.narrowest_drawn_um"), drawn * 1e6);
        metrics.insert(format!("{layer}.narrowest_printed_um"), width * 1e6);
        metrics.insert(format!("{layer}.narrowest_end_pullback_um"), pullback * 1e6);
    }
    failing.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.subject.cmp(&b.1.subject)));
    findings.extend(failing.into_iter().take(NAMED).map(|(_, finding)| finding));
}

/// Whether every narrow feature of `masks` prints at its drawn width, within the model's
/// tolerance, without vanishing or joining a neighbour.
pub fn check(masks: &MaskSet, model: &PrintModel) -> Check {
    let mut findings = Vec::new();
    let mut metrics = BTreeMap::new();
    for mask in &masks.masks {
        mask_check(masks, mask, model, &mut findings, &mut metrics);
    }
    let mut check = Check::from_findings("print", findings, metrics, &[]);
    let fitted = if model.calibrated {
        "a model fitted to this process"
    } else {
        "an uncalibrated model: its blur and threshold are not measurements of this process"
    };
    check.note = Some(format!(
        "features narrower than {:.1} um, by {fitted}; each layer names its {NAMED} worst",
        CRITICAL * model.blur * 1e6
    ));
    check
}

/// For each layer, the bias at which its narrowest feature, standing alone, prints at its drawn
/// width. Empty when the process has no print model; a layer whose narrowest feature cannot be
/// made to print is left out.
pub fn solve_bias(chip: &Chip, process: &Process) -> BTreeMap<String, f64> {
    let Some(model) = process.print else {
        return BTreeMap::new();
    };
    let flat = chip.layout.flattened();
    let mut solved = BTreeMap::new();
    for layer in chip.layers.iter() {
        let widths = flat
            .iter()
            .filter(|rectangle| rectangle.layer == layer.name)
            .map(|rectangle| rectangle.bounds());
        let Some(drawn) = widths
            .map(|rect| rect.width().min(rect.height()))
            .min_by(f64::total_cmp)
        else {
            continue;
        };
        let level = if process.mask(&layer.name).tone == Tone::Dark {
            1.0 - model.threshold
        } else {
            model.threshold
        };
        let model = PrintModel {
            threshold: level,
            ..model
        };
        // A long line of the drawn width, with its mask edges moved by `bias`.
        let error = |bias: f64| {
            let line = Rect {
                min: crate::geometry::Point::new(-drawn / 2.0 - bias, -1e3 * model.blur),
                max: crate::geometry::Point::new(drawn / 2.0 + bias, 1e3 * model.blur),
            };
            match printed(&line, &[], &model) {
                Printed::Width { width, .. } => width - drawn,
                _ => -drawn,
            }
        };
        let (mut low, mut high) = (-drawn / 2.0 + 1e-9, CRITICAL * model.blur);
        if error(high) < 0.0 {
            continue;
        }
        for _ in 0..60 {
            let middle = (low + high) / 2.0;
            if error(middle) < 0.0 {
                low = middle;
            } else {
                high = middle;
            }
        }
        solved.insert(layer.name.clone(), (high * 1e9).round() * 1e-9);
    }
    solved
}

#[cfg(test)]
mod tests {
    use crate::fixtures::chip;
    use crate::mask::prepare;
    use crate::maskspec::PrintModel;
    use crate::print::{check, erf, solve_bias};
    use crate::process::Process;

    const MODEL: PrintModel = PrintModel {
        blur: 0.4e-6,
        threshold: 0.5,
        tolerance: 0.1e-6,
        calibrated: false,
    };

    #[test]
    fn the_error_function_is_the_error_function() {
        for (x, value) in [
            (0.0, 0.0),
            (0.5, 0.520_499_877_8),
            (1.0, 0.842_700_792_9),
            (2.0, 0.995_322_265),
        ] {
            assert!((erf(x) - value).abs() < 2e-7 && (erf(-x) + value).abs() < 2e-7, "{x}");
        }
    }

    #[test]
    fn a_wide_line_is_not_looked_at_a_fine_one_prints_and_a_hair_does_not() {
        // Lines 4, 1.2 and 0.5 um wide, far apart.
        let rectangles = [
            ("wide", "flow", [100.0, 100.0, 104.0, 900.0]),
            ("fine", "flow", [200.0, 100.0, 201.2, 900.0]),
            ("hair", "flow", [300.0, 100.0, 300.5, 900.0]),
        ];
        let masks = prepare(&chip([2.0, 2.0], &rectangles, &[], &[]), None);
        let found = check(&masks, &MODEL);
        assert_eq!(found.metrics["flow.checked"], 2.0, "{found:?}");
        let said: Vec<(&str, &str)> = (found.findings.iter())
            .map(|finding| (finding.subject.as_str(), finding.message.as_str()))
            .collect();
        assert_eq!(
            said,
            [("hair", "hair, drawn 0.50 um wide, does not print at all")],
            "{found:?}"
        );
        // The fine line prints a little narrow, within tolerance, and short at its ends.
        assert!((found.metrics["flow.narrowest_drawn_um"] - 1.2).abs() < 1e-9);
        let printed = found.metrics["flow.narrowest_printed_um"];
        assert!(printed < 1.2 && printed > 1.1, "{printed}");
        assert!(found.metrics["flow.narrowest_end_pullback_um"] > 0.0);
        assert!(found.note.as_deref().is_some_and(|note| note.contains("uncalibrated")));
    }

    #[test]
    fn lines_drawn_too_close_print_as_one() {
        // Two 1.2 um lines 0.4 um apart, and two 3 um apart.
        let rectangles = [
            ("a", "flow", [100.0, 100.0, 101.2, 900.0]),
            ("b", "flow", [101.6, 100.0, 102.8, 900.0]),
            ("c", "flow", [200.0, 100.0, 201.2, 900.0]),
            ("d", "flow", [204.2, 100.0, 205.4, 900.0]),
        ];
        let found = check(&prepare(&chip([2.0, 2.0], &rectangles, &[], &[]), None), &MODEL);
        let joined: Vec<&str> = (found.findings.iter())
            .filter(|finding| finding.message.contains("prints joined"))
            .map(|finding| finding.subject.as_str())
            .collect();
        assert_eq!(joined, ["a", "b"], "{found:?}");
    }

    #[test]
    fn the_bias_that_prints_a_narrow_line_at_its_drawn_width_is_found() {
        // At a threshold above a half a narrow line prints narrow, and a bias restores it.
        let text = "name = \"p\"\n[print]\nblur_um = 0.4\nthreshold = 0.6\ntolerance_um = 0.1\n";
        let process = Process::parse("p.toml", text).unwrap();
        let drawn = chip([2.0, 2.0], &[("fine", "flow", [200.0, 100.0, 201.2, 900.0])], &[], &[]);
        let unbiased = check(&prepare(&drawn, Some(&process)), &process.print.unwrap());
        assert!(unbiased.metrics["flow.worst_error_um"] < -0.1, "{unbiased:?}");
        let bias = solve_bias(&drawn, &process)["flow"];
        assert!(bias > 0.05e-6 && bias < 0.4e-6, "{bias}");
        let biased = format!("{text}[[masks]]\nlayer = \"flow\"\nbias_um = {}\n", bias * 1e6);
        let process = Process::parse("p.toml", &biased).unwrap();
        let printed = check(&prepare(&drawn, Some(&process)), &process.print.unwrap());
        assert!(
            printed.passes() && printed.metrics["flow.worst_error_um"].abs() < 0.005,
            "{printed:?}"
        );
    }
}
