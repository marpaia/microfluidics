//! The function of a microfluidic chip: its fluid network extracted from what the design
//! states, solved, held to the design's requirements, and verified.

mod analysis;
mod bench;
mod close;
mod duct;
mod error;
mod graph;
mod linalg;
mod optimise;
mod project;
mod requirement;
mod scene;
mod session;
mod solve;
mod tolerance;
mod variation;
mod verify;

pub use crate::analysis::{Analysis, AnalysisError, analyse, each};
pub use crate::bench::{Benchmark, Score};
pub use crate::close::{Change, Closure, close};
pub use crate::duct::{WATER_DENSITY, resistance_per_viscosity, reynolds, wall_shear};
pub use crate::error::NetworkError;
pub use crate::graph::{Edge, Graph, extract};
pub use crate::optimise::{Fit, least_squares};
pub use crate::project::{Exported, Project, ProjectError, WaferExported, export_wafer, process_named};
pub use crate::requirement::{distances, evaluate, values, wanted};
pub use crate::scene::{Scene, scene};
pub use crate::session::{Host, serve};
pub use crate::solve::Solution;
pub use crate::tolerance::{Across, across, corners, yield_};
pub use crate::variation::Variation;
pub use crate::verify::{Group, Made, Report, Verified, group, verify};
