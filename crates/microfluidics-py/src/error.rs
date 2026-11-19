//! Faults in a design, as Python sees them.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use microfluidics_layout::{BuildError, Origin};

create_exception!(
    _engine,
    DesignError,
    PyException,
    "A fault in a design: its message, and the file and line that caused it when known."
);

/// A fault as the exception Python raises: its arguments are the message, the file and the line.
pub fn raised(fault: BuildError) -> PyErr {
    let (file, line) = fault
        .origin
        .map_or((None, None), |origin| (Some(origin.file), Some(origin.line)));
    DesignError::new_err((fault.message, file, line))
}

/// A fault that is not about a line of a design.
pub fn stated(message: impl Into<String>) -> PyErr {
    DesignError::new_err((message.into(), None::<String>, None::<u32>))
}

/// A fault the engine gives as text, as the exception Python raises. Text that begins with a
/// file and a line, as a fault in a design does, gives them back as the exception's own.
pub fn reported(text: &str) -> PyErr {
    let located = text.split_once(": ").and_then(|(place, message)| {
        let (file, line) = place.rsplit_once(':')?;
        Some((message.to_owned(), file.to_owned(), line.parse::<u32>().ok()?))
    });
    match located {
        Some((message, file, line)) => DesignError::new_err((message, Some(file), Some(line))),
        None => stated(text),
    }
}

/// An exception raised while a design was built, as the fault the engine reports.
pub fn caught(py: Python<'_>, error: &PyErr) -> BuildError {
    let value = error.value(py);
    let stated = value
        .getattr("args")
        .and_then(|args| args.extract::<(String, Option<String>, Option<u32>)>());
    match stated {
        Ok((message, Some(file), Some(line))) if error.is_instance_of::<DesignError>(py) => {
            BuildError::at(&Origin::new(&file, line), message)
        }
        Ok((message, _, _)) if error.is_instance_of::<DesignError>(py) => BuildError::new(message),
        _ => BuildError::new(value.to_string()),
    }
}
