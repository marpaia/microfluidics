//! Benchmarks: a task stated once, by which any design for it is scored.
//!
//! A benchmark is a directory holding a brief in plain words (`brief.md`) and, in
//! `benchmark.toml`, its name, its process and the regions a design may place. The conditions a
//! candidate is run and judged under are written beside them, in `conditions.py`, and are put
//! in place of the candidate's own when it is built, so that no design is judged by
//! requirements it chose.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use microfluidics_core::{Check, Finding};
use microfluidics_layout::{Build, Overrides, Process};

use crate::project::{ProjectError, process_named};
use crate::verify::{Report, verify};

#[derive(Debug, Clone)]
pub struct Benchmark {
    pub name: String,
    /// The task, in plain words.
    pub brief: String,
    pub process: Process,
    /// The regions a candidate may place.
    pub regions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Written {
    name: String,
    process: String,
    regions: Vec<String>,
}

/// How a candidate did.
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub report: Report,
    /// How many of the benchmark's requirements hold, and how many there are.
    pub met: usize,
    pub of: usize,
}

impl Benchmark {
    /// Read the benchmark in the directory `dir`.
    pub fn open(dir: &Path) -> Result<Self, ProjectError> {
        let read = |name: &str| {
            let path = dir.join(name);
            fs::read_to_string(&path).map_err(|error| ProjectError::of(format!("{}: {error}", path.display())))
        };
        let stated = dir.join("benchmark.toml").display().to_string();
        let written: Written =
            toml::from_str(&read("benchmark.toml")?).map_err(|error| ProjectError::of(format!("{stated}: {error}")))?;
        let process = process_named(&written.process, dir)?;
        Ok(Self {
            name: written.name,
            brief: read("brief.md")?,
            process,
            regions: written.regions,
        })
    }

    /// Verify a candidate under this benchmark's process, with no waivers. `candidate`
    /// builds the design with the benchmark's conditions in place of its own.
    pub fn score(&self, candidate: Build<'_>, overrides: &Overrides) -> Result<Score, ProjectError> {
        let verified = verify(candidate, overrides, Some(&self.process), &[])?;
        let mut findings = Vec::new();
        let mut placed: BTreeMap<&str, usize> = BTreeMap::new();
        let instances = &verified.analysis.elaborated.instances;
        for instance in instances.iter().filter(|instance| instance.of_design()) {
            *placed.entry(instance.region.as_str()).or_default() += 1;
        }
        for region in placed
            .keys()
            .filter(|region| !self.regions.iter().any(|allowed| allowed == *region))
        {
            let message = format!(
                "the design places a `{region}`, which this benchmark does not allow; it allows {}",
                self.regions.join(", ")
            );
            findings.push(Finding {
                rule: "benchmark/regions".to_owned(),
                subject: (*region).to_owned(),
                measured: None,
                limit: None,
                message,
                waived: None,
            });
        }
        let metrics = placed
            .into_iter()
            .map(|(region, count)| (region.to_owned(), count as f64))
            .collect();
        let allowed = Check::from_findings("benchmark/regions", findings, metrics, &[]);

        let mut report = verified.report;
        report.checks.insert(0, allowed);
        report.judge();
        let required: Vec<&Check> = report
            .checks
            .iter()
            .filter(|check| check.id.starts_with("requirement/"))
            .collect();
        let (met, of) = (required.iter().filter(|check| check.passes()).count(), required.len());
        Ok(Score { report, met, of })
    }
}
