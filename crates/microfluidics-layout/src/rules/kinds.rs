//! What each kind of rule finds broken. This is the only place a rule's kind is interpreted.

use crate::geometry::{Rect, TOLERANCE};
use crate::process::{Rule, RuleKind};
use crate::rules::pairs::{penetration, within};
use crate::rules::{Broken, Scene};

const UM: f64 = 1e6;

/// Every breach of `rule`, and the most extreme value measured for it.
pub(crate) fn breaches(rule: &Rule, scene: &Scene) -> (Vec<Broken>, Option<f64>) {
    let (features, pairs) = (scene.features, &scene.pairs);
    let (rectangles, bounds): (_, &[Rect]) = (&features.rectangles, &scene.bounds);
    let narrow = |at: usize| bounds[at].width().min(bounds[at].height());
    let height = |at: usize| features.layers.height(&rectangles[at].layer);
    let on = |at: usize, layer: &Option<String>| layer.as_deref().is_none_or(|layer| rectangles[at].layer == layer);
    // An access point is known by the smallest feature it lies in, such as `inlet.pad`.
    let access = |point: &crate::geometry::Point| {
        let holds = |at: &usize| within(&bounds[*at], *point);
        let smallest = (0..rectangles.len())
            .filter(holds)
            .min_by(|a, b| rectangles[*a].area().total_cmp(&rectangles[*b].area()));
        smallest.map_or_else(
            || format!("the access point at ({:.0}, {:.0}) um", point.x * UM, point.y * UM),
            |at| format!("the access point in {}", rectangles[at].name),
        )
    };
    let mut broken: Vec<Broken> = Vec::new();
    let mut extreme: Option<f64> = None;
    let mut note = |value: f64, least: bool| {
        extreme = Some(extreme.map_or(value, |held: f64| if least { held.min(value) } else { held.max(value) }));
    };
    match &rule.kind {
        RuleKind::Width { layer, min } => {
            for at in (0..rectangles.len()).filter(|at| on(*at, layer)) {
                note(narrow(at) * UM, true);
                if narrow(at) < min - 1e-15 {
                    let message = format!(
                        "{} is {:.3} um wide; the process makes nothing under {:.3} um",
                        rectangles[at].name,
                        narrow(at) * UM,
                        min * UM
                    );
                    broken.push((
                        min - narrow(at),
                        rectangles[at].name.clone(),
                        Some(narrow(at) * UM),
                        Some(min * UM),
                        message,
                    ));
                }
            }
        }
        RuleKind::Space { layer, min } => {
            let mut seen = false;
            for &(gap, a, b) in pairs.close.iter().filter(|(_, a, _)| on(*a, layer)) {
                seen = true;
                note(gap * UM, true);
                if gap < min - 1e-15 {
                    let message = format!(
                        "{} and {} are {:.3} um apart; the process needs {:.3} um between features",
                        rectangles[a].name,
                        rectangles[b].name,
                        gap * UM,
                        min * UM
                    );
                    broken.push((
                        min - gap,
                        rectangles[a].name.clone(),
                        Some(gap * UM),
                        Some(min * UM),
                        message,
                    ));
                }
            }
            // With nothing within the rule's reach, the nearest pair on any layer is the measure.
            if let (Some(nearest), false, true) = (pairs.nearest, seen, layer.is_none()) {
                note(nearest * UM, true);
            }
        }
        RuleKind::Overlap { layer, into, min } if features.prepared => {
            for &(a, b) in &pairs.touching {
                let (shallow, deep) = if height(a) < height(b) - TOLERANCE {
                    (a, b)
                } else {
                    (b, a)
                };
                if height(shallow) >= height(deep) - TOLERANCE || !on(shallow, layer) || !on(deep, into) {
                    continue;
                }
                let Some(depth) = penetration(&bounds[shallow], &bounds[deep]) else {
                    continue;
                };
                note(depth * UM, true);
                if depth < min - 1e-15 {
                    let message = format!(
                        "{} is drawn {:.3} um into {}; layers may sit {:.3} um apart",
                        rectangles[shallow].name,
                        depth * UM,
                        rectangles[deep].name,
                        min * UM
                    );
                    broken.push((
                        min - depth,
                        rectangles[shallow].name.clone(),
                        Some(depth * UM),
                        Some(min * UM),
                        message,
                    ));
                }
            }
        }
        RuleKind::Overlap { .. } => {}
        RuleKind::Aspect { layer, max } => {
            for at in (0..rectangles.len()).filter(|at| on(*at, layer)) {
                let aspect = height(at) / narrow(at);
                note(aspect, false);
                if aspect > max + 1e-9 {
                    let message = format!(
                        "{} is {aspect:.2} times as tall as it is wide; the process holds {max}",
                        rectangles[at].name
                    );
                    broken.push((
                        aspect - max,
                        rectangles[at].name.clone(),
                        Some(aspect),
                        Some(*max),
                        message,
                    ));
                }
            }
            // The wall between two features stands as tall as they do.
            for &(gap, a, b) in pairs.close.iter().filter(|(_, a, _)| on(*a, layer)) {
                let aspect = height(a) / gap;
                note(aspect, false);
                if aspect > max + 1e-9 {
                    let message = format!(
                        "the wall between {} and {} is {aspect:.2} times as tall as it is wide; the process holds {max}",
                        rectangles[a].name, rectangles[b].name
                    );
                    broken.push((
                        aspect - max,
                        rectangles[a].name.clone(),
                        Some(aspect),
                        Some(*max),
                        message,
                    ));
                }
            }
        }
        RuleKind::RoofSpan { layer, max, punch } => {
            for at in (0..rectangles.len()).filter(|at| on(*at, layer)) {
                let punched = features.access.iter().any(|point| within(&bounds[at], *point));
                let span = (narrow(at) - if punched { *punch } else { 0.0 }).max(0.0) / height(at);
                note(span, false);
                if span > max + 1e-9 {
                    let message = format!(
                        "the roof of {} spans {span:.1} times its height; past {max} it sags shut",
                        rectangles[at].name
                    );
                    broken.push((span - max, rectangles[at].name.clone(), Some(span), Some(*max), message));
                }
            }
        }
        RuleKind::PortPitch { min } => {
            for (at, a) in features.access.iter().enumerate() {
                for b in &features.access[at + 1..] {
                    let pitch = (a.x - b.x).hypot(a.y - b.y);
                    note(pitch * UM, true);
                    if pitch < min - 1e-12 {
                        let subject = access(a);
                        let message = format!(
                            "{subject} and {} are {:.0} um apart; tubing needs {:.0} um",
                            access(b),
                            pitch * UM,
                            min * UM
                        );
                        broken.push((min - pitch, subject, Some(pitch * UM), Some(min * UM), message));
                    }
                }
            }
        }
        RuleKind::EdgeMargin { min } => {
            for point in features.access {
                let margin = point
                    .x
                    .min(point.y)
                    .min(features.size[0] - point.x)
                    .min(features.size[1] - point.y);
                note(margin * UM, true);
                if margin < min - 1e-12 {
                    let subject = access(point);
                    let message = format!(
                        "{subject} is {:.0} um from the chip's edge; the process needs {:.0} um",
                        margin * UM,
                        min * UM
                    );
                    broken.push((min - margin, subject, Some(margin * UM), Some(min * UM), message));
                }
            }
        }
        RuleKind::Extent { max } => {
            // The chip may lie either way round on its substrate.
            let (long, short) = (
                features.size[0].max(features.size[1]),
                features.size[0].min(features.size[1]),
            );
            let (most, least) = (max[0].max(max[1]), max[0].min(max[1]));
            note(long * 1e3, false);
            if long > most + 1e-12 || short > least + 1e-12 {
                let message = format!(
                    "the chip is {:.1} x {:.1} mm; the substrate takes {:.1} x {:.1} mm",
                    long * 1e3,
                    short * 1e3,
                    most * 1e3,
                    least * 1e3
                );
                broken.push((
                    long - most,
                    "chip".to_owned(),
                    Some(long * 1e3),
                    Some(most * 1e3),
                    message,
                ));
            }
        }
        RuleKind::LayerHeight { min, max } => {
            for layer in features.layers.iter() {
                note(layer.height * UM, true);
                if layer.height < min - 1e-15 || layer.height > max + 1e-15 {
                    let message = format!(
                        "layer {} is {:.3} um high; the process spins {:.3} to {:.3} um",
                        layer.name,
                        layer.height * UM,
                        min * UM,
                        max * UM
                    );
                    broken.push((1.0, layer.name.clone(), Some(layer.height * UM), None, message));
                }
            }
        }
        RuleKind::LayerCount { max } => {
            note(features.layers.len() as f64, false);
            if features.layers.len() > *max {
                let message = format!(
                    "the chip has {} layers; the process builds {max}",
                    features.layers.len()
                );
                broken.push((
                    1.0,
                    "chip".to_owned(),
                    Some(features.layers.len() as f64),
                    Some(*max as f64),
                    message,
                ));
            }
        }
    }
    (broken, extreme)
}
