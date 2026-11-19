//! Data types that pass between crates.

mod edge;
mod layer;
mod requirement;

pub use crate::model::edge::{DuctSpec, EdgeSpec, PortKind};
pub use crate::model::layer::{LayerSpec, LayerStack};
pub use crate::model::requirement::{Bound, Drive, Measure, Operating, Requirement};
