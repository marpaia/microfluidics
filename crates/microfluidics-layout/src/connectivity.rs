//! Connectivity: whether the connections a design states are the connections its geometry makes.
//!
//! A design states connections three ways: by placing one port on another, by routing a channel
//! between two ports, and by each structure's edges between its own ports. The geometry makes
//! connections where features share an edge. The two must agree.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use microfluidics_core::{Check, Finding, Waiver};

use crate::chip::Chip;
use crate::geometry::{Point, Rect};
use crate::rectangle::Rectangle;
use crate::rules::pairs::{pairs, root, within};

/// Ports placed together: one vertex of the flow graph.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Net {
    /// The first of its ports by name.
    pub name: String,
    pub ports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Connectivity {
    /// Every port, in the net it belongs to.
    pub nets: Vec<Net>,
    pub check: Check,
}

/// Sets of names, joined pair by pair.
#[derive(Default)]
struct Groups {
    index: BTreeMap<String, usize>,
    parents: Vec<usize>,
}

impl Groups {
    fn of(&mut self, name: &str) -> usize {
        let next = self.parents.len();
        let at = *self.index.entry(name.to_owned()).or_insert(next);
        if at == next {
            self.parents.push(next);
        }
        root(&mut self.parents, at)
    }

    fn join(&mut self, a: &str, b: &str) {
        let (a, b) = (self.of(a), self.of(b));
        self.parents[a] = b;
    }
}

fn finding(rule: &str, subject: &str, message: String) -> Finding {
    Finding {
        rule: rule.to_owned(),
        subject: subject.to_owned(),
        measured: None,
        limit: None,
        message,
        waived: None,
    }
}

