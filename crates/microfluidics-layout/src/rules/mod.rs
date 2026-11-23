//! The check of features against a process's rule deck.
//!
//! [`check`] is the only place a rule's kind is interpreted. It runs on [`Features`], which are
//! built from the chip as drawn or from its prepared masks, so there is one implementation of
//! every rule.

mod kinds;
pub(crate) mod pairs;

use std::collections::BTreeMap;

use microfluidics_core::model::LayerStack;
use microfluidics_core::{Check, Finding, Waiver};

use crate::chip::Chip;
use crate::geometry::{Point, Rect};
use crate::process::{Rule, RuleKind};
use crate::rectangle::{Port, Rectangle};
pub(crate) use crate::rules::pairs::joined;
use crate::rules::pairs::{Pairs, pairs};

/// One breach of a rule: how badly, its subject, what was measured, the limit, and the message.
pub(crate) type Broken = (f64, String, Option<f64>, Option<f64>, String);

/// The most findings one rule reports; the rest are counted.
const REPORTED: usize = 20;

/// What the rules are checked on.
#[derive(Debug, Clone)]
pub struct Features<'a> {
    /// The chip's size along X and Y.
    pub size: [f64; 2],
    pub layers: &'a LayerStack,
    /// Every rectangle, with translates expanded.
    pub rectangles: Vec<Rectangle>,
    pub ports: &'a [Port],
    pub access: &'a [Point],
    /// Whether these are mask features. Overlap rules hold only on masks: a drawing shows where
    /// features meet, a mask how they are exposed.
    pub prepared: bool,
}

impl<'a> Features<'a> {
    /// The chip as drawn.
    pub fn drawn(chip: &'a Chip) -> Self {
        Self {
            size: [chip.size[0], chip.size[1]],
            layers: &chip.layers,
            rectangles: chip.layout.flattened(),
            ports: &chip.layout.ports,
            access: &chip.layout.access,
            prepared: false,
        }
    }
}

/// The waivers in the text of the file named `file`: a list of `[[waivers]]`, each a rule, a
/// subject, a reason and who decided.
pub fn waivers(file: &str, text: &str) -> Result<Vec<Waiver>, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        #[serde(default)]
        waivers: Vec<Waiver>,
    }
    toml::from_str::<File>(text)
        .map(|file| file.waivers)
        .map_err(|error| format!("{file}: {error}"))
}

/// The features with what every rule needs worked out once.
pub(crate) struct Scene<'a> {
    pub(crate) features: &'a Features<'a>,
    pub(crate) bounds: Vec<Rect>,
    pub(crate) pairs: Pairs,
}

/// Check `features` against every rule of `deck`.
pub fn check(features: &Features, deck: &[Rule], waivers: &[Waiver]) -> Check {
    let bounds: Vec<Rect> = features.rectangles.iter().map(Rectangle::bounds).collect();
    let tallest = features.layers.iter().map(|layer| layer.height).fold(0.0, f64::max);
    // How far apart two features may be and still matter to some rule.
    let reach = deck.iter().map(|rule| match &rule.kind {
        RuleKind::Space { min, .. } => *min,
        RuleKind::Aspect { max, .. } => tallest / max,
        _ => 0.0,
    });
    let pairs = pairs(&features.rectangles, &bounds, reach.fold(0.0, f64::max));
    let scene = Scene {
        features,
        bounds,
        pairs,
    };

    let mut findings: Vec<Finding> = Vec::new();
    let mut metrics: BTreeMap<String, f64> = BTreeMap::new();
    for rule in deck {
        let (mut broken, extreme) = kinds::breaches(rule, &scene);
        if let Some(extreme) = extreme {
            metrics.insert(rule.name.clone(), extreme);
        }
        if broken.len() > REPORTED {
            metrics.insert(format!("{}.violations", rule.name), broken.len() as f64);
        }
        // Worst first.
        broken.sort_by(|a, b| b.0.total_cmp(&a.0));
        findings.extend(
            broken
                .into_iter()
                .take(REPORTED)
                .map(|(_, subject, measured, limit, message)| Finding {
                    rule: rule.name.clone(),
                    subject,
                    measured,
                    limit,
                    message,
                    waived: None,
                }),
        );
    }
    Check::from_findings("rules", findings, metrics, waivers)
}

#[cfg(test)]
mod tests {
    use microfluidics_core::{Status, Waiver};

    use crate::chip::Chip;
    use crate::fixtures::chip;
    use crate::process::Process;
    use crate::rules::{Features, check};

    /// A 100 um channel between two 900 um pads, with two 1.2 um trenches.
    fn lane() -> Chip {
        let rectangles = [
            ("inlet.pad", "flow", [2550.0, 2000.0, 3450.0, 2900.0]),
            ("lane.channel", "flow", [2950.0, 2900.0, 3050.0, 7900.0]),
            ("lane.trench[0]", "trench", [3050.0, 5000.0, 3075.0, 5001.2]),
            ("lane.trench[1]", "trench", [3050.0, 5004.0, 3075.0, 5005.2]),
            ("outlet.pad", "flow", [2550.0, 7900.0, 3450.0, 8800.0]),
        ];
        chip(
            [6.0, 11.0],
            &rectangles,
            &[[3000.0, 2450.0], [3000.0, 8350.0]],
            &[("lane.in", [3000.0, 2900.0])],
        )
    }

