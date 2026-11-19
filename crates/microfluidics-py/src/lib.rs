//! The engine as the Python module `microfluidics._engine`.
//!
//! Bindings only: each class and function hands over to the Rust crates, and no rule of the
//! tool is stated here.

mod builder;
mod design;
mod engine;
mod error;
mod quantity;
mod session;

use pyo3::prelude::*;

use crate::builder::PyBuilder;
use crate::design::{PyElaborated, PyOverrides, PyProcess};
use crate::error::DesignError;
use crate::quantity::{FlowRate, Length, Pressure, PyQuantity, Time, Viscosity};

/// The extension module.
#[pymodule]
fn _engine(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("DesignError", module.py().get_type::<DesignError>())?;
    module.add_class::<PyQuantity>()?;
    module.add_class::<Length>()?;
    module.add_class::<Pressure>()?;
    module.add_class::<FlowRate>()?;
    module.add_class::<Viscosity>()?;
    module.add_class::<Time>()?;
    module.add_class::<PyBuilder>()?;
    module.add_class::<PyElaborated>()?;
    module.add_class::<PyOverrides>()?;
    module.add_class::<PyProcess>()?;
    module.add_function(wrap_pyfunction!(quantity::unit, module)?)?;
    module.add_function(wrap_pyfunction!(quantity::parse, module)?)?;
    module.add_function(wrap_pyfunction!(engine::beside, module)?)?;
    module.add_function(wrap_pyfunction!(engine::verify, module)?)?;
    module.add_function(wrap_pyfunction!(engine::close, module)?)?;
    module.add_function(wrap_pyfunction!(engine::export, module)?)?;
    module.add_function(wrap_pyfunction!(engine::corners, module)?)?;
    module.add_function(wrap_pyfunction!(engine::chip_yield, module)?)?;
    module.add_function(wrap_pyfunction!(engine::solve_bias, module)?)?;
    module.add_function(wrap_pyfunction!(engine::scene, module)?)?;
    module.add_function(wrap_pyfunction!(engine::keep, module)?)?;
    module.add_function(wrap_pyfunction!(engine::benchmark, module)?)?;
    module.add_function(wrap_pyfunction!(engine::score, module)?)?;
    module.add_function(wrap_pyfunction!(engine::export_wafer, module)?)?;
    module.add_function(wrap_pyfunction!(session::session, module)?)?;
    Ok(())
}
