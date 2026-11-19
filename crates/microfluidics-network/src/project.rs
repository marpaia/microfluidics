//! A design as it sits on disk: its file, what was tuned, what a person waived, and the
//! process it names.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use microfluidics_core::{Check, Waiver};
use microfluidics_layout::{
    Build, BuildError, MaskSet, Origin, Overrides, Process, Wafer, WaferSpec, assemble, export, prepare, waivers,
};

use crate::analysis::AnalysisError;
use crate::close::{Closure, close};
use crate::verify::{Report, Verified, verify};

/// Why a design, or something beside it, could not be read, built or written.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectError {
    pub message: String,
    /// The line of the design that caused it, when it is a fault in the design.
    pub origin: Option<Origin>,
}

impl ProjectError {
    /// A fault that is not about a line of a design.
    pub(crate) fn of(error: impl fmt::Display) -> Self {
        Self {
            message: error.to_string(),
            origin: None,
        }
    }
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.origin {
            Some(origin) => write!(f, "{}:{}: {}", origin.file, origin.line, self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for ProjectError {}

impl From<BuildError> for ProjectError {
    fn from(fault: BuildError) -> Self {
        Self {
            message: fault.message,
            origin: fault.origin,
        }
    }
}

impl From<AnalysisError> for ProjectError {
    fn from(error: AnalysisError) -> Self {
        match error {
            AnalysisError::Design(fault) => fault.into(),
            AnalysisError::Graph(error) => Self::of(error),
        }
    }
}

/// A design with what sits beside its file.
pub struct Project<'a> {
    /// The file the design is written in.
    pub file: PathBuf,
    /// The design, as something that can be built again with other values.
    pub build: Build<'a>,
    /// What closure tuned: `<design>.tuned.toml`, when it is there.
    pub overrides: Overrides,
    /// What a person waived: `<design>.waivers.toml`, when it is there.
    pub waivers: Vec<Waiver>,
    /// The process the design names: `<name>.toml` beside the design, or one built in.
    pub process: Option<Process>,
}

/// What an export wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Exported {
    /// The verification report, with what reading the written files back found.
    pub report: Report,
    /// Every file written, by its path within the directory.
    pub files: Vec<String>,
}

/// A wafer written to disk.
#[derive(Debug, Clone, PartialEq)]
pub struct WaferExported {
    pub wafer: Wafer,
    /// Whether each design on the wafer passes verification, by its file as the wafer names it.
    pub passes: BTreeMap<String, bool>,
    /// What reading the written files back found.
    pub checks: Vec<Check>,
    pub files: Vec<String>,
}

fn read(path: &Path) -> Result<String, ProjectError> {
    fs::read_to_string(path).map_err(|error| ProjectError::of(format!("{}: {error}", path.display())))
}

fn write(path: &Path, text: &str) -> Result<(), ProjectError> {
    fs::write(path, text).map_err(|error| ProjectError::of(format!("{}: {error}", path.display())))
}

/// The process named `name`: the file `<name>.toml` in `dir`, or else one built in.
pub fn process_named(name: &str, dir: &Path) -> Result<Process, ProjectError> {
    let beside = dir.join(format!("{name}.toml"));
    match fs::read_to_string(&beside) {
        Ok(text) => Process::parse(&beside.display().to_string(), &text).map_err(ProjectError::of),
        Err(_) => Process::bundled(name).map_err(ProjectError::of),
    }
}

impl<'a> Project<'a> {
    /// The design `build` makes, with what sits beside the file it is written in.
    pub fn beside(file: &Path, build: Build<'a>) -> Result<Self, ProjectError> {
        let waived = file.with_extension("waivers.toml");
        let waivers = match fs::read_to_string(&waived) {
            Ok(text) => waivers(&waived.display().to_string(), &text).map_err(ProjectError::of)?,
            Err(_) => Vec::new(),
        };
        let tuned = file.with_extension("tuned.toml");
        let overrides = match fs::read_to_string(&tuned) {
            Ok(text) => Overrides::parse(&tuned.display().to_string(), &text).map_err(ProjectError::of)?,
            Err(_) => Overrides::default(),
        };
        // The process is the one the design names, which building it gives.
        let named = build(&overrides)?.process;
        let dir = file.parent().unwrap_or(Path::new("."));
        let process = named.map(|name| process_named(&name, dir)).transpose()?;
        Ok(Self {
            file: file.to_owned(),
            build,
            overrides,
            waivers,
            process,
        })
    }

