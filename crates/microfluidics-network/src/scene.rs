//! The scene: one document that every viewer draws a design from.
//!
//! It holds what is drawn, where each thing was made in the design's source, what flows, and
//! what every check found, in units a viewer uses directly: micrometres, millibar, microlitres
//! a minute. It says nothing of how it is to be drawn. The format has a version of its own.

use std::collections::BTreeMap;

use serde::Serialize;

use microfluidics_core::model::{Bound, Drive, Requirement};
use microfluidics_core::{Check, Finding, Status};
use microfluidics_layout::{Origin, Point, Tunable};

use crate::duct::{WATER_DENSITY, reynolds, wall_shear};
use crate::graph::extract;
use crate::variation::Variation;
use crate::verify::{Group, Verified, group, title};

/// The version of the scene's format, as a semantic version. While the major is 0 the format is
/// not settled: the minor is raised when a reader written for the last one would misread this
/// one, by a field removed or renamed, or one whose meaning or unit is another; the patch is
/// raised when fields are added, which such a reader passes over. A reader takes any scene of
/// the major and minor it was written for whose patch is at least the one that added the last
/// field it reads.
pub const VERSION: &str = "0.6.0";

const UM: f64 = 1e6;

/// The colour of each layer, the tallest first. None is red, which a viewer keeps for what is
/// wrong.
const COLOURS: [&str; 4] = ["#4c9be8", "#e0a030", "#3fb8a0", "#b07cd8"];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneLayer {
    pub name: String,
    pub height_um: f64,
    pub gds: u16,
    pub colour: String,
}

/// A rectangle: its path, its layer, and its corners in micrometres.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shape {
    pub path: String,
    pub layer: String,
    /// `[x0, y0, x1, y1]`.
    pub rect: [f64; 4],
}

/// Translates of one outline: `count` placements of `outline`, each `step` from the last.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneTranslates {
    pub path: String,
    /// The name of the outline placed, in the scene's `outlines`.
    pub outline: String,
    pub origin: [f64; 2],
    pub step: [f64; 2],
    pub count: u32,
    pub quarter_turns: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenePort {
    pub path: String,
    pub at: [f64; 2],
    pub width_um: f64,
    pub layer: String,
    /// The direction a channel leaves by it, as a unit vector.
    pub facing: [f64; 2],
}

/// What flows through one edge of the graph.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneFlow {
    /// The instance or channel the edge belongs to.
    pub element: String,
    pub start: String,
    pub end: String,
    pub flow_ul_min: f64,
    pub velocity_mm_s: f64,
    pub shear_pa: f64,
    pub reynolds: f64,
}

/// A parameter a slider can move, with its value and range as numbers and as they are written.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneTunable {
    pub path: String,
    pub value: f64,
    pub low: f64,
    pub high: f64,
    pub count: bool,
    pub written: String,
}

/// What a requirement allows, in the unit its values are reported in. A side with no limit is
/// absent.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneBound {
    /// `equal_within`, `target`, `at_most` or `at_least`.
    pub kind: String,
    pub low: Option<f64>,
    pub high: Option<f64>,
    /// What the values are meant to be, when the bound names it: for values that must be equal,
    /// their mean.
    pub target: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneRequirement {
    pub name: String,
    pub statement: String,
    pub status: Status,
    /// What was measured at each subject, in `unit`.
    pub values: BTreeMap<String, f64>,
    /// The unit of the values and the bound; empty for a plain number.
    pub unit: String,
    pub bound: SceneBound,
    /// How it fares across the corners of the process, when that was found.
    pub across: Option<SceneAcross>,
    pub note: Option<String>,
}

/// A requirement across the corners of a process, in the requirement's unit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneAcross {
    /// The least and greatest values over every corner, and the corner of each.
    pub low: f64,
    pub low_corner: String,
    pub high: f64,
    pub high_corner: String,
    /// The value furthest from holding and the corner it is at.
    pub reached: f64,
    pub corner: String,
    /// Whether the requirement holds at every corner.
    pub holds: bool,
}

