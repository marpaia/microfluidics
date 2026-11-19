//! What a build gives and takes: an elaborated design, overrides, and a process.

use std::collections::BTreeMap;

use pyo3::prelude::*;

use microfluidics_core::Sha256Hex;
use microfluidics_layout::{Elaborated, Overrides, Process, fingerprint};

use crate::error::stated;
use crate::quantity::{object, quantity};

/// A tunable as Python sees it: its path, value, low, high, and whether it is a count.
type Tuned = (String, Py<PyAny>, Py<PyAny>, Py<PyAny>, bool);

/// A design with every instance placed and every number final.
#[pyclass(name = "Elaborated", module = "microfluidics._engine")]
pub struct PyElaborated {
    pub inner: Elaborated,
}

#[pymethods]
impl PyElaborated {
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// The name of the process the design is made in, when it names one.
    #[getter]
    fn process(&self) -> Option<String> {
        self.inner.process.clone()
    }

    /// The name of the region each instance the design itself places is of.
    #[getter]
    fn placed(&self) -> Vec<String> {
        let own = self.inner.instances.iter().filter(|instance| instance.of_design());
        own.map(|instance| instance.region.clone()).collect()
    }

    /// What regions state about themselves, by path.
    #[getter]
    fn quantities(&self) -> BTreeMap<String, f64> {
        self.inner.quantities.clone()
    }

    /// The parameters the design allows to be tuned: `(path, value, low, high, count)`.
    fn tunables(&self, py: Python<'_>) -> PyResult<Vec<Tuned>> {
        let each = |tunable: &microfluidics_layout::Tunable| {
            Ok((
                tunable.path.clone(),
                object(py, tunable.value)?,
                object(py, tunable.low)?,
                object(py, tunable.high)?,
                tunable.count,
            ))
        };
        self.inner.tunables.iter().map(each).collect()
    }

    /// Where each named thing was made: `{path: (file, line)}`.
    fn origins(&self) -> BTreeMap<String, (String, u32)> {
        self.inner
            .origins
            .iter()
            .map(|(path, origin)| (path.clone(), (origin.file.clone(), origin.line)))
            .collect()
    }

    /// Record the files the design was built from, by name, each to be hashed.
    fn add_source(&mut self, name: &str, text: &[u8]) {
        self.inner
            .sources
            .insert(name.to_owned(), Sha256Hex::of_bytes(text).as_str().to_owned());
    }

    /// The design as JSON: what verification hashes.
    fn json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(|error| stated(error.to_string()))
    }

    /// The design's fingerprint as JSON: everything that must not change when the way it is
    /// written does.
    fn fingerprint(&self) -> String {
        fingerprint(&self.inner).to_string()
    }
}

/// Values that replace parameters, by path.
#[pyclass(name = "Overrides", module = "microfluidics._engine")]
#[derive(Clone)]
pub struct PyOverrides {
    pub inner: Overrides,
}

#[pymethods]
impl PyOverrides {
    #[new]
    #[pyo3(signature = (values=None))]
    fn new(values: Option<BTreeMap<String, Bound<'_, PyAny>>>) -> PyResult<Self> {
        let mut inner = Overrides::default();
        for (path, value) in values.unwrap_or_default() {
            inner.0.insert(path, quantity(&value)?);
        }
        Ok(Self { inner })
    }

    /// Read overrides from the text of a tuned-values file.
    #[staticmethod]
    fn parse(file: &str, text: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Overrides::parse(file, text).map_err(stated)?,
        })
    }

    /// The overrides as the text of a tuned-values file.
    fn toml(&self) -> String {
        self.inner.toml()
    }

    /// The value that replaces the parameter at `path`, when there is one.
    fn get(&self, py: Python<'_>, path: &str) -> PyResult<Option<Py<PyAny>>> {
        self.inner.0.get(path).map(|value| object(py, *value)).transpose()
    }

    fn paths(&self) -> Vec<String> {
        self.inner.0.keys().cloned().collect()
    }

    fn __len__(&self) -> usize {
        self.inner.0.len()
    }
}

/// A fabrication process.
#[pyclass(name = "Process", module = "microfluidics._engine")]
#[derive(Clone)]
pub struct PyProcess {
    pub inner: Process,
}

#[pymethods]
impl PyProcess {
    /// A process built into the tool.
    #[staticmethod]
    fn bundled(name: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Process::bundled(name).map_err(|error| stated(error.to_string()))?,
        })
    }

    /// A process read from the text of its file.
    #[staticmethod]
    fn parse(file: &str, text: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Process::parse(file, text).map_err(|error| stated(error.to_string()))?,
        })
    }

    /// The names of the processes built in.
    #[staticmethod]
    fn names() -> Vec<String> {
        Process::bundled_names()
    }

    /// The text of a built-in process's file.
    #[staticmethod]
    fn text(name: &str) -> Option<String> {
        Process::bundled_text(name).map(str::to_owned)
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }
}