    /// Where what closure tunes is kept.
    pub fn tuned_path(&self) -> PathBuf {
        self.file.with_extension("tuned.toml")
    }

    /// Every check, with the design as it is tuned.
    pub fn verify(&self) -> Result<Verified, ProjectError> {
        Ok(verify(
            self.build,
            &self.overrides,
            self.process.as_ref(),
            &self.waivers,
        )?)
    }

    /// Tune the design, from its source, until its requirements hold, and keep what was tuned
    /// beside it.
    pub fn close(&mut self) -> Result<Closure, ProjectError> {
        let closure = close(self.build, &Overrides::default(), self.process.as_ref(), &self.waivers)?;
        write(&self.tuned_path(), &closure.overrides.toml())?;
        self.overrides = closure.overrides.clone();
        Ok(closure)
    }

    /// Keep `values` beside the design, over what is kept there already. They are kept only
    /// when the design builds with them.
    pub fn keep(&mut self, values: &Overrides) -> Result<(), ProjectError> {
        let mut kept = self.overrides.clone();
        kept.0
            .extend(values.0.iter().map(|(path, value)| (path.clone(), *value)));
        (self.build)(&kept)?;
        write(&self.tuned_path(), &kept.toml())?;
        self.overrides = kept;
        Ok(())
    }

    /// Verify the design and write what fabrication needs into `dir`: the masks, the report
    /// as `REPORT.md` and `report.json`, and the elaborated design as `design.json`.
    pub fn export(&self, dir: &Path) -> Result<Exported, ProjectError> {
        let verified = self.verify()?;
        let written = export(&verified.masks, self.process.as_ref(), &dir.join("masks")).map_err(ProjectError::of)?;
        let mut report = verified.report;
        // The GDSII file was already read back in verification; the drawings are read back here.
        report
            .checks
            .extend(written.checks.into_iter().filter(|check| check.id != "gds-readback"));
        report.judge();
        let elaborated = serde_json::to_string_pretty(&verified.analysis.elaborated).map_err(ProjectError::of)?;
        write(&dir.join("report.json"), &report.json())?;
        write(&dir.join("REPORT.md"), &report.markdown())?;
        write(&dir.join("design.json"), &elaborated)?;
        let mut files: Vec<String> = ["REPORT.md", "report.json", "design.json"].map(str::to_owned).into();
        files.extend(written.files.iter().map(|file| format!("masks/{file}")));
        Ok(Exported { report, files })
    }
}

/// A design a wafer names, verified: what `open` gives for its file.
pub type Opened = (Verified, Option<Process>);

/// Assemble the wafer the file `file` states and write its masks into `dir`. `open` signs off
/// the design in a file the wafer names, which lies beside the wafer's own.
pub fn export_wafer(
    file: &Path,
    dir: &Path,
    open: &dyn Fn(&Path) -> Result<Opened, ProjectError>,
) -> Result<WaferExported, ProjectError> {
    let spec = WaferSpec::parse(&file.display().to_string(), &read(file)?).map_err(ProjectError::of)?;
    let beside = file.parent().unwrap_or(Path::new("."));
    let mut chips: BTreeMap<String, MaskSet> = BTreeMap::new();
    let mut passes = BTreeMap::new();
    let mut process = None;
    for block in &spec.blocks {
        if chips.contains_key(&block.design) {
            continue;
        }
        let (verified, named) = open(&beside.join(&block.design))?;
        passes.insert(block.design.clone(), verified.report.passes);
        chips.insert(
            block.design.clone(),
            prepare(&verified.analysis.elaborated.chip, named.as_ref()),
        );
        process = process.or(named);
    }
    let wafer = assemble(&spec, &chips).map_err(ProjectError::of)?;
    let written = export(&wafer.masks, process.as_ref(), dir).map_err(ProjectError::of)?;
    Ok(WaferExported {
        wafer,
        passes,
        checks: written.checks,
        files: written.files,
    })
}
