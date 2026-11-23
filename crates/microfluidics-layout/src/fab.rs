//! Who makes a process's chips, and how a design reaches them.
//!
//! A process may name the fab it describes: where its limits were read, which of them the fab
//! has not published, how the fab takes a design, and the steps by which a chip is made there.

use serde::{Deserialize, Serialize};

/// How a design reaches the fab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Handoff {
    /// The fab is sent files and makes the chip.
    Order,
    /// A person makes the chip at the fab, step by step.
    Traveler,
}

/// The mask file the fab works from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Drawing {
    /// A flat drawing for each layer.
    Dxf,
    /// Every layer in one GDSII file.
    Gds,
}

/// The fab a process describes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fab {
    pub name: String,
    pub handoff: Handoff,
    pub drawing: Drawing,
    /// Where the process's limits were read.
    pub source: String,
    /// The date they were read, as `YYYY-MM-DD`.
    pub read: String,
    /// What the fab has not published: rules by name, and `tolerances`, `masks` or `print`.
    #[serde(default)]
    pub assumed: Vec<String>,
    /// How the fab wants the tone of a mask stated.
    pub polarity: Option<String>,
    /// What to ask the fab before relying on what is assumed.
    #[serde(default)]
    pub questions: Vec<String>,
}

/// One step by which a chip is made at the fab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub title: String,
    /// The layer the step makes; a step that names none is of the whole chip, after its layers.
    pub layer: Option<String>,
    pub text: String,
}

/// The tables of a process file that a fab may leave unpublished.
pub(crate) const TABLES: [&str; 3] = ["tolerances", "masks", "print"];

impl Fab {
    /// What is assumed, in words, among `rules` limits: `None` when the fab published them all.
    pub fn caveat(&self, rules: usize) -> Option<String> {
        if self.assumed.is_empty() {
            return None;
        }
        let (tables, limits): (Vec<&str>, Vec<&str>) = (self.assumed.iter())
            .map(String::as_str)
            .partition(|name| TABLES.contains(name));
        let mut said = Vec::new();
        if !limits.is_empty() {
            said.push(format!(
                "{} of {rules} limits are assumed, not published by {}: {}",
                limits.len(),
                self.name,
                limits.join(", ")
            ));
        }
        if !tables.is_empty() {
            let named: Vec<&str> = (tables.iter())
                .map(|table| match *table {
                    "masks" => "mask preparation",
                    "print" => "print model",
                    other => other,
                })
                .collect();
            said.push(format!(
                "also assumed, not published by {}: its {}",
                self.name,
                named.join(" and ")
            ));
        }
        Some(said.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use crate::process::Process;

    const FAB: &str = "name = \"p\"\n[[rules]]\nname = \"w\"\nkind = \"width\"\nmin_um = 5.0\n[[rules]]\nname = \"s\"\nkind = \"space\"\nmin_um = 5.0\n[fab]\nname = \"A fab\"\nhandoff = \"order\"\ndrawing = \"dxf\"\nsource = \"https://example.org/rules\"\nread = \"2026-11-23\"\n";

    #[test]
    fn a_process_names_its_fab_and_what_the_fab_has_not_published() {
        let published = Process::parse("p.toml", FAB).unwrap();
        assert_eq!(published.fab.as_ref().unwrap().name, "A fab");
        assert_eq!(published.caveat(), None);
        assert_eq!(Process::default().caveat(), None);

        let assumed = Process::parse("p.toml", &format!("{FAB}assumed = [\"s\", \"print\"]\n")).unwrap();
        assert_eq!(
            assumed.caveat().unwrap(),
            "1 of 2 limits are assumed, not published by A fab: s; also assumed, not published by A fab: its print model"
        );
    }

    #[test]
    fn what_is_assumed_must_be_a_rule_or_a_table_of_the_process() {
        let unknown = Process::parse("p.toml", &format!("{FAB}assumed = [\"feature.widht\"]\n")).unwrap_err();
        assert!(unknown.message.contains("`feature.widht`"), "{unknown}");
        assert!(unknown.message.contains("w, s, tolerances, masks, print"), "{unknown}");
        let step = "[[steps]]\ntitle = \"Coat\"\nlayer = \"flow\"\ntext = \"Spin the resist.\"\n";
        let stepped = Process::parse("p.toml", &format!("{FAB}{step}")).unwrap();
        assert_eq!(stepped.steps[0].layer.as_deref(), Some("flow"));
    }
}