/// One placed instance of a region.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneInstance {
    pub path: String,
    /// The name of the region it is an instance of.
    pub region: String,
    /// The box about everything it made, `[x0, y0, x1, y1]`; absent when it made nothing drawn.
    pub rect: Option<[f64; 4]>,
    /// Its parameters as they would be written, in the order its region states them.
    pub parameters: Vec<(String, String)>,
    pub origin: Option<Origin>,
}

/// What is applied where tubing reaches the chip.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneDrive {
    /// The instance that holds the access point.
    pub instance: String,
    /// The vertex of the graph it drives.
    pub vertex: String,
    pub at: [f64; 2],
    pub pressure_mbar: Option<f64>,
    pub flow_ul_min: Option<f64>,
}

/// One check: what it is called and how it came out. Its findings are among the scene's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneCheck {
    pub id: String,
    pub title: String,
    /// The question it answers.
    pub group: Group,
    pub status: Status,
    pub note: Option<String>,
    pub metrics: BTreeMap<String, f64>,
    /// The process the check was made against, when it depends on one.
    pub process: Option<String>,
}

/// One finding, with the things it is about and where the first of them was made.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneFinding {
    pub check: String,
    /// The process its check was made against, when it depends on one.
    pub process: Option<String>,
    /// The question its check answers.
    pub group: Group,
    pub rule: String,
    pub message: String,
    pub waived: Option<String>,
    /// The paths of the things the finding is about.
    pub paths: Vec<String>,
    pub origin: Option<Origin>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Scene {
    pub version: &'static str,
    pub design: String,
    pub passes: bool,
    /// How many things stand in the way of it passing.
    pub problems: usize,
    pub size_um: [f64; 2],
    /// How thick the slab is.
    pub thickness_um: f64,
    /// The width, depth and thickness of the glass the slab is bonded to, at the glass's middle.
    pub glass_um: [f64; 3],
    /// The diameter of the hole punched at each access point, when the first process the design
    /// names states it.
    pub punch_um: Option<f64>,
    pub layers: Vec<SceneLayer>,
    /// Repeated outlines, stated once: each a list of rectangles about its own origin.
    pub outlines: BTreeMap<String, Vec<Shape>>,
    pub shapes: Vec<Shape>,
    pub translates: Vec<SceneTranslates>,
    pub ports: Vec<ScenePort>,
    /// Where punched holes reach the chip.
    pub access: Vec<[f64; 2]>,
    pub flows: Vec<SceneFlow>,
    /// The pressure at each vertex of the graph, in millibar.
    pub pressures_mbar: BTreeMap<String, f64>,
    /// The viscosity of what flows, in millipascal seconds, when the design says how it is run.
    pub viscosity_mpa_s: Option<f64>,
    /// For each fed species, the share of what arrives at each vertex of the graph that is that
    /// species, from 0 to 1.
    pub fractions: BTreeMap<String, BTreeMap<String, f64>>,
    pub tunables: Vec<SceneTunable>,
    pub requirements: Vec<SceneRequirement>,
    pub instances: Vec<SceneInstance>,
    pub drives: Vec<SceneDrive>,
    pub checks: Vec<SceneCheck>,
    pub findings: Vec<SceneFinding>,
    /// Where each named thing was made, by its path.
    pub origins: BTreeMap<String, Origin>,
}

/// A length in micrometres, to the nanometre masks are written on.
fn microns(metres: f64) -> f64 {
    (metres * UM * 1e3).round() / 1e3
}

fn um(point: Point) -> [f64; 2] {
    [microns(point.x), microns(point.y)]
}

