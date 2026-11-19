//! Verification: every check a design must pass, run in order, and the report of them.

use std::collections::BTreeMap;

use serde::Serialize;

use microfluidics_core::model::{Measure, Requirement};
use microfluidics_core::{Check, Finding, Sha256Hex, Status, Waiver};
use microfluidics_layout::{
    Build, Elaborated, Features, MaskSet, Overrides, Process, check, gds_readback, gds_write, prepare, print_check,
};

use crate::analysis::{Analysis, AnalysisError, analyse};
use crate::graph::extract;
use crate::requirement::evaluate;
use crate::tolerance::{Across, across, corners_of, yield_};
use crate::variation::Variation;

/// How many chips the yield is found over, and the seed they are drawn from.
const YIELD_CHIPS: usize = 500;
const YIELD_SEED: u64 = 1;

/// What was checked, against what, and whether the design may go to fabrication.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub design: String,
    /// The hash of everything the result depends on: each source file read, the process, and
    /// the elaborated design.
    pub inputs: BTreeMap<String, String>,
    pub checks: Vec<Check>,
    /// True only when every check is satisfied or waived. A check that did not run stands in the way.
    pub passes: bool,
    /// How many things stand in the way: each finding that is not waived, and each check that
    /// did not run.
    pub problems: usize,
}

/// The question a check answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    /// Whether the chip can be made in the process.
    Fabrication,
    /// Whether the chip, made as drawn, does what the design asks.
    Function,
    /// Whether it still does across the chips the process makes.
    Variation,
    /// Whether what the tool itself wrote is sound.
    Integrity,
}

/// The question the check `id` answers.
pub fn group(id: &str) -> Group {
    match id {
        "rules" | "mask-rules" | "print" | "benchmark/regions" => Group::Fabrication,
        "corners" | "yield" => Group::Variation,
        "deterministic" | "gds-readback" | "dxf-readback" => Group::Integrity,
        _ => Group::Function,
    }
}

/// A report with the analysis and masks it was made from.
#[derive(Debug, Clone, PartialEq)]
pub struct Verified {
    pub report: Report,
    pub analysis: Analysis,
    pub masks: MaskSet,
    /// How each requirement fares across the corners of the process, when that was found.
    pub across: BTreeMap<String, Across>,
}

/// What a check is called where a person reads it.
pub fn title(id: &str) -> String {
    let known = match id {
        "deterministic" => "Reproducible build",
        "rules" => "Design rules",
        "connectivity" => "Connections",
        "mask-rules" => "Design rules after mask preparation",
        "gds-readback" => "Mask file round trip",
        "dxf-readback" => "Mask drawings round trip",
        "print" => "Printable features",
        "graph" => "Flow",
        "corners" => "Fabrication variation",
        "yield" => "Yield",
        "benchmark/regions" => "Regions the benchmark allows",
        _ => "",
    };
    match (known, id.strip_prefix("requirement/")) {
        ("", Some(name)) => format!("Requirement `{name}`"),
        ("", None) => id.to_owned(),
        (known, _) => known.to_owned(),
    }
}

/// Whether two builds of one design, with the same values, are the same design.
fn deterministic(first: &Elaborated, second: &Elaborated) -> Check {
    let same = Sha256Hex::of_json(first).ok() == Sha256Hex::of_json(second).ok();
    let findings = if same {
        Vec::new()
    } else {
        let message = "the design built twice with the same values is not the same design: it depends on something other than its parameters".to_owned();
        vec![Finding {
            rule: "deterministic".to_owned(),
            subject: first.name.clone(),
            measured: None,
            limit: None,
            message,
            waived: None,
        }]
    };
    Check::from_findings("deterministic", findings, BTreeMap::new(), &[])
}

