//! The result of a verification.
//!
//! Every stage that verifies something returns a [`Check`]. Nothing else decides whether a
//! design passes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Satisfied,
    Violated,
    /// Every finding was waived by a person, with a reason.
    Waived,
    /// The check did not run; its note says why.
    NotRun,
}

/// One thing a check found wrong.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
    /// The rule or requirement broken, by its name.
    pub rule: String,
    /// The feature, port or element it is about.
    pub subject: String,
    pub measured: Option<f64>,
    pub limit: Option<f64>,
    pub message: String,
    /// The reason a person gave for accepting it.
    pub waived: Option<String>,
}

/// A person's decision to accept one finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Waiver {
    pub rule: String,
    pub subject: String,
    /// The process whose finding is accepted; one that names none accepts it at every process.
    #[serde(default)]
    pub process: Option<String>,
    pub reason: String,
    /// Who decided.
    pub by: String,
}

impl Waiver {
    /// Whether the waiver speaks of findings made against `process`.
    pub fn applies(&self, process: Option<&str>) -> bool {
        self.process.is_none() || self.process.as_deref() == process
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Check {
    pub id: String,
    pub status: Status,
    pub findings: Vec<Finding>,
    /// What was measured, each named with its unit.
    pub metrics: BTreeMap<String, f64>,
    /// Why the check did not run, or a caveat on its result.
    pub note: Option<String>,
    /// The process the check was made against, when it depends on one.
    pub process: Option<String>,
}

impl Check {
    /// A check from what it found, after `waivers` are applied: violated if any finding is not
    /// waived, waived if all are, satisfied if there are none.
    pub fn from_findings(
        id: &str,
        mut findings: Vec<Finding>,
        metrics: BTreeMap<String, f64>,
        waivers: &[Waiver],
    ) -> Self {
        for finding in &mut findings {
            let waiver = waivers
                .iter()
                .find(|waiver| waiver.rule == finding.rule && waiver.subject == finding.subject);
            finding.waived = waiver.map(|waiver| format!("{} ({})", waiver.reason, waiver.by));
        }
        let status = match (
            findings.is_empty(),
            findings.iter().all(|finding| finding.waived.is_some()),
        ) {
            (true, _) => Status::Satisfied,
            (false, true) => Status::Waived,
            (false, false) => Status::Violated,
        };
        Self {
            id: id.to_owned(),
            status,
            findings,
            metrics,
            note: None,
            process: None,
        }
    }

    pub fn not_run(id: &str, why: &str) -> Self {
        Self {
            id: id.to_owned(),
            status: Status::NotRun,
            findings: Vec::new(),
            metrics: BTreeMap::new(),
            note: Some(why.to_owned()),
            process: None,
        }
    }

    /// The check as one made against the process `name`.
    pub fn for_process(mut self, name: Option<&str>) -> Self {
        self.process = name.map(str::to_owned);
        self
    }

    /// Whether the check stands in the way of a design passing.
    pub fn passes(&self) -> bool {
        matches!(self.status, Status::Satisfied | Status::Waived)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::check::{Check, Finding, Status, Waiver};

    fn finding(subject: &str) -> Finding {
        Finding {
            rule: "trench.space".into(),
            subject: subject.into(),
            measured: Some(1.3),
            limit: Some(2.0),
            message: "too close".into(),
            waived: None,
        }
    }

    #[test]
    fn a_waiver_changes_a_findings_status_and_never_removes_it() {
        let waiver = Waiver {
            rule: "trench.space".into(),
            subject: "a".into(),
            process: None,
            reason: "measured on the last run".into(),
            by: "a researcher".into(),
        };
        assert_eq!(
            Check::from_findings("rules", Vec::new(), BTreeMap::new(), &[]).status,
            Status::Satisfied
        );
        assert_eq!(
            Check::from_findings("rules", vec![finding("a")], BTreeMap::new(), &[]).status,
            Status::Violated
        );
        let waived = Check::from_findings(
            "rules",
            vec![finding("a")],
            BTreeMap::new(),
            std::slice::from_ref(&waiver),
        );
        assert_eq!((waived.status, waived.findings.len()), (Status::Waived, 1));
        assert_eq!(
            waived.findings[0].waived.as_deref(),
            Some("measured on the last run (a researcher)")
        );
        let partly = Check::from_findings("rules", vec![finding("a"), finding("b")], BTreeMap::new(), &[waiver]);
        assert!(partly.status == Status::Violated && !partly.passes() && waived.passes());
        assert_eq!(
            Check::not_run("print", "the process states no print model").status,
            Status::NotRun
        );
    }
}