/// Compare the connections `chip` states with those its features make.
pub fn connectivity(chip: &Chip, waivers: &[Waiver]) -> Connectivity {
    let rectangles = chip.layout.flattened();
    let bounds: Vec<Rect> = rectangles.iter().map(Rectangle::bounds).collect();
    // The features that touch, as sets.
    let mut parents: Vec<usize> = (0..rectangles.len()).collect();
    let touching = pairs(&rectangles, &bounds, 0.0).touching;
    for &(a, b) in &touching {
        let (a, b) = (root(&mut parents, a), root(&mut parents, b));
        parents[a] = b;
    }
    let body_at = |point: Point, layer: Option<&str>, parents: &mut Vec<usize>| {
        let holds =
            |at: &usize| within(&bounds[*at], point) && layer.is_none_or(|layer| rectangles[*at].layer == layer);
        (0..rectangles.len()).find(holds).map(|at| root(parents, at))
    };

    let ports = &chip.layout.ports;
    let mut findings = Vec::new();
    // The body of fluid each port opens into: just inside the port, on its layer.
    let mut body: BTreeMap<&str, usize> = BTreeMap::new();
    for port in ports {
        let inward = port.direction.opposite().unit();
        let inside = Point::new(port.position.x + inward.x * 1e-9, port.position.y + inward.y * 1e-9);
        match body_at(inside, Some(&port.layer), &mut parents) {
            Some(at) => {
                body.insert(&port.name, at);
            }
            None => findings.push(finding(
                "open",
                &port.name,
                format!("the port {} opens into nothing on layer {}", port.name, port.layer),
            )),
        }
    }

    // Nets: ports placed together.
    let mut placed = Groups::default();
    for port in ports {
        placed.of(&port.name);
    }
    for (a, b) in &chip.joints {
        placed.join(a, b);
    }
    let mut nets: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for port in ports {
        nets.entry(placed.of(&port.name)).or_default().push(port.name.clone());
    }
    let mut nets: Vec<Net> = (nets.into_values())
        .map(|mut ports| {
            ports.sort();
            Net {
                name: ports[0].clone(),
                ports,
            }
        })
        .collect();
    nets.sort_by(|a, b| a.name.cmp(&b.name));

    // What the design states is connected: joints, and the two ends of every edge.
    let mut stated = Groups::default();
    for (a, b) in &chip.joints {
        stated.join(a, b);
        if let (Some(first), Some(second)) = (body.get(a.as_str()), body.get(b.as_str()))
            && first != second
        {
            findings.push(finding(
                "open",
                a,
                format!("{a} is placed on {b}, but their features do not meet"),
            ));
        }
    }
    for edge in &chip.edges {
        stated.join(&edge.from, &edge.to);
        if let (Some(first), Some(second)) = (body.get(edge.from.as_str()), body.get(edge.to.as_str()))
            && first != second
        {
            let message = format!(
                "{} states a edge from {} to {}, but no features join them",
                edge.element, edge.from, edge.to
            );
            findings.push(finding("open", &edge.element, message));
        }
    }
    // What touches without the design saying so: one body of fluid holding ports of two stated groups.
    let mut held: BTreeMap<usize, BTreeMap<usize, &str>> = BTreeMap::new();
    for port in ports {
        if let Some(at) = body.get(port.name.as_str()) {
            held.entry(*at)
                .or_default()
                .entry(stated.of(&port.name))
                .or_insert(&port.name);
        }
    }
    for groups in held.values().filter(|groups| groups.len() > 1) {
        let names: Vec<&str> = groups.values().copied().collect();
        let message = format!(
            "{} and {} are not connected in the design, but their features touch",
            names[0], names[1]
        );
        findings.push(finding("short", names[0], message));
    }
    // Features of two parts may touch only where the design connects them: at a port placed on
    // another, or where a channel meets a port. Anywhere else, fluid has a way the design does
    // not know of, and whatever it bypasses is not the resistance its edges state.
    let part = |at: usize| rectangles[at].name.split('.').next().unwrap_or_default();
    let mut met: Vec<Point> = chip
        .joints
        .iter()
        .flat_map(|(a, b)| [a, b])
        .filter_map(|name| ports.iter().find(|port| &port.name == name))
        .map(|port| port.position)
        .collect();
    for edge in chip
        .edges
        .iter()
        .filter(|edge| !edge.from.starts_with(&format!("{}.", edge.element)))
    {
        met.extend(
            [&edge.from, &edge.to]
                .into_iter()
                .filter_map(|name| ports.iter().find(|port| &port.name == name))
                .map(|port| port.position),
        );
    }
    let mut bypassed: BTreeSet<(&str, &str)> = BTreeSet::new();
    for &(a, b) in touching.iter().filter(|(a, b)| part(*a) != part(*b)) {
        let stated = met
            .iter()
            .any(|point| within(&bounds[a], *point) && within(&bounds[b], *point));
        if !stated && bypassed.insert((part(a).min(part(b)), part(a).max(part(b)))) {
            let message = format!(
                "{} touches {} where the design connects nothing, so fluid passes there by a way the design does not state; join their ports with `attach` or a `channel`, or move them apart",
                rectangles[a].name, rectangles[b].name
            );
            findings.push(finding("short", &rectangles[a].name, message));
        }
    }
    // A port nothing outside its own structure connects to.
    let mut connected: BTreeSet<&str> = chip.joints.iter().flat_map(|(a, b)| [a.as_str(), b.as_str()]).collect();
    for edge in &chip.edges {
        let outside = |port: &str| !port.starts_with(&format!("{}.", edge.element));
        connected.extend(
            [edge.from.as_str(), edge.to.as_str()]
                .into_iter()
                .filter(|port| outside(port)),
        );
    }
    for port in ports.iter().filter(|port| !connected.contains(port.name.as_str())) {
        findings.push(finding(
            "dangling",
            &port.name,
            format!("nothing is connected to the port {}", port.name),
        ));
    }
    // Every body of fluid that holds a port needs a way in and a way out.
    let mut holes: BTreeMap<usize, usize> = BTreeMap::new();
    for point in &chip.layout.access {
        if let Some(at) = body_at(*point, None, &mut parents) {
            *holes.entry(at).or_default() += 1;
        }
    }
    let components: BTreeMap<usize, &str> = held
        .iter()
        .filter_map(|(at, groups)| Some((*at, *groups.values().next()?)))
        .collect();
    for (at, port) in &components {
        let holes = holes.get(at).copied().unwrap_or(0);
        if holes < 2 {
            let message = format!(
                "the channels joined to {port} reach {holes} access points; fluid needs a way in and a way out"
            );
            findings.push(finding("closed", port, message));
        }
    }

    let through = components
        .keys()
        .filter(|at| holes.get(*at).copied().unwrap_or(0) >= 2)
        .count();
    let metrics = [
        ("ports", ports.len()),
        ("nets", nets.len()),
        ("edges", chip.edges.len()),
        ("components", components.len()),
        ("open_to_flow", through),
    ];
    let metrics = metrics
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value as f64))
        .collect();
    Connectivity {
        nets,
        check: Check::from_findings("connectivity", findings, metrics, waivers),
    }
}