/// Run every check on `design`: its rules as drawn, its connectivity, its masks' rules, the
/// GDSII file read back, how its masks print, the network solved, each requirement, and the
/// process's corners; and first, that it builds the same way twice.
pub fn verify(
    build: Build<'_>,
    overrides: &Overrides,
    process: Option<&Process>,
    waivers: &[Waiver],
) -> Result<Verified, AnalysisError> {
    let analysis = analyse(build, overrides, process, waivers)?;
    let chip = &analysis.elaborated.chip;
    // A design is code, and code can read the clock or a random number. It is built a second
    // time, and must come out the same.
    let again = build(overrides)?;
    let mut checks = vec![deterministic(&analysis.elaborated, &again)];
    checks.extend(analysis.checks.iter().cloned());

    let masks = prepare(chip, process);
    let mut on_masks = check(
        &Features::prepared(&masks, chip),
        process.map_or(&[][..], |process| &process.rules),
        waivers,
    );
    on_masks.id = "mask-rules".to_owned();
    checks.push(on_masks);
    // The file holds the masks as written, which may be mirrored; it is compared with those.
    let written = masks.as_written();
    checks.push(gds_readback(&written, &gds_write(&written)));
    if let Some(model) = process.and_then(|process| process.print) {
        checks.push(print_check(&masks, &model));
    }

    // Requirements measured at places; yield, of the whole chip, is found with the tolerances.
    let requirements: Vec<&Requirement> = analysis
        .elaborated
        .requirements
        .iter()
        .filter(|requirement| requirement.measure != Measure::Yield)
        .collect();
    let requirements = &requirements;
    let unanswered = |why: &str, checks: &mut Vec<Check>| {
        checks.extend(
            requirements
                .iter()
                .map(|requirement| Check::not_run(&format!("requirement/{}", requirement.name), why)),
        );
    };
    match &analysis.elaborated.operating {
        None => {
            checks.push(Check::not_run(
                "graph",
                "the design has no `operate` block, so there is no flow to solve",
            ));
            unanswered("the design has no `operate` block", &mut checks);
        }
        Some(operating) => match extract(chip, &analysis.nets, &Variation::nominal()).and_then(|network| {
            let solution = network.solve(operating)?;
            let mut solved = vec![solution.check()];
            solved.extend(requirements.iter().map(|requirement| evaluate(requirement, &solution)));
            Ok(solved)
        }) {
            Ok(solved) => checks.extend(solved),
            Err(error) => {
                checks.push(Check::not_run("graph", &error.to_string()));
                unanswered("the network could not be solved", &mut checks);
            }
        },
    }

    // Across the chips the process makes, when it says how they vary.
    let mut fared = BTreeMap::new();
    if let Some(process) = process.filter(|process| process.tolerances.is_some() && !requirements.is_empty()) {
        let found = across(&analysis, process);
        checks.push(corners_of(&analysis, &found));
        fared = found.unwrap_or_default();
        // Yield is found when the design asks for one; the corners speak for every design.
        if analysis
            .elaborated
            .requirements
            .iter()
            .any(|requirement| requirement.measure == Measure::Yield)
        {
            checks.push(yield_(&analysis, process, YIELD_CHIPS, YIELD_SEED));
        }
    }

    let mut inputs: BTreeMap<String, String> = analysis.elaborated.sources.clone();
    if let Some(process) = process {
        inputs.insert(format!("process {}", process.name), process.sha256.clone());
    }
    if !overrides.0.is_empty() {
        inputs.insert(
            "tuned values".to_owned(),
            Sha256Hex::of_bytes(overrides.toml().as_bytes()).as_str().to_owned(),
        );
    }
    if let Ok(hash) = Sha256Hex::of_json(&analysis.elaborated) {
        inputs.insert("elaborated design".to_owned(), hash.as_str().to_owned());
    }
    let report = Report::of(analysis.elaborated.name.clone(), inputs, checks);
    Ok(Verified {
        report,
        analysis,
        masks,
        across: fared,
    })
}

impl Report {
    /// The report of `checks`, with whether they pass and how many problems they hold.
    pub fn of(design: String, inputs: BTreeMap<String, String>, checks: Vec<Check>) -> Self {
        let mut report = Self {
            design,
            inputs,
            checks,
            passes: false,
            problems: 0,
        };
        report.judge();
        report
    }

    /// Find again whether the checks pass and how many problems they hold, after they change.
    pub fn judge(&mut self) {
        self.passes = self.checks.iter().all(Check::passes);
        self.problems = (self.checks.iter())
            .map(|check| match check.status {
                Status::NotRun => 1,
                _ => check.findings.iter().filter(|finding| finding.waived.is_none()).count(),
            })
            .sum();
    }

    pub fn json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|error| format!("{{\"error\": \"{error}\"}}"))
    }

    /// The report for a person to read.
    pub fn markdown(&self) -> String {
        let word = |status: Status| match status {
            Status::Satisfied => "satisfied",
            Status::Violated => "VIOLATED",
            Status::Waived => "waived",
            Status::NotRun => "NOT RUN",
        };
        let verdict = match (self.passes, self.problems) {
            (true, _) => "Passes: every check is satisfied or waived.".to_owned(),
            (false, 1) => "Does not pass: 1 problem.".to_owned(),
            (false, problems) => format!("Does not pass: {problems} problems."),
        };
        let mut text = format!(
            "# Verification report: {}\n\n**{verdict}**\n\n| Check | Result | Findings |\n| - | - | - |\n",
            self.design
        );
        for check in &self.checks {
            text.push_str(&format!(
                "| `{}` | {} | {} |\n",
                check.id,
                word(check.status),
                check.findings.len()
            ));
        }
        for check in &self.checks {
            text.push_str(&format!("\n## `{}`: {}\n\n", check.id, word(check.status)));
            if let Some(note) = &check.note {
                text.push_str(&format!("{note}\n\n"));
            }
            for finding in &check.findings {
                let waived = finding
                    .waived
                    .as_ref()
                    .map_or(String::new(), |reason| format!(" **Waived:** {reason}"));
                text.push_str(&format!(
                    "- **{}** ({}): {}{waived}\n",
                    finding.rule, finding.subject, finding.message
                ));
            }
            if !check.metrics.is_empty() {
                text.push_str("\n| Measured | Value |\n| - | - |\n");
                for (name, value) in &check.metrics {
                    text.push_str(&format!("| {name} | {value:.6} |\n"));
                }
            }
        }
        text.push_str("\n## Inputs\n\n| Input | SHA-256 |\n| - | - |\n");
        for (name, hash) in &self.inputs {
            text.push_str(&format!("| {name} | `{hash}` |\n"));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use crate::verify::{Group, group};

    #[test]
    fn each_check_answers_one_question() {
        let asked = |ids: &[&str]| ids.iter().map(|id| group(id)).collect::<Vec<_>>();
        assert_eq!(
            asked(&["rules", "mask-rules", "print", "benchmark/regions"]),
            [Group::Fabrication; 4]
        );
        assert_eq!(
            asked(&["connectivity", "graph", "requirement/gentle"]),
            [Group::Function; 3]
        );
        assert_eq!(asked(&["corners", "yield"]), [Group::Variation; 2]);
        assert_eq!(
            asked(&["deterministic", "gds-readback", "dxf-readback"]),
            [Group::Integrity; 3]
        );
    }
}
