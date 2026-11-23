//! Fabrication processes: content that says what a process can make.
//!
//! A process is a deck of rules, as a fab's design rule manual is. Each rule is of one of a
//! fixed set of kinds; which rules a process has, on which layers and with what limits, is the
//! file's to say.

use serde::Deserialize;

use crate::bundled::PROCESSES;
use crate::fab::{Fab, Step, TABLES};
use crate::maskspec::{MaskSpec, PrintModel, WrittenMask, WrittenPrint};

/// A process file that cannot be used.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{file}: {message}")]
pub struct ProcessError {
    pub file: String,
    pub message: String,
}

/// What a rule limits. Lengths are metres. A rule that names no layer applies to every layer.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleKind {
    /// The narrowest a feature may be.
    Width { layer: Option<String>, min: f64 },
    /// The closest two features of one layer may be without joining.
    Space { layer: Option<String>, min: f64 },
    /// How far a feature that opens into a deeper one must be drawn into it on the mask.
    Overlap {
        layer: Option<String>,
        into: Option<String>,
        min: f64,
    },
    /// The tallest a feature, or a wall between two features, may be relative to its width.
    Aspect { layer: Option<String>, max: f64 },
    /// The widest a roof may be relative to its height, less what a punched hole removes.
    RoofSpan {
        layer: Option<String>,
        max: f64,
        punch: f64,
    },
    /// The closest two access points may be.
    PortPitch { min: f64 },
    /// The closest an access point may be to the chip's edge.
    EdgeMargin { min: f64 },
    /// The largest chip, either way round.
    Extent { max: [f64; 2] },
    /// The heights a layer may have.
    LayerHeight { min: f64, max: f64 },
    /// The most layers a chip may have.
    LayerCount { max: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub name: String,
    pub kind: RuleKind,
}

