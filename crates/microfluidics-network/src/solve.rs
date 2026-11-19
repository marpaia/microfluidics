//! Solving a graph: the pressure at every vertex and the flow in every edge.
//!
//! Each edge carries a flow in proportion to the pressure across it, so the pressures solve a
//! set of linear equations: at every vertex not held at a pressure, what flows in flows out.

use std::collections::BTreeMap;

use microfluidics_core::model::{Drive, Measure, Operating};
use microfluidics_core::{Check, Finding};

use crate::duct::{WATER_DENSITY, reynolds, wall_shear};
use crate::error::NetworkError;
use crate::graph::Graph;
use crate::linalg::solve_dense;

/// A solved graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Solution<'a> {
    pub graph: &'a Graph,
    pub operating: &'a Operating,
    /// The pressure at each vertex, in pascals.
    pub pressure: Vec<f64>,
    /// The flow in each edge from its first vertex to its second, in cubic metres a second.
    pub flow: Vec<f64>,
    /// The fraction of each species at each vertex.
    pub fractions: BTreeMap<String, Vec<f64>>,
}

impl Graph {
    /// The pressures and flows when the chip is run as `operating` says.
    pub fn solve<'a>(&'a self, operating: &'a Operating) -> Result<Solution<'a>, NetworkError> {
        let count = self.vertices.len();
        let mut held: BTreeMap<usize, f64> = BTreeMap::new();
        let mut injected = vec![0.0; count];
        for (name, drive) in &operating.drives {
            let vertex = self
                .vertex(name)
                .ok_or_else(|| NetworkError::UnknownDrive(name.clone()))?;
            match drive {
                Drive::Pressure(pressure) => {
                    held.insert(vertex, *pressure);
                }
                Drive::FlowRate(flow) => injected[vertex] += flow,
            }
        }
        // Every vertex must be joined, through edges, to somewhere held at a pressure.
        let mut reached: Vec<bool> = (0..count).map(|vertex| held.contains_key(&vertex)).collect();
        loop {
            let before = reached.iter().filter(|reached| **reached).count();
            for edge in &self.edges {
                if reached[edge.from] || reached[edge.to] {
                    (reached[edge.from], reached[edge.to]) = (true, true);
                }
            }
            if reached.iter().filter(|reached| **reached).count() == before {
                break;
            }
        }
        if let Some(vertex) = reached.iter().position(|reached| !reached) {
            return Err(NetworkError::Floating(self.vertices[vertex].clone()));
        }

        let free: Vec<usize> = (0..count).filter(|vertex| !held.contains_key(vertex)).collect();
        let row_of: BTreeMap<usize, usize> = free.iter().enumerate().map(|(row, vertex)| (*vertex, row)).collect();
        let mut matrix = vec![vec![0.0; free.len()]; free.len()];
        let mut right: Vec<f64> = free.iter().map(|vertex| injected[*vertex]).collect();
        let conductance = |at: usize| 1.0 / (operating.viscosity * self.edges[at].resistance_per_viscosity);
        for (at, edge) in self.edges.iter().enumerate() {
            let conductance = conductance(at);
            for (own, other) in [(edge.from, edge.to), (edge.to, edge.from)] {
                let Some(&row) = row_of.get(&own) else { continue };
                matrix[row][row] += conductance;
                match (row_of.get(&other), held.get(&other)) {
                    (Some(&column), _) => matrix[row][column] -= conductance,
                    (None, Some(pressure)) => right[row] += conductance * pressure,
                    (None, None) => {}
                }
            }
        }
        let solved = solve_dense(matrix, right).ok_or_else(|| {
            NetworkError::Floating(
                free.first()
                    .map_or_else(String::new, |vertex| self.vertices[*vertex].clone()),
            )
        })?;
        let mut pressure = vec![0.0; count];
        for (vertex, value) in &held {
            pressure[*vertex] = *value;
        }
        for (row, vertex) in free.iter().enumerate() {
            pressure[*vertex] = solved[row];
        }
        let flow: Vec<f64> = (0..self.edges.len())
            .map(|at| conductance(at) * (pressure[self.edges[at].from] - pressure[self.edges[at].to]))
            .collect();
        let mut solution = Solution {
            graph: self,
            operating,
            pressure,
            flow,
            fractions: BTreeMap::new(),
        };
        solution.mix();
        Ok(solution)
    }
}

