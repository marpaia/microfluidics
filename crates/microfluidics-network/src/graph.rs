//! The flow graph of a chip: vertices joined by edges, each with a resistance.
//!
//! Extraction reads only what the chip states: its edges and the nets its connectivity gave. It
//! never looks at an outline. A routed channel is a edge like any other, so its resistance is
//! that of the length it was actually routed to.

use std::collections::BTreeMap;

use microfluidics_layout::{Chip, Net};

use crate::duct::resistance_per_viscosity;
use crate::error::NetworkError;
use crate::variation::Variation;

/// One edge of the graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// The instance or channel the edge belongs to.
    pub element: String,
    pub from: usize,
    pub to: usize,
    /// The pressure drop per unit flow rate and viscosity, in 1/m^3.
    pub resistance_per_viscosity: f64,
    /// The fluid the edge holds, in cubic metres.
    pub volume: f64,
    /// The width and height of its tightest duct: where speed and shear are highest.
    pub narrowest: (f64, f64),
    /// How many straight ducts it is stated as, end to end.
    pub ducts: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Graph {
    pub vertices: Vec<String>,
    pub edges: Vec<Edge>,
    /// The vertex each name stands for: a port stands for its net.
    names: BTreeMap<String, usize>,
}

impl Graph {
    /// A graph of named vertices and the edges between them.
    pub fn from_parts(vertices: Vec<String>, edges: Vec<Edge>) -> Self {
        let names = vertices
            .iter()
            .enumerate()
            .map(|(at, name)| (name.clone(), at))
            .collect();
        Self { vertices, edges, names }
    }

    /// The vertex `name` is, or is part of.
    pub fn vertex(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    /// The one edge `name` means: the edge of an instance or channel that has one, or the edge
    /// at a port.
    pub fn edge(&self, name: &str) -> Result<usize, NetworkError> {
        let own: Vec<usize> = (0..self.edges.len())
            .filter(|at| self.edges[*at].element == name)
            .collect();
        let at_port = || {
            let (owner, vertex) = (name.rsplit_once('.')?.0, self.vertex(name)?);
            let touching = |at: &usize| {
                self.edges[*at].element == owner && (self.edges[*at].from == vertex || self.edges[*at].to == vertex)
            };
            Some((0..self.edges.len()).filter(touching).collect::<Vec<usize>>())
        };
        match own.as_slice() {
            [one] => Ok(*one),
            [] => match at_port().as_deref() {
                Some([one]) => Ok(*one),
                Some([_, _, ..]) => Err(NetworkError::Ambiguous(name.to_owned())),
                _ => Err(NetworkError::Unknown(name.to_owned())),
            },
            _ => Err(NetworkError::Ambiguous(name.to_owned())),
        }
    }
}

/// The graph of `chip`, whose ports are joined as `nets` say, made as `variation` says.
pub fn extract(chip: &Chip, nets: &[Net], variation: &Variation) -> Result<Graph, NetworkError> {
    if chip.edges.is_empty() {
        return Err(NetworkError::Empty);
    }
    let mut graph = Graph::default();
    // Ports placed together are one vertex, named for their net.
    for net in nets {
        let vertex = graph.vertices.len();
        graph.vertices.push(net.name.clone());
        graph.names.extend(net.ports.iter().map(|port| (port.clone(), vertex)));
    }
    let mut counted = 0;
    for edge in &chip.edges {
        let end = |name: &str, graph: &mut Graph| match graph.names.get(name) {
            Some(vertex) => *vertex,
            None => {
                let vertex = graph.vertices.len();
                graph.vertices.push(name.to_owned());
                graph.names.insert(name.to_owned(), vertex);
                vertex
            }
        };
        let (from, to) = (end(&edge.from, &mut graph), end(&edge.to, &mut graph));
        let (mut resistance, mut volume, mut narrowest) = (0.0, 0.0, (f64::INFINITY, f64::INFINITY));
        for duct in &edge.ducts {
            let (height, width) = variation.of(&duct.layer, chip.layers.height(&duct.layer), duct.width, counted);
            counted += 1;
            resistance += resistance_per_viscosity(width, height, duct.length);
            volume += width * height * duct.length;
            // Shear goes as one over the long side times the short side squared.
            let tightness = |(width, height): (f64, f64)| width.max(height) * width.min(height).powi(2);
            if tightness((width, height)) < tightness(narrowest) {
                narrowest = (width, height);
            }
        }
        if !(resistance.is_finite() && resistance > 0.0) {
            return Err(NetworkError::NoLength(edge.element.clone()));
        }
        graph.edges.push(Edge {
            element: edge.element.clone(),
            from,
            to,
            resistance_per_viscosity: resistance,
            volume,
            narrowest,
            ducts: edge.ducts.len(),
        });
    }
    Ok(graph)
}