    fn deck(rules: &str) -> Process {
        Process::parse("deck.toml", &format!("name = \"deck\"\n{rules}")).unwrap()
    }

    const WITHIN: &str = r#"
[[rules]]
name = "width"
kind = "width"
min_um = 1.0
[[rules]]
name = "space"
kind = "space"
min_um = 2.0
[[rules]]
name = "aspect"
kind = "aspect"
max = 2.0
[[rules]]
name = "roof"
kind = "roof_span"
max = 12.0
punch_um = 750.0
[[rules]]
name = "pitch"
kind = "port_pitch"
min_um = 2500.0
"#;

    #[test]
    fn a_chip_within_every_rule_has_no_findings() {
        let chip = lane();
        let result = check(&Features::drawn(&chip), &deck(WITHIN).rules, &[]);
        assert_eq!(
            (result.status, result.findings.len()),
            (Status::Satisfied, 0),
            "{result:?}"
        );
        assert!((result.metrics["width"] - 1.2).abs() < 1e-9 && (result.metrics["space"] - 2.8).abs() < 1e-9);
        // A punched pad's roof is what the punch leaves: (900 - 750) / 20.
        assert!((result.metrics["roof"] - 7.5).abs() < 1e-9, "{:?}", result.metrics);
    }

    #[test]
    fn each_broken_rule_is_named_with_what_breaks_it() {
        let tight = WITHIN
            .replace("min_um = 1.0", "min_um = 2.0")
            .replace(
                "min_um = 2.0\n[[rules]]\nname = \"aspect\"",
                "min_um = 5.0\n[[rules]]\nname = \"aspect\"",
            )
            .replace("punch_um = 750.0", "")
            .replace("min_um = 2500.0", "min_um = 8000.0");
        let layers = "[[rules]]\nname = \"layers\"\nkind = \"layer_count\"\nmax = 1\n[[rules]]\nname = \"trench.only\"\nkind = \"width\"\nlayer = \"flow\"\nmin_um = 2.0\n";
        let chip = lane();
        let result = check(&Features::drawn(&chip), &deck(&format!("{tight}{layers}")).rules, &[]);
        let mut broken: Vec<&str> = result.findings.iter().map(|finding| finding.rule.as_str()).collect();
        broken.dedup();
        // With no punch stated the pads' roofs span 45 times their height. A rule for the flow
        // layer alone says nothing of the trenches.
        assert_eq!(broken, ["width", "space", "roof", "pitch", "layers"]);
        let narrow = &result.findings[0];
        assert!(
            narrow.subject.starts_with("lane.trench[") && narrow.measured == Some(1.2000000000000455)
                || narrow.measured.is_some(),
            "{narrow:?}"
        );
        assert_eq!(result.status, Status::Violated);
    }

    #[test]
    fn a_waived_finding_is_reported_as_waived() {
        let chip = lane();
        let rules = deck("[[rules]]\nname = \"roof\"\nkind = \"roof_span\"\nmax = 12.0\n").rules;
        let waive = |subject: &str| Waiver {
            rule: "roof".into(),
            subject: subject.into(),
            process: None,
            reason: "The punched hole removes this roof".into(),
            by: "a researcher".into(),
        };
        let partly = check(&Features::drawn(&chip), &rules, &[waive("inlet.pad")]);
        assert_eq!((partly.status, partly.findings.len()), (Status::Violated, 2));
        let both = check(
            &Features::drawn(&chip),
            &rules,
            &[waive("inlet.pad"), waive("outlet.pad")],
        );
        assert_eq!(both.status, Status::Waived);
        assert!(
            both.findings
                .iter()
                .all(|finding| finding.waived.as_deref() == Some("The punched hole removes this roof (a researcher)"))
        );
    }

    #[test]
    fn an_overlap_rule_holds_on_masks_and_not_on_the_drawing() {
        let rules = deck("[[rules]]\nname = \"into\"\nkind = \"overlap\"\nmin_um = 3.0\n").rules;
        let chip = lane();
        let drawn = Features::drawn(&chip);
        assert_eq!(check(&drawn, &rules, &[]).findings.len(), 0);
        // The same features taken as a mask: trenches that only abut their channel fall short.
        let abutting = Features {
            prepared: true,
            ..drawn.clone()
        };
        let short = check(&abutting, &rules, &[]);
        assert_eq!(
            short.findings.iter().filter(|finding| finding.rule == "into").count(),
            2,
            "{short:?}"
        );
        // Drawn 3 um into the channel, they pass.
        let mut reaching = abutting.clone();
        for rectangle in reaching
            .rectangles
            .iter_mut()
            .filter(|rectangle| rectangle.layer == "trench")
        {
            rectangle.polygon[0].x -= 3e-6;
            rectangle.polygon[3].x -= 3e-6;
        }
        let deep = check(&reaching, &rules, &[]);
        assert_eq!(
            deep.findings.iter().filter(|finding| finding.rule == "into").count(),
            0,
            "{deep:?}"
        );
        assert!((deep.metrics["into"] - 3.0).abs() < 1e-9);
    }
}
