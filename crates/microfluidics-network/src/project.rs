//! A design as it sits on disk: its file, what was tuned, what a person waived, and the
//! processes it names.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use microfluidics_core::{Check, Sha256Hex, Waiver};
use microfluidics_layout::{
    Build, BuildError, MaskSet, Origin, Overrides, Process, Wafer, WaferSpec, assemble, export, sheet, waivers,
};

use crate::analysis::AnalysisError;
use crate::close::{Closure, close};
use crate::verify::{Made, Report, Verified, verify};

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
    /// The processes the design names, in its order: each `<name>.toml` beside the design, or
    /// one built in.
    pub processes: Vec<Process>,
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
        // The processes are those the design names, which building it gives.
        let named = build(&overrides)?.processes;
        let dir = file.parent().unwrap_or(Path::new("."));
        let processes = (named.iter())
            .map(|name| process_named(name, dir))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            file: file.to_owned(),
            build,
            overrides,
            waivers,
            processes,
        })
    }

    /// Where what closure tunes is kept.
    pub fn tuned_path(&self) -> PathBuf {
        self.file.with_extension("tuned.toml")
    }

    /// Every check, with the design as it is tuned.
    pub fn verify(&self) -> Result<Verified, ProjectError> {
        Ok(verify(self.build, &self.overrides, &self.processes, &self.waivers)?)
    }

    /// Tune the design, from its source, until its requirements hold, and keep what was tuned
    /// beside it.
    pub fn close(&mut self) -> Result<Closure, ProjectError> {
        let closure = close(self.build, &Overrides::default(), &self.processes, &self.waivers)?;
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

    /// Verify the design and write what fabrication needs into `dir`: the elaborated design as
    /// `design.json`, every check as `report.json`, and for each process a folder of its name
    /// holding its masks, its report as `REPORT.md`, and the sheet for its fab when it names one.
    /// A design that names no process has its masks and report written into `dir` itself.
    pub fn export(&self, dir: &Path) -> Result<Exported, ProjectError> {
        let verified = self.verify()?;
        let mut report = verified.report;
        let mut written = Vec::new();
        for made in &verified.made {
            let name = made.process.as_ref().map(|process| process.name.as_str());
            let folder = name.map_or_else(String::new, |name| format!("{name}/"));
            let wrote = export(&made.masks, made.process.as_ref(), &dir.join(&folder).join("masks"))
                .map_err(ProjectError::of)?;
            // The GDSII file was already read back in verification; the drawings are read back here.
            let read_back = wrote.checks.into_iter().filter(|check| check.id != "gds-readback");
            report.checks.extend(read_back.map(|check| check.for_process(name)));
            let mut files: Vec<String> = wrote.files.iter().map(|file| format!("masks/{file}")).collect();
            // The sheet for whoever makes the chip, naming each mask file by its hash.
            let on_masks = |check: &&Check| check.id == "mask-rules" && check.process.as_deref() == name;
            if let (Some(process), Some(rules)) = (&made.process, report.checks.iter().find(on_masks)) {
                let mut hashed = Vec::new();
                for file in &files {
                    let hash = Sha256Hex::of_file(&dir.join(&folder).join(file)).map_err(ProjectError::of)?;
                    hashed.push((file.clone(), hash.as_str().to_owned()));
                }
                let chip = &verified.analysis.elaborated.chip;
                if let Some(sheet) = sheet(&report.design, chip, &made.masks, process, rules, &hashed) {
                    write(&dir.join(&folder).join(sheet.file), &sheet.text)?;
                    files.insert(0, sheet.file.to_owned());
                }
            }
            written.push((folder, files));
        }
        report.judge();
        let elaborated = serde_json::to_string_pretty(&verified.analysis.elaborated).map_err(ProjectError::of)?;
        write(&dir.join("design.json"), &elaborated)?;
        write(&dir.join("report.json"), &report.json())?;
        let mut files: Vec<String> = ["design.json", "report.json"].map(str::to_owned).into();
        for (made, (folder, written)) in verified.made.iter().zip(written) {
            let bearing = match &made.process {
                Some(process) => report.for_process(&process.name),
                None => report.clone(),
            };
            write(&dir.join(&folder).join("REPORT.md"), &bearing.markdown())?;
            files.push(format!("{folder}REPORT.md"));
            files.extend(written.iter().map(|file| format!("{folder}{file}")));
        }
        Ok(Exported { report, files })
    }
}

/// The chip as the process `wanted` makes it, among those `verified` holds; with none wanted,
/// the only one there is.
fn made_in<'a>(verified: &'a Verified, wanted: Option<&str>, design: &str) -> Result<&'a Made, ProjectError> {
    let named = |made: &&Made| made.process.as_ref().map(|process| process.name.as_str()) == wanted;
    match (wanted, verified.made.as_slice()) {
        (None, [only]) => Ok(only),
        (None, _) => Err(ProjectError::of(format!(
            "`{design}` names {} processes; the wafer must say which it is made in with `process = \"<name>\"`",
            verified.made.len()
        ))),
        (Some(name), made) => made
            .iter()
            .find(named)
            .ok_or_else(|| ProjectError::of(format!("the wafer is made in `{name}`, which `{design}` does not name"))),
    }
}

/// Assemble the wafer the file `file` states and write its masks into `dir`. `open` signs off
/// the design in a file the wafer names, which lies beside the wafer's own.
pub fn export_wafer(
    file: &Path,
    dir: &Path,
    open: &dyn Fn(&Path) -> Result<Verified, ProjectError>,
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
        let verified = open(&beside.join(&block.design))?;
        let made = made_in(&verified, spec.process.as_deref(), &block.design)?;
        let passes_there = match &made.process {
            Some(process) => verified.report.for_process(&process.name).passes,
            None => verified.report.passes,
        };
        passes.insert(block.design.clone(), passes_there);
        chips.insert(block.design.clone(), made.masks.clone());
        process = process.or(made.process.clone());
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
