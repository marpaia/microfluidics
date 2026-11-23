//! Solving a graph: the pressure at every vertex and the flow in every edge.
//!
//! Each edge carries a flow in proportion to the pressure across it, so the pressures solve a
//! set of linear equations: at every vertex not held at a pressure, what flows in flows out.

use std::collections::BTreeMap;

use microfluidics_core::model::{Drive, Measure, Operating};
use microfluidics_core::{Check, Finding, Waiver};

use crate::duct::{LAMINAR_LIMIT, WATER_DENSITY, dynamic_pressure, reynolds, wall_shear};
use crate::error::NetworkError;
use crate::graph::Graph;
use crate::linalg::solve_dense;

/// The share of the pressure that drives a chip which one edge may lose to inertia before the
/// solution is not to be trusted.
const INERTIAL_SHARE: f64 = 0.01;

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

    /// Whether the solution can be trusted: flow is conserved at every vertex, laminar in every
    /// edge, and loses little enough to inertia for pressure lost to viscosity alone to stand
    /// for the whole.
    pub fn check(&self, waivers: &[Waiver]) -> Check {
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
        let in_tightest = |edge: &crate::graph::Edge, flow: f64| {
            let (width, height) = edge.narrowest;
            (
                reynolds(WATER_DENSITY, self.operating.viscosity, flow, width, height),
                dynamic_pressure(WATER_DENSITY, flow, width, height),
            )
        };
        let fastest = (self.graph.edges.iter().zip(&self.flow))
            .map(|(edge, flow)| (in_tightest(edge, *flow).0, edge.element.as_str()))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        // The largest difference of pressure on the chip: what drives it.
        let (low, high) = (self.pressure.iter()).fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), held| {
            (low.min(*held), high.max(*held))
        });
        let span = (high - low).max(f64::MIN_POSITIVE);
        // Each duct of an edge has an entrance, and where it meets the next a bend or a change
        // of section: each loses about one dynamic pressure, which the model leaves out.
        let most_inertial = (self.graph.edges.iter().zip(&self.flow))
            .map(|(edge, flow)| {
                let lost = edge.ducts.max(1) as f64 * in_tightest(edge, *flow).1;
                (lost / span, edge.element.as_str())
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
        if let Some((number, element)) = fastest.filter(|(number, _)| *number > LAMINAR_LIMIT) {
            let message = format!(
                "the Reynolds number in {element} is {number:.0}; above {LAMINAR_LIMIT:.0} flow in a duct may no longer be laminar"
            );
            findings.push(finding("laminar", element, number, LAMINAR_LIMIT, message));
        }
        if let Some((share, element)) = most_inertial.filter(|(share, _)| *share > INERTIAL_SHARE) {
            let message = format!(
                "inertia in {element} loses about {:.1}% of the pressure that drives the chip, which the model leaves out; it allows {:.0}%",
                share * 100.0,
                INERTIAL_SHARE * 100.0
            );
            findings.push(finding(
                "inertia",
                element,
                share * 100.0,
                INERTIAL_SHARE * 100.0,
                message,
            ));
        }
        let metrics = [
            ("vertices", self.pressure.len() as f64),
            ("edges", self.flow.len() as f64),
            ("conservation_residual", residual),
            ("max_reynolds", fastest.map_or(0.0, |(number, _)| number)),
            (
                "inertial_loss_percent",
                most_inertial.map_or(0.0, |(share, _)| share * 100.0),
            ),
            ("edges_without_flow", still as f64),
        ];
        Check::from_findings(
            "graph",
            findings,
            metrics
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
            waivers,
        )
    }
}
