//! What a design can be asked: each takes the design's file and a function that builds it, and
//! gives its answer as JSON text.

use std::path::Path;

use pyo3::prelude::*;
use serde_json::{Value, json};

use microfluidics_core::Check;
use microfluidics_layout::{BuildError, Elaborated, Overrides, solve_bias as bias_for};
use microfluidics_network::{
    Benchmark, Closure, Project, ProjectError, Report, analyse, corners as at_corners, export_wafer as wafer_of,
    scene as scene_of, yield_ as over_chips,
};

use crate::design::{PyElaborated, PyOverrides};
use crate::error::{caught, raised, reported, stated};

/// What a Python build gave, as the engine takes it: the elaborated design, or the fault raised.
pub(crate) fn built(py: Python<'_>, made: PyResult<Bound<'_, PyAny>>) -> Result<Elaborated, BuildError> {
    let made = made.map_err(|error| caught(py, &error))?;
    let made = made
        .cast::<PyElaborated>()
        .map_err(|_| BuildError::new("the design's build did not give an elaborated design"))?;
    Ok(made.borrow().inner.clone())
}

/// A Python function that builds a design, as the engine calls it.
fn building<'py>(
    py: Python<'py>,
    build: &'py Bound<'py, PyAny>,
) -> impl Fn(&Overrides) -> Result<Elaborated, BuildError> + 'py {
    move |overrides| {
        let given = PyOverrides {
            inner: overrides.clone(),
        };
        built(py, build.call1((given,)))
    }
}

fn failed(error: ProjectError) -> PyErr {
    match error.origin {
        Some(origin) => raised(BuildError::at(&origin, error.message)),
        None => reported(&error.message),
    }
}

fn text(value: &Value) -> String {
    value.to_string()
}

fn report(report: &Report) -> Value {
    json!({ "design": report.design, "inputs": report.inputs, "checks": report.checks, "passes": report.passes, "problems": report.problems })
}

/// What sits beside a design's file: its tuned values, how many waivers, and its processes.
#[pyfunction]
pub fn beside(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>) -> PyResult<String> {
    let build = building(py, build);
    let project = Project::beside(Path::new(file), &build).map_err(failed)?;
    let processes: Vec<&str> = (project.processes.iter())
        .map(|process| process.name.as_str())
        .collect();
    let tuned = project.tuned_path().display().to_string();
    Ok(text(
        &json!({ "tuned": project.overrides.toml(), "tuned_path": tuned, "waivers": project.waivers.len(), "processes": processes }),
    ))
}

/// Every check on the design, as it is tuned.
#[pyfunction]
pub fn verify(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>) -> PyResult<String> {
    let build = building(py, build);
    let verified = Project::beside(Path::new(file), &build)
        .map_err(failed)?
        .verify()
        .map_err(failed)?;
    Ok(text(&report(&verified.report)))
}

fn closure(closure: &Closure, tuned: String) -> Value {
    let changes: Vec<Value> = (closure.changes.iter())
        .map(|change| json!({ "path": change.path, "start": change.from.written(), "end": change.to.written() }))
        .collect();
    json!({
        "converged": closure.converged, "iterations": closure.iterations, "changes": changes,
        "distances": closure.distances, "limiting": closure.limiting, "tuned": closure.overrides.toml(), "tuned_path": tuned,
    })
}

/// Tune the design until its requirements hold, and keep what was tuned beside its file.
#[pyfunction]
pub fn close(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>) -> PyResult<String> {
    let build = building(py, build);
    let mut project = Project::beside(Path::new(file), &build).map_err(failed)?;
    let closed = project.close().map_err(failed)?;
    Ok(text(&closure(&closed, project.tuned_path().display().to_string())))
}

/// Verify the design and write its masks, report and elaborated design into `directory`.
#[pyfunction]
pub fn export(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>, directory: &str) -> PyResult<String> {
    let build = building(py, build);
    let exported = Project::beside(Path::new(file), &build)
        .map_err(failed)?
        .export(Path::new(directory))
        .map_err(failed)?;
    let mut answer = report(&exported.report);
    answer["files"] = json!(exported.files);
    Ok(text(&answer))
}

/// One check for each process the design names, each needing the design analysed and that
/// process's tolerances.
fn across(
    py: Python<'_>,
    file: &str,
    build: &Bound<'_, PyAny>,
    run: impl Fn(&microfluidics_network::Analysis, &microfluidics_layout::Process) -> Check,
) -> PyResult<String> {
    let build = building(py, build);
    let project = Project::beside(Path::new(file), &build).map_err(failed)?;
    let analysis = analyse(&build, &project.overrides, &project.processes, &project.waivers).map_err(raised)?;
    if project.processes.is_empty() {
        return Err(stated(
            "the design names no process, so there are no tolerances to vary",
        ));
    }
    let checks: Vec<Check> = (project.processes.iter())
        .map(|process| run(&analysis, process).for_process(Some(&process.name)))
        .collect();
    Ok(text(&json!(checks)))
}

/// Whether the design's requirements hold at every corner of each of its processes.
#[pyfunction]
pub fn corners(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>) -> PyResult<String> {
    across(py, file, build, at_corners)
}