impl Solution<'_> {
    /// What leaves `vertex` by its edges, less what arrives: what must enter it from outside.
    fn supplied(&self, vertex: usize) -> f64 {
        let edges = self.graph.edges.iter().zip(&self.flow);
        edges
            .map(|(edge, flow)| {
                if edge.from == vertex {
                    *flow
                } else if edge.to == vertex {
                    -flow
                } else {
                    0.0
                }
            })
            .sum()
    }

    /// The steady fraction of each fed species at each vertex: fluid arriving at a vertex is mixed
    /// in proportion to its flow, worked from the highest pressure down.
    fn mix(&mut self) {
        let species: Vec<String> = self
            .operating
            .feeds
            .values()
            .flat_map(|feed| feed.keys().cloned())
            .collect();
        let mut order: Vec<usize> = (0..self.pressure.len()).collect();
        order.sort_by(|a, b| self.pressure[*b].total_cmp(&self.pressure[*a]));
        for name in species {
            let fed = |vertex: usize| {
                self.operating
                    .feeds
                    .get(&self.graph.vertices[vertex])
                    .and_then(|feed| feed.get(&name))
                    .copied()
                    .unwrap_or(0.0)
            };
            let mut fraction = vec![0.0; self.pressure.len()];
            for &vertex in &order {
                let outside = self.supplied(vertex).max(0.0);
                let (mut arriving, mut carried) = (outside, outside * fed(vertex));
                for (edge, flow) in self.graph.edges.iter().zip(&self.flow) {
                    let (other, inward) = if edge.to == vertex {
                        (edge.from, *flow)
                    } else if edge.from == vertex {
                        (edge.to, -flow)
                    } else {
                        continue;
                    };
                    if inward > 0.0 {
                        arriving += inward;
                        carried += inward * fraction[other];
                    }
                }
                fraction[vertex] = if arriving > 0.0 { carried / arriving } else { 0.0 };
            }
            self.fractions.insert(name, fraction);
        }
    }

    fn vertex(&self, name: &str) -> Result<usize, NetworkError> {
        let access = format!("{name}.access");
        self.graph
            .vertex(name)
            .or_else(|| self.graph.vertex(&access))
            .ok_or_else(|| NetworkError::Unknown(name.to_owned()))
    }

    /// One measure at one place, in SI units. A pressure drop is measured with [`Solution::drop`].
    pub fn quantity(&self, measure: &Measure, at: &str) -> Result<f64, NetworkError> {
        let edge = |at: &str| {
            let edge = self.graph.edge(at)?;
            Ok::<_, NetworkError>((&self.graph.edges[edge], self.flow[edge].abs()))
        };
        Ok(match measure {
            Measure::Flow => edge(at)?.1,
            Measure::Velocity => {
                let (edge, flow) = edge(at)?;
                flow / (edge.narrowest.0 * edge.narrowest.1)
            }
            Measure::Shear => {
                let (edge, flow) = edge(at)?;
                wall_shear(self.operating.viscosity, flow, edge.narrowest.0, edge.narrowest.1)
            }
            Measure::Reynolds => {
                let (edge, flow) = edge(at)?;
                reynolds(
                    WATER_DENSITY,
                    self.operating.viscosity,
                    flow,
                    edge.narrowest.0,
                    edge.narrowest.1,
                )
            }
            Measure::Residence => {
                let (edge, flow) = edge(at)?;
                edge.volume / flow
            }
            Measure::Pressure | Measure::Drop => self.pressure[self.vertex(at)?],
            Measure::Yield => {
                return Err(NetworkError::Unknown(
                    "a place for yield, which is of the whole chip".to_owned(),
                ));
            }
            Measure::Fraction { species } => {
                let fractions = self
                    .fractions
                    .get(species)
                    .ok_or_else(|| NetworkError::Unknown(format!("species {species}")))?;
                fractions[self.vertex(at)?]
            }
        })
    }

    /// The pressure from one vertex to another, in pascals.
    pub fn drop(&self, from: &str, to: &str) -> Result<f64, NetworkError> {
        Ok(self.pressure[self.vertex(from)?] - self.pressure[self.vertex(to)?])
    }

    /// Whether the solution can be trusted: flow is conserved at every vertex, and slow enough
    /// everywhere for the linear relation between pressure and flow to hold.
    pub fn check(&self) -> Check {
        let largest = self
            .flow
            .iter()
            .fold(0.0_f64, |most, flow| most.max(flow.abs()))
            .max(f64::MIN_POSITIVE);
        let driven = |vertex: usize| self.operating.drives.contains_key(&self.graph.vertices[vertex]);
        let residual = (0..self.pressure.len())
            .filter(|vertex| !driven(*vertex))
            .map(|vertex| self.supplied(vertex).abs() / largest)
            .fold(0.0, f64::max);
        let fastest = (self.graph.edges.iter().zip(&self.flow))
            .map(|(edge, flow)| {
                (
                    reynolds(
                        WATER_DENSITY,
                        self.operating.viscosity,
                        *flow,
                        edge.narrowest.0,
                        edge.narrowest.1,
                    ),
                    edge.element.as_str(),
                )
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));
        // A edge nothing flows through: it leads nowhere, or its ends are at one pressure.
        let most = self.flow.iter().fold(0.0_f64, |most, flow| most.max(flow.abs()));
        let still = self.flow.iter().filter(|flow| flow.abs() <= 1e-9 * most).count();
        let mut findings = Vec::new();
        let finding = |rule: &str, subject: &str, measured: f64, limit: f64, message: String| Finding {
            rule: rule.to_owned(),
            subject: subject.to_owned(),
            measured: Some(measured),
            limit: Some(limit),
            message,
            waived: None,
        };
        if residual > 1e-9 {
            findings.push(finding(
                "conservation",
                "graph",
                residual,
                1e-9,
                format!("flow is not conserved: a vertex gains or loses {residual:e} of the largest flow"),
            ));
        }
        if let Some((number, element)) = fastest.filter(|(number, _)| *number > 1.0) {
            let message = format!(
                "the Reynolds number in {element} is {number:.2}; above 1 the flow is no longer in simple proportion to pressure"
            );
            findings.push(finding("laminar", element, number, 1.0, message));
        }
        let metrics = [
            ("vertices", self.pressure.len() as f64),
            ("edges", self.flow.len() as f64),
            ("conservation_residual", residual),
            ("max_reynolds", fastest.map_or(0.0, |(number, _)| number)),
            ("edges_without_flow", still as f64),
        ];
        Check::from_findings(
            "graph",
            findings,
            metrics
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
            &[],
        )
    }
}