/// The paths a finding is about: its subject, and every known path its message names. A path
/// that is not itself known stands for the nearest thing that holds it.
fn involved(finding: &Finding, known: &BTreeMap<String, Origin>) -> Vec<String> {
    let nearest = |name: &str| {
        let mut path = name;
        loop {
            if known.contains_key(path) {
                return Some(path.to_owned());
            }
            // An element of a row, then the instance that holds it, and so on outward.
            path = match path.rfind(['.', '[']) {
                Some(at) => &path[..at],
                None => return None,
            };
        }
    };
    let named = |text: &str| -> Vec<String> {
        let words = text.split(|letter: char| !(letter.is_alphanumeric() || matches!(letter, '_' | '.' | '[' | ']')));
        words
            .map(|word| word.trim_matches('.'))
            .filter(|word| word.contains(['.', '[']))
            .filter_map(nearest)
            .collect()
    };
    let mut paths: Vec<String> = nearest(&finding.subject)
        .into_iter()
        .chain(named(&finding.subject))
        .collect();
    paths.extend(named(&finding.message));
    let mut seen = Vec::new();
    paths.retain(|path| {
        !seen.contains(path) && {
            seen.push(path.clone());
            true
        }
    });
    paths
}

fn tunable(tunable: &Tunable) -> SceneTunable {
    // A length is shown in micrometres; anything else as it is.
    let length = tunable.value.written().ends_with("um");
    let shown = |si: f64| if length { microns(si) } else { si };
    SceneTunable {
        path: tunable.path.clone(),
        value: shown(tunable.value.si),
        low: shown(tunable.low.si),
        high: shown(tunable.high.si),
        count: tunable.count,
        written: tunable.value.written(),
    }
}

fn listed(check: &Check) -> SceneCheck {
    SceneCheck {
        id: check.id.clone(),
        title: title(&check.id),
        group: group(&check.id),
        status: check.status,
        note: check.note.clone(),
        metrics: check.metrics.clone(),
        process: check.process.clone(),
    }
}

/// What `bound` allows of `values`, with everything in the unit of which `per_si` make one SI
/// unit.
fn allowed(bound: &Bound, values: &BTreeMap<String, f64>, per_si: f64) -> SceneBound {
    let made = |kind: &str, low: Option<f64>, high: Option<f64>, target: Option<f64>| SceneBound {
        kind: kind.to_owned(),
        low,
        high,
        target,
    };
    match *bound {
        Bound::EqualWithin(share) => {
            let mean = values.values().sum::<f64>() / values.len().max(1) as f64;
            let slack = share * mean.abs();
            made("equal_within", Some(mean - slack), Some(mean + slack), Some(mean))
        }
        Bound::Is {
            value,
            within,
            relative,
        } => {
            let slack = if relative { within * value.abs() } else { within } * per_si;
            let value = value * per_si;
            made("target", Some(value - slack), Some(value + slack), Some(value))
        }
        Bound::AtMost(value) => made("at_most", None, Some(value * per_si), None),
        Bound::AtLeast(value) => made("at_least", Some(value * per_si), None, None),
    }
}

/// The box about every rectangle of `boxes` whose path is `path` or lies within it.
fn about(path: &str, boxes: &[(String, [f64; 4])]) -> Option<[f64; 4]> {
    let within = |name: &str| {
        name.strip_prefix(path)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '[']))
    };
    let mut held = boxes.iter().filter(|(name, _)| within(name)).map(|(_, rect)| *rect);
    let first = held.next()?;
    Some(held.fold(first, |all, rect| {
        [
            all[0].min(rect[0]),
            all[1].min(rect[1]),
            all[2].max(rect[2]),
            all[3].max(rect[3]),
        ]
    }))
}

