//! Quantities: a value in SI units with its dimension, as a Python class for each dimension
//! the API takes.

use pyo3::basic::CompareOp;
use pyo3::prelude::*;

use microfluidics_core::{Dim, Quantity};

use crate::error::stated;

/// A value with a dimension. Sums need one dimension; products and quotients combine them.
#[pyclass(subclass, frozen, name = "Quantity", module = "microfluidics._engine")]
pub struct PyQuantity {
    pub inner: Quantity,
}

macro_rules! dimension {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[pyclass(extends = PyQuantity, frozen, module = "microfluidics._engine")]
        pub struct $name;
    };
}

dimension!(
    /// A length.
    Length
);
dimension!(
    /// A pressure, or a stress.
    Pressure
);
dimension!(
    /// A volume passing in a time.
    FlowRate
);
dimension!(
    /// A dynamic viscosity.
    Viscosity
);
dimension!(
    /// A time.
    Time
);

/// `quantity` as a Python value: a plain number when it has no dimension, and otherwise an
/// object of the class of its dimension.
pub fn object(py: Python<'_>, quantity: Quantity) -> PyResult<Py<PyAny>> {
    let base = PyClassInitializer::from(PyQuantity { inner: quantity });
    Ok(match quantity.dim {
        Dim::NONE => quantity.si.into_pyobject(py)?.into_any().unbind(),
        Dim::LENGTH => Py::new(py, base.add_subclass(Length))?.into_any(),
        Dim::PRESSURE => Py::new(py, base.add_subclass(Pressure))?.into_any(),
        Dim::FLOW => Py::new(py, base.add_subclass(FlowRate))?.into_any(),
        Dim::VISCOSITY => Py::new(py, base.add_subclass(Viscosity))?.into_any(),
        Dim::TIME => Py::new(py, base.add_subclass(Time))?.into_any(),
        _ => Py::new(py, base)?.into_any(),
    })
}

/// The quantity a Python value is: a `Quantity`, or a plain number.
pub fn quantity(value: &Bound<'_, PyAny>) -> PyResult<Quantity> {
    if let Ok(held) = value.cast::<PyQuantity>() {
        return Ok(held.get().inner);
    }
    match value.extract::<f64>() {
        Ok(number) => Ok(Quantity::number(number)),
        Err(_) => Err(stated(format!("{} is not a quantity or a number", value.repr()?))),
    }
}

/// A Python value as a number of the dimension `dim`, in SI units.
pub fn si(value: &Bound<'_, PyAny>, dim: Dim, what: &str) -> PyResult<f64> {
    quantity(value)?.expect(dim, what).map_err(stated)
}

#[pymethods]
impl PyQuantity {
    /// The value in SI units.
    #[getter]
    fn si(&self) -> f64 {
        self.inner.si
    }

    fn __repr__(&self) -> String {
        self.inner.written()
    }

    fn __hash__(&self) -> u64 {
        self.inner.si.to_bits()
            ^ ((self.inner.dim.length as u64) << 8
                | (self.inner.dim.mass as u64) << 16
                | (self.inner.dim.time as u64) << 24)
    }

    fn __add__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        object(py, self.inner.plus(quantity(other)?).map_err(stated)?)
    }

    fn __radd__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.__add__(py, other)
    }

    fn __sub__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        object(py, self.inner.minus(quantity(other)?).map_err(stated)?)
    }

    fn __rsub__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        object(py, quantity(other)?.minus(self.inner).map_err(stated)?)
    }

    fn __mul__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        object(py, self.inner.times(quantity(other)?))
    }

    fn __rmul__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        self.__mul__(py, other)
    }

    fn __truediv__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        object(py, self.inner.over(quantity(other)?))
    }

    fn __rtruediv__(&self, py: Python<'_>, other: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        object(py, quantity(other)?.over(self.inner))
    }

    fn __pow__(&self, py: Python<'_>, exponent: i8, _modulo: Option<i64>) -> PyResult<Py<PyAny>> {
        object(py, self.inner.powi(exponent))
    }

    fn __neg__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        object(
            py,
            Quantity {
                si: -self.inner.si,
                dim: self.inner.dim,
            },
        )
    }

    fn __abs__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        object(
            py,
            Quantity {
                si: self.inner.si.abs(),
                dim: self.inner.dim,
            },
        )
    }

    fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> PyResult<bool> {
        let Ok(other) = quantity(other) else {
            return Ok(matches!(op, CompareOp::Ne));
        };
        if other.dim != self.inner.dim {
            return match op {
                CompareOp::Eq => Ok(false),
                CompareOp::Ne => Ok(true),
                _ => Err(stated(format!(
                    "this compares {} and {}",
                    self.inner.dim.name(),
                    other.dim.name()
                ))),
            };
        }
        let (a, b) = (self.inner.si, other.si);
        Ok(match op {
            CompareOp::Lt => a < b,
            CompareOp::Le => a <= b,
            CompareOp::Eq => a == b,
            CompareOp::Ne => a != b,
            CompareOp::Gt => a > b,
            CompareOp::Ge => a >= b,
        })
    }
}

/// One of the unit `name`: `unit("um")` is a micrometre.
#[pyfunction]
pub fn unit(py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
    let one = Quantity::with_unit(1.0, name).ok_or_else(|| stated(format!("there is no unit `{name}`")))?;
    object(py, one)
}

/// The quantity `text` writes, such as `"412.6um"`.
#[pyfunction]
pub fn parse(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let read =
        Quantity::parse(text).ok_or_else(|| stated(format!("{text:?} is not a quantity such as \"412.6um\"")))?;
    object(py, read)
}
