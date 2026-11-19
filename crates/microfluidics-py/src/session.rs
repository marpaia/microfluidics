//! A Python design held for a viewer by the engine's session.

use std::cell::RefCell;
use std::io::{self, BufReader, ErrorKind};
use std::path::Path;

use pyo3::exceptions::PyKeyboardInterrupt;
use pyo3::prelude::*;

use microfluidics_layout::{BuildError, Elaborated, Overrides};
use microfluidics_network::{Host, serve};

use crate::design::PyOverrides;
use crate::engine::built;
use crate::error::{caught, stated};

/// The Python object that reads a design again, builds it and runs commands on it, as a session
/// holds it.
struct Held<'a, 'py> {
    py: Python<'py>,
    host: &'a Bound<'py, PyAny>,
    /// The interrupt that ends the session, once one was raised.
    interrupted: RefCell<Option<PyErr>>,
}

impl Held<'_, '_> {
    /// An exception as the fault the session reports. An interrupt is kept, to end the session.
    fn fault(&self, error: PyErr) -> BuildError {
        let fault = caught(self.py, &error);
        if error.is_instance_of::<PyKeyboardInterrupt>(self.py) {
            self.interrupted.replace(Some(error));
        }
        fault
    }
}

impl Host for Held<'_, '_> {
    fn reload(&mut self) -> Result<(), BuildError> {
        match self.host.call_method0("reload") {
            Ok(_) => Ok(()),
            Err(error) => Err(self.fault(error)),
        }
    }

    fn build(&self, overrides: &Overrides) -> Result<Elaborated, BuildError> {
        let given = PyOverrides {
            inner: overrides.clone(),
        };
        match self.host.call_method1("build", (given,)) {
            Ok(made) => built(self.py, Ok(made)),
            Err(error) => Err(self.fault(error)),
        }
    }

    fn run(&mut self, command: &str) -> (bool, String) {
        let said = self
            .host
            .call_method1("run", (command,))
            .and_then(|said| said.extract::<(bool, String)>());
        said.unwrap_or_else(|error| (false, self.fault(error).to_string()))
    }

    fn idle(&mut self) -> io::Result<()> {
        if let Err(error) = self.py.check_signals() {
            self.interrupted.replace(Some(error));
        }
        if self.interrupted.borrow().is_some() {
            return Err(io::Error::other("the session was interrupted"));
        }
        Ok(())
    }
}

/// Hold the design in `file` for a viewer on standard input and output until the viewer ends
/// the session. `host` reads the design again (`reload`), builds it with given values (`build`),
/// and runs a command on it (`run`).
#[pyfunction]
pub fn session<'py>(py: Python<'py>, file: &str, host: &Bound<'py, PyAny>) -> PyResult<()> {
    let mut held = Held {
        py,
        host,
        interrupted: RefCell::new(None),
    };
    let served = serve(
        Path::new(file),
        &mut held,
        BufReader::new(io::stdin()),
        &mut io::stdout().lock(),
    );
    match (held.interrupted.take(), served) {
        (Some(interrupt), _) => Err(interrupt),
        (None, Ok(())) => Ok(()),
        // A viewer that has gone has ended the session.
        (None, Err(error)) if error.kind() == ErrorKind::BrokenPipe => Ok(()),
        (None, Err(error)) => Err(stated(error.to_string())),
    }
}