/// The scene of a verified design, made in `process`.
pub fn scene(verified: &Verified) -> Scene {
    // What varies with the process is shown as the first the design names has it.
    let first = verified.made.first();
    let process = first.and_then(|made| made.process.as_ref());
    let elaborated = &verified.analysis.elaborated;
    let (chip, layout) = (&elaborated.chip, &elaborated.chip.layout);
    let shape = |rectangle: &microfluidics_layout::Rectangle| {
        let bounds = rectangle.bounds();
        Shape {
            path: rectangle.name.clone(),
            layer: rectangle.layer.clone(),
            rect: [
                microns(bounds.min.x),
                microns(bounds.min.y),
                microns(bounds.max.x),
                microns(bounds.max.y),
            ],
        }
    };
    // A layer's place by height, the tallest first; of two as tall, the one stated first.
    let taller = |at: usize, height: f64| {
        (chip.layers.iter().enumerate())
            .filter(|(other, layer)| layer.height > height || (layer.height == height && *other < at))
            .count()
    };
    let layers = (chip.layers.iter().enumerate())
        .map(|(at, layer)| SceneLayer {
            name: layer.name.clone(),
            height_um: microns(layer.height),
            gds: layer.gds,
            colour: COLOURS[taller(at, layer.height) % COLOURS.len()].to_owned(),
        })
        .collect();
    let translates = (layout.translates.iter())
        .map(|row| SceneTranslates {
            path: row.name.clone(),
            outline: row.structure.clone(),
            origin: um(row.origin),
            step: um(row.step),
            count: row.count,
            quarter_turns: row.quarter_turns,
        })
        .collect();
    let ports = (layout.ports.iter())
        .map(|port| {
            let facing = port.direction.unit();
            ScenePort {
                path: port.name.clone(),
                at: um(port.position),
                width_um: microns(port.width),
                layer: port.layer.clone(),
                facing: [facing.x, facing.y],
            }
        })
        .collect();

    // What flows: the graph solved again as drawn, when the design says how it is run.
    let (mut flows, mut pressures_mbar, mut fractions) = (Vec::new(), BTreeMap::new(), BTreeMap::new());
    if let Some(operating) = &elaborated.operating
        && let Ok(network) = extract(chip, &verified.analysis.nets, &Variation::nominal())
        && let Ok(solution) = network.solve(operating)
    {
        for (edge, flow) in network.edges.iter().zip(&solution.flow) {
            let (width, height) = edge.narrowest;
            flows.push(SceneFlow {
                element: edge.element.clone(),
                start: network.vertices[edge.from].clone(),
                end: network.vertices[edge.to].clone(),
                flow_ul_min: flow * 60.0e9,
                velocity_mm_s: flow.abs() / (width * height) * 1e3,
                shear_pa: wall_shear(operating.viscosity, *flow, width, height),
                reynolds: reynolds(WATER_DENSITY, operating.viscosity, *flow, width, height),
            });
        }
        pressures_mbar = network
            .vertices
            .iter()
            .cloned()
            .zip(solution.pressure.iter().map(|pascals| pascals / 100.0))
            .collect();
        fractions = (solution.fractions.iter())
            .map(|(species, shares)| {
                let at = network.vertices.iter().cloned().zip(shares.iter().copied());
                (species.clone(), at.collect())
            })
            .collect();
    }

    // The box of every drawn thing by its path: a row of translates as one.
    let mut boxes: Vec<(String, [f64; 4])> = Vec::new();
    for rectangle in &layout.rectangles {
        let held = shape(rectangle);
        boxes.push((held.path, held.rect));
    }
    for row in &layout.translates {
        if let Some(structure) = layout.structures.get(&row.structure) {
            boxes.extend(row.rectangles(structure).map(|rectangle| {
                let held = shape(&rectangle);
                (row.name.clone(), held.rect)
            }));
        }
    }
    let instances: Vec<SceneInstance> = (elaborated.instances.iter())
        .map(|instance| SceneInstance {
            path: instance.path.clone(),
            region: instance.region.clone(),
            rect: about(&instance.path, &boxes),
            parameters: (instance.parameters.iter())
                .map(|(name, value)| (name.clone(), value.written()))
                .collect(),
            origin: elaborated.origins.get(&instance.path).cloned(),
        })
        .collect();
    // What is applied at each access point, drawn at the point its instance holds.
    let access: Vec<[f64; 2]> = layout.access.iter().map(|point| um(*point)).collect();
    let drives = (elaborated.operating.iter())
        .flat_map(|operating| &operating.drives)
        .map(|(vertex, drive)| {
            let instance = vertex.rsplit_once('.').map_or(vertex.as_str(), |(holder, _)| holder);
            let rect = instances
                .iter()
                .find(|held| held.path == instance)
                .and_then(|held| held.rect);
            let inside = |point: &&[f64; 2]| {
                rect.is_some_and(|rect| {
                    point[0] >= rect[0] && point[0] <= rect[2] && point[1] >= rect[1] && point[1] <= rect[3]
                })
            };
            let middle = rect.map(|rect| [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0]);
            SceneDrive {
                instance: instance.to_owned(),
                vertex: vertex.clone(),
                at: access.iter().find(inside).copied().or(middle).unwrap_or_default(),
                pressure_mbar: match drive {
                    Drive::Pressure(pascals) => Some(pascals / 100.0),
                    Drive::FlowRate(_) => None,
                },
                flow_ul_min: match drive {
                    Drive::FlowRate(flow) => Some(flow * 60.0e9),
                    Drive::Pressure(_) => None,
                },
            }
        })
        .collect();

    let checks = &verified.report.checks;
    let stated = |requirement: &Requirement| {
        let check = checks
            .iter()
            .find(|check| check.id == format!("requirement/{}", requirement.name));
        let values = check.map(|check| check.metrics.clone()).unwrap_or_default();
        let (unit, per_si) = requirement.measure.reported();
        SceneRequirement {
            name: requirement.name.clone(),
            statement: requirement.statement.clone(),
            status: check.map_or(Status::NotRun, |check| check.status),
            bound: allowed(&requirement.bound, &values, per_si),
            values,
            unit: unit.to_owned(),
            across: first
                .and_then(|made| made.across.get(&requirement.name))
                .map(|across| SceneAcross {
                    low: across.low * per_si,
                    low_corner: across.low_corner.clone(),
                    high: across.high * per_si,
                    high_corner: across.high_corner.clone(),
                    reached: across.reached * per_si,
                    corner: across.corner.clone(),
                    holds: across.distance <= 1.0 + 1e-9,
                }),
            note: check.and_then(|check| check.note.clone()),
        }
    };
    let mut findings = Vec::new();
    for check in checks {
        for finding in &check.findings {
            let paths = involved(finding, &elaborated.origins);
            let origin = paths.first().and_then(|path| elaborated.origins.get(path)).cloned();
            findings.push(SceneFinding {
                check: check.id.clone(),
                process: check.process.clone(),
                group: group(&check.id),
                rule: finding.rule.clone(),
                message: finding.message.clone(),
                waived: finding.waived.clone(),
                paths,
                origin,
            });
        }
    }
    Scene {
        version: VERSION,
        design: elaborated.name.clone(),
        passes: verified.report.passes,
        problems: verified.report.problems,
        size_um: [microns(chip.size[0]), microns(chip.size[1])],
        thickness_um: microns(chip.size[2]),
        glass_um: chip.glass.map(microns),
        punch_um: process.and_then(|process| process.punch()).map(microns),
        layers,
        outlines: layout
            .structures
            .iter()
            .map(|(name, structure)| (name.clone(), structure.rectangles.iter().map(shape).collect()))
            .collect(),
        shapes: layout.rectangles.iter().map(shape).collect(),
        translates,
        ports,
        access,
        flows,
        pressures_mbar,
        viscosity_mpa_s: elaborated.operating.as_ref().map(|operating| operating.viscosity * 1e3),
        fractions,
        tunables: elaborated.tunables.iter().map(tunable).collect(),
        requirements: elaborated.requirements.iter().map(stated).collect(),
        instances,
        drives,
        checks: checks.iter().map(listed).collect(),
        findings,
        origins: elaborated.origins.clone(),
    }
}