/// The share of chips, made within each process's tolerances, that meet every requirement.
#[pyfunction]
pub fn chip_yield(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>, chips: usize, seed: u64) -> PyResult<String> {
    across(py, file, build, |analysis, process| {
        over_chips(analysis, process, chips, seed)
    })
}

/// For each process that states a print model, and each layer: the mask bias in micrometres at
/// which the layer's narrowest feature prints as drawn, with the bias the process states.
#[pyfunction]
pub fn solve_bias(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>) -> PyResult<String> {
    let build = building(py, build);
    let project = Project::beside(Path::new(file), &build).map_err(failed)?;
    let printing: Vec<_> = (project.processes.iter())
        .filter(|process| process.print.is_some())
        .collect();
    if printing.is_empty() {
        return Err(stated(
            "no process the design names states a print model to find a bias by",
        ));
    }
    let chip = build(&project.overrides).map_err(raised)?.chip;
    let mut layers: Vec<Value> = Vec::new();
    for process in printing {
        let solved = bias_for(&chip, process);
        layers.extend(chip.layers.iter().map(|layer| {
            json!({
                "process": process.name, "layer": layer.name,
                "bias_um": solved.get(&layer.name).map(|bias| bias * 1e6 + 0.0),
                "stated_um": process.mask(&layer.name).bias * 1e6 + 0.0,
            })
        }));
    }
    Ok(text(&json!(layers)))
}

/// Keep the values in `values`, the text of tuned values, beside the design's file, over what
/// is kept there already. Gives the text of what is now kept.
#[pyfunction]
pub fn keep(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>, values: &str) -> PyResult<String> {
    let build = building(py, build);
    let mut project = Project::beside(Path::new(file), &build).map_err(failed)?;
    let values = Overrides::parse("values to keep", values).map_err(stated)?;
    project.keep(&values).map_err(failed)?;
    Ok(project.overrides.toml())
}

/// The scene of the design: what a viewer draws. With `trial`, the text of tuned values, those
/// are used over what is kept beside the design, and nothing is written.
#[pyfunction]
#[pyo3(signature = (file, build, trial=None))]
pub fn scene(py: Python<'_>, file: &str, build: &Bound<'_, PyAny>, trial: Option<&str>) -> PyResult<String> {
    let build = building(py, build);
    let mut project = Project::beside(Path::new(file), &build).map_err(failed)?;
    if let Some(trial) = trial {
        let trial = Overrides::parse("trial values", trial).map_err(stated)?;
        project.overrides.0.extend(trial.0);
    }
    let verified = project.verify().map_err(failed)?;
    serde_json::to_string(&scene_of(&verified)).map_err(|error| stated(error.to_string()))
}

/// A benchmark: its name, its brief, its process and the regions a design may place.
#[pyfunction]
pub fn benchmark(directory: &str) -> PyResult<String> {
    let held = Benchmark::open(Path::new(directory)).map_err(failed)?;
    Ok(text(
        &json!({ "name": held.name, "brief": held.brief, "process": held.process.name, "regions": held.regions }),
    ))
}

/// A benchmark's score for a candidate. `build` makes the candidate with the benchmark's
/// conditions in place of its own; what was tuned beside `file` is used.
#[pyfunction]
pub fn score(py: Python<'_>, directory: &str, file: &str, build: &Bound<'_, PyAny>) -> PyResult<String> {
    let held = Benchmark::open(Path::new(directory)).map_err(failed)?;
    let build = building(py, build);
    let tuned = Path::new(file).with_extension("tuned.toml");
    let overrides = match std::fs::read_to_string(&tuned) {
        Ok(text) => Overrides::parse(&tuned.display().to_string(), &text).map_err(stated)?,
        Err(_) => Overrides::default(),
    };
    let scored = held.score(&build, &overrides).map_err(failed)?;
    let mut answer = report(&scored.report);
    answer["benchmark"] = json!(held.name);
    answer["requirements_met"] = json!(scored.met);
    answer["requirements"] = json!(scored.of);
    answer["tuned_values"] = json!(overrides.0.len());
    Ok(text(&answer))
}

/// Assemble a wafer and write its masks. `open` gives, for a design's file, the function that
/// builds it.
#[pyfunction]
pub fn export_wafer(py: Python<'_>, file: &str, directory: &str, open: &Bound<'_, PyAny>) -> PyResult<String> {
    let opened = |design: &Path| {
        let build = open
            .call1((design.display().to_string(),))
            .map_err(|error| ProjectError::from(caught(py, &error)))?;
        let build = building(py, &build);
        let project = Project::beside(design, &build)?;
        project.verify()
    };
    let exported = wafer_of(Path::new(file), Path::new(directory), &opened).map_err(failed)?;
    let chips: Vec<Value> = (exported.wafer.placements.iter())
        .map(|chip| json!({ "label": chip.label, "design": chip.design, "x_mm": chip.origin.x * 1e3, "y_mm": chip.origin.y * 1e3 }))
        .collect();
    Ok(text(&json!({
        "chips": chips, "alignment_marks": exported.wafer.alignment_marks, "passes": exported.passes,
        "checks": exported.checks, "files": exported.files,
    })))
}
