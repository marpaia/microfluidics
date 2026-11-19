//! The solver on small networks whose answers are known.

// A test stops on the first thing that is not as expected, in its helpers as in its cases.
#![allow(clippy::unwrap_used)]

use microfluidics_core::model::{Drive, Measure, Operating};
use microfluidics_network::{Edge, Graph, NetworkError};

/// A network of paths `(from, to, resistance)` between named vertices, each 1 m^3 of a 1 x 1 m duct.
fn network(paths: &[(&str, &str, f64)]) -> Graph {
    let mut vertices: Vec<String> = Vec::new();
    let at = |name: &str, vertices: &mut Vec<String>| {
        vertices.iter().position(|held| held == name).unwrap_or_else(|| {
            vertices.push(name.to_owned());
            vertices.len() - 1
        })
    };
    let edges = (paths.iter().enumerate())
        .map(|(index, (from, to, resistance))| Edge {
            element: format!("p{index}"),
            from: at(from, &mut vertices),
            to: at(to, &mut vertices),
            resistance_per_viscosity: *resistance,
            volume: 1.0,
            narrowest: (1.0, 1.0),
        })
        .collect();
    Graph::from_parts(vertices, edges)
}

fn run(drives: &[(&str, Drive)]) -> Operating {
    Operating {
        viscosity: 1.0,
        drives: drives
            .iter()
            .map(|(name, drive)| ((*name).to_owned(), *drive))
            .collect(),
        ..Operating::default()
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1.0)
}

#[test]
fn paths_in_series_share_one_flow_and_divide_the_pressure() {
    let network = network(&[("a", "b", 1.0), ("b", "c", 2.0)]);
    let operating = run(&[("a", Drive::Pressure(3.0)), ("c", Drive::Pressure(0.0))]);
    let solution = network.solve(&operating).unwrap();
    assert!(close(solution.flow[0], 1.0) && close(solution.flow[1], 1.0));
    assert!(close(solution.quantity(&Measure::Pressure, "b").unwrap(), 2.0));
    assert!(close(solution.drop("a", "b").unwrap(), 1.0));
    assert!(solution.check().metrics["conservation_residual"] < 1e-12);
}

#[test]
fn paths_in_parallel_share_the_flow_by_their_conductance() {
    let network = network(&[("a", "b", 1.0), ("a", "b", 3.0)]);
    let operating = run(&[("a", Drive::Pressure(3.0)), ("b", Drive::Pressure(0.0))]);
    let solution = network.solve(&operating).unwrap();
    assert!(close(solution.flow[0], 3.0) && close(solution.flow[1], 1.0));
}

#[test]
fn a_balanced_bridge_carries_nothing_across() {
    let network = network(&[
        ("a", "b", 1.0),
        ("a", "c", 2.0),
        ("b", "d", 2.0),
        ("c", "d", 4.0),
        ("b", "c", 1.0),
    ]);
    let operating = run(&[("a", Drive::Pressure(6.0)), ("d", Drive::Pressure(0.0))]);
    let solution = network.solve(&operating).unwrap();
    assert!(solution.flow[4].abs() < 1e-12, "{}", solution.flow[4]);
    assert!(close(solution.pressure[1], 4.0) && close(solution.pressure[2], 4.0));
    // Unbalanced, the bridge carries flow from the higher side to the lower.
    let tilted = network_with_bridge();
    let solved = tilted.solve(&operating).unwrap();
    assert!(solved.flow[4] > 0.0 && solved.check().metrics["conservation_residual"] < 1e-12);
}

fn network_with_bridge() -> Graph {
    network(&[
        ("a", "b", 1.0),
        ("a", "c", 2.0),
        ("b", "d", 2.0),
        ("c", "d", 1.0),
        ("b", "c", 1.0),
    ])
}

#[test]
fn a_flow_rate_may_be_driven_in_place_of_a_pressure() {
    let network = network(&[("a", "b", 1.0), ("b", "c", 2.0)]);
    let operating = run(&[("a", Drive::FlowRate(2.0)), ("c", Drive::Pressure(0.0))]);
    let solution = network.solve(&operating).unwrap();
    assert!(close(solution.flow[1], 2.0) && close(solution.pressure[0], 6.0));
}

#[test]
fn what_is_fed_mixes_in_proportion_to_its_flow() {
    let network = network(&[("a", "j", 1.0), ("b", "j", 3.0), ("j", "o", 1.0)]);
    let mut operating = run(&[
        ("a", Drive::Pressure(7.0)),
        ("b", Drive::Pressure(7.0)),
        ("o", Drive::Pressure(0.0)),
    ]);
    operating.feeds.insert("a".to_owned(), [("dye".to_owned(), 1.0)].into());
    let solution = network.solve(&operating).unwrap();
    let dye = Measure::Fraction {
        species: "dye".to_owned(),
    };
    assert!(close(solution.quantity(&dye, "o").unwrap(), 0.75));
    assert!(close(solution.quantity(&dye, "a").unwrap(), 1.0) && close(solution.quantity(&dye, "b").unwrap(), 0.0));
}

#[test]
fn a_network_that_cannot_be_solved_says_what_is_missing() {
    let apart = network(&[("a", "b", 1.0), ("c", "d", 1.0)]);
    let operating = run(&[("a", Drive::Pressure(1.0)), ("b", Drive::Pressure(0.0))]);
    assert_eq!(
        apart.solve(&operating).unwrap_err(),
        NetworkError::Floating("c".to_owned())
    );
    let nowhere = run(&[("z", Drive::Pressure(1.0))]);
    assert_eq!(
        apart.solve(&nowhere).unwrap_err(),
        NetworkError::UnknownDrive("z".to_owned())
    );
    let held = run(&[
        ("a", Drive::Pressure(1.0)),
        ("b", Drive::Pressure(0.0)),
        ("c", Drive::Pressure(0.0)),
        ("d", Drive::Pressure(0.0)),
    ]);
    let solved = apart.solve(&held).unwrap();
    assert_eq!(
        solved.graph.edge("nothing"),
        Err(NetworkError::Unknown("nothing".to_owned()))
    );
}