/// A rule as a process file writes it, in the units a person reads.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Written {
    Width {
        name: String,
        layer: Option<String>,
        min_um: f64,
    },
    Space {
        name: String,
        layer: Option<String>,
        min_um: f64,
    },
    Overlap {
        name: String,
        layer: Option<String>,
        into: Option<String>,
        min_um: f64,
    },
    Aspect {
        name: String,
        layer: Option<String>,
        max: f64,
    },
    RoofSpan {
        name: String,
        layer: Option<String>,
        max: f64,
        punch_um: Option<f64>,
    },
    PortPitch {
        name: String,
        min_um: f64,
    },
    EdgeMargin {
        name: String,
        min_um: f64,
    },
    Extent {
        name: String,
        max_mm: [f64; 2],
    },
    LayerHeight {
        name: String,
        min_um: f64,
        max_um: f64,
    },
    LayerCount {
        name: String,
        max: usize,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    rules: Vec<Written>,
    tolerances: Option<WrittenTolerances>,
    #[serde(default)]
    masks: Vec<WrittenMask>,
    print: Option<WrittenPrint>,
    fab: Option<Fab>,
    #[serde(default)]
    steps: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenTolerances {
    width_sigma_um: f64,
    width_bias_sigma_um: f64,
    height_sigma_fraction: f64,
}

/// How far a made chip may be from its drawing, each as one standard deviation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerances {
    /// The width of each feature on its own, in metres.
    pub width_sigma: f64,
    /// The width of every feature of a layer together, in metres.
    pub width_bias_sigma: f64,
    /// The height of a layer, as a fraction of it.
    pub height_fraction_sigma: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Process {
    pub name: String,
    pub description: String,
    pub rules: Vec<Rule>,
    /// The hash of the file the process was read from, in hex.
    pub sha256: String,
    /// How far a made chip may be from its drawing, when the process says.
    pub tolerances: Option<Tolerances>,
    /// How each layer's mask is made from its drawing; a layer not named is drawn as it is.
    pub masks: Vec<MaskSpec>,
    /// How masks print, when the process says.
    pub print: Option<PrintModel>,
    /// The fab the process describes, when it names one.
    pub fab: Option<Fab>,
    /// The steps by which a chip is made, in order.
    pub steps: Vec<Step>,
}

impl Process {
    /// Read a process from the text of the file named `file`.
    pub fn parse(file: &str, text: &str) -> Result<Self, ProcessError> {
        let fault = |message: String| ProcessError {
            file: file.to_owned(),
            message,
        };
        let written: File = toml::from_str(text).map_err(|error| fault(error.to_string()))?;
        let um = 1e-6;
        let mut rules: Vec<Rule> = Vec::new();
        for rule in written.rules {
            let (name, kind) = match rule {
                Written::Width { name, layer, min_um } => (
                    name,
                    RuleKind::Width {
                        layer,
                        min: min_um * um,
                    },
                ),
                Written::Space { name, layer, min_um } => (
                    name,
                    RuleKind::Space {
                        layer,
                        min: min_um * um,
                    },
                ),
                Written::Overlap {
                    name,
                    layer,
                    into,
                    min_um,
                } => (
                    name,
                    RuleKind::Overlap {
                        layer,
                        into,
                        min: min_um * um,
                    },
                ),
                Written::Aspect { name, layer, max } => (name, RuleKind::Aspect { layer, max }),
                Written::RoofSpan {
                    name,
                    layer,
                    max,
                    punch_um,
                } => (
                    name,
                    RuleKind::RoofSpan {
                        layer,
                        max,
                        punch: punch_um.unwrap_or(0.0) * um,
                    },
                ),
                Written::PortPitch { name, min_um } => (name, RuleKind::PortPitch { min: min_um * um }),
                Written::EdgeMargin { name, min_um } => (name, RuleKind::EdgeMargin { min: min_um * um }),
                Written::Extent { name, max_mm } => (
                    name,
                    RuleKind::Extent {
                        max: [max_mm[0] * 1e-3, max_mm[1] * 1e-3],
                    },
                ),
                Written::LayerHeight { name, min_um, max_um } => (
                    name,
                    RuleKind::LayerHeight {
                        min: min_um * um,
                        max: max_um * um,
                    },
                ),
                Written::LayerCount { name, max } => (name, RuleKind::LayerCount { max }),
            };
            if rules.iter().any(|held| held.name == name) {
                return Err(fault(format!("the rule `{name}` is stated twice")));
            }
            rules.push(Rule { name, kind });
        }
        let sha256 = microfluidics_core::Sha256Hex::of_bytes(text.as_bytes())
            .as_str()
            .to_owned();
        let tolerances = written.tolerances.map(|given| Tolerances {
            width_sigma: given.width_sigma_um * um,
            width_bias_sigma: given.width_bias_sigma_um * um,
            height_fraction_sigma: given.height_sigma_fraction,
        });
        let print = written
            .print
            .as_ref()
            .map(WrittenPrint::model)
            .transpose()
            .map_err(fault)?;
        let mut masks: Vec<MaskSpec> = Vec::new();
        for mask in written.masks.into_iter().map(MaskSpec::from) {
            if masks.iter().any(|held| held.layer == mask.layer) {
                return Err(fault(format!("the mask of layer `{}` is stated twice", mask.layer)));
            }
            masks.push(mask);
        }
        if let Some(fab) = &written.fab {
            let known = |name: &String| rules.iter().any(|rule| &rule.name == name) || TABLES.contains(&name.as_str());
            if let Some(unknown) = fab.assumed.iter().find(|name| !known(name)) {
                let names: Vec<&str> = (rules.iter().map(|rule| rule.name.as_str())).chain(TABLES).collect();
                return Err(fault(format!(
                    "`{unknown}` is said to be assumed, but is no rule or table of the process; those are {}",
                    names.join(", ")
                )));
            }
        }
        Ok(Self {
            name: written.name,
            description: written.description,
            rules,
            sha256,
            tolerances,
            masks,
            print,
            fab: written.fab,
            steps: written.steps,
        })
    }

    /// The names of the processes built into this crate.
    pub fn bundled_names() -> Vec<String> {
        PROCESSES
            .iter()
            .map(|(file, _)| file.trim_end_matches(".toml").to_owned())
            .collect()
    }

    /// The text of the file a built-in process is read from.
    pub fn bundled_text(name: &str) -> Option<&'static str> {
        let file = format!("{name}.toml");
        PROCESSES.iter().find(|(held, _)| *held == file).map(|(_, text)| *text)
    }

    /// The process named `name` among those built into this crate.
    pub fn bundled(name: &str) -> Result<Self, ProcessError> {
        let file = format!("{name}.toml");
        let found = PROCESSES.iter().find(|(held, _)| *held == file);
        let (file, text) = found.ok_or_else(|| {
            let message = format!(
                "there is no process `{name}`; those built in are {}",
                Self::bundled_names().join(", ")
            );
            ProcessError {
                file: file.clone(),
                message,
            }
        })?;
        Self::parse(file, text)
    }

    /// What of the process its fab has not published, in words; `None` when it names no fab, or
    /// the fab published everything.
    pub fn caveat(&self) -> Option<String> {
        self.fab.as_ref().and_then(|fab| fab.caveat(self.rules.len()))
    }

    /// The diameter of the hole punched at each access point, when a roof rule states one.
    pub fn punch(&self) -> Option<f64> {
        self.rules.iter().find_map(|rule| match rule.kind {
            RuleKind::RoofSpan { punch, .. } if punch > 0.0 => Some(punch),
            _ => None,
        })
    }

    /// How the mask of `layer` is made from its drawing.
    pub fn mask(&self, layer: &str) -> MaskSpec {
        let stated = self.masks.iter().find(|mask| mask.layer == layer);
        stated.cloned().unwrap_or_else(|| MaskSpec::plain(layer))
    }

    /// How far a feature on `layer` that opens into one on `into` is drawn into it: the largest
    /// distance any overlap rule asks for the pair, or zero when none does.
    pub fn overlap(&self, layer: &str, into: &str) -> f64 {
        let applies = |held: &Option<String>, name: &str| held.as_deref().is_none_or(|held| held == name);
        let asked = self.rules.iter().filter_map(|rule| match &rule.kind {
            RuleKind::Overlap {
                layer: from,
                into: to,
                min,
            } if applies(from, layer) && applies(to, into) => Some(*min),
            _ => None,
        });
        asked.fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use crate::process::{Process, RuleKind};

    #[test]
    fn every_built_in_process_is_named_as_its_file() {
        assert!(!Process::bundled_names().is_empty());
        for name in Process::bundled_names() {
            assert_eq!(Process::bundled(&name).unwrap().name, name);
        }
        assert!(
            Process::bundled("absent")
                .unwrap_err()
                .to_string()
                .contains("those built in are")
        );
    }

    #[test]
    fn a_deck_is_read_into_rules_in_metres() {
        let text = "name = \"p\"\n[[rules]]\nname = \"t.width\"\nkind = \"width\"\nlayer = \"trench\"\nmin_um = 1.0\n[[rules]]\nname = \"into\"\nkind = \"overlap\"\nmin_um = 3.0\n";
        let process = Process::parse("p.toml", text).unwrap();
        assert_eq!(
            process.rules[0].kind,
            RuleKind::Width {
                layer: Some("trench".into()),
                min: 1e-6
            }
        );
        assert!((process.overlap("trench", "flow") - 3e-6).abs() < 1e-15);
        assert_eq!(Process::default().overlap("trench", "flow"), 0.0);
    }

    #[test]
    fn a_deck_with_an_unknown_kind_or_a_repeated_name_is_refused() {
        let unknown = Process::parse(
            "p.toml",
            "name = \"p\"\n[[rules]]\nname = \"a\"\nkind = \"density\"\nmin_um = 1.0\n",
        )
        .unwrap_err()
        .to_string();
        assert!(
            unknown.starts_with("p.toml") && unknown.contains("density"),
            "{unknown}"
        );
        let twice = "name = \"p\"\n[[rules]]\nname = \"a\"\nkind = \"width\"\nmin_um = 1.0\n[[rules]]\nname = \"a\"\nkind = \"space\"\nmin_um = 1.0\n";
        assert!(
            Process::parse("p.toml", twice)
                .unwrap_err()
                .message
                .contains("stated twice")
        );
        assert!(
            Process::parse(
                "p.toml",
                "name = \"p\"\n[[rules]]\nname = \"a\"\nkind = \"width\"\nmin = 1.0\n"
            )
            .is_err()
        );
    }
}
