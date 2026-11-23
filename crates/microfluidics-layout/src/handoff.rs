//! The sheet that goes with a chip's masks to whoever makes it.
//!
//! A fab that is sent files gets an order: what is sent, what is asked for, and what to settle
//! first. A chip made by hand gets a traveler: each layer's steps in the order they are done,
//! with room to write down what was measured. This is the only place a fab's way of taking a
//! design is interpreted.

use std::collections::BTreeMap;

use microfluidics_core::Check;

use crate::chip::Chip;
use crate::fab::{Drawing, Fab, Handoff};
use crate::format::{Dxf, Format, Gds};
use crate::mask::{Mask, MaskSet};
use crate::maskspec::Tone;
use crate::process::Process;

/// A hand-off sheet: the name of its file, and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub file: &'static str,
    pub text: String,
}

const UM: f64 = 1e6;
const MM: f64 = 1e3;

/// How many of a layer's narrowest widths a traveler asks to have measured.
const MEASURED: usize = 5;

/// The mask files the fab works from, as the masks' export names them.
fn drawings(fab: &Fab, masks: &MaskSet) -> Vec<(String, String)> {
    match fab.drawing {
        Drawing::Dxf => (masks.masks.iter())
            .map(|mask| (Dxf.file_name(Some(&mask.layer)), mask.layer.name.clone()))
            .collect(),
        Drawing::Gds => vec![(Gds.file_name(None), "every layer".to_owned())],
    }
}

/// What the fab has not published and what to ask it, set out before anything else.
fn unsettled(process: &Process, fab: &Fab) -> String {
    let mut text = String::new();
    if let Some(caveat) = process.caveat() {
        text.push_str(&format!("**Not confirmed by {}.** {caveat}.\n\n", fab.name));
    }
    if !fab.questions.is_empty() {
        text.push_str(&format!("Ask {} before relying on this sheet:\n\n", fab.name));
        for question in &fab.questions {
            text.push_str(&format!("- {question}\n"));
        }
        text.push('\n');
    }
    text
}

/// The narrowest feature of a mask, and its `MEASURED` narrowest distinct widths each with one
/// feature that has it; widths in metres.
fn narrowest(masks: &MaskSet, mask: &Mask) -> Vec<(f64, String)> {
    let mut widths: BTreeMap<i64, String> = BTreeMap::new();
    for feature in masks.flattened(mask) {
        let bounds = feature.bounds();
        let width = bounds.width().min(bounds.height());
        widths.entry((width * 1e9).round() as i64).or_insert(feature.name);
    }
    (widths.into_iter().take(MEASURED))
        .map(|(nm, name)| (nm as f64 * 1e-9, name))
        .collect()
}

fn files_table(fab: &Fab, masks: &MaskSet, files: &[(String, String)]) -> String {
    let mut text = "| File | Holds | SHA-256 |\n| - | - | - |\n".to_owned();
    for (file, holds) in drawings(fab, masks) {
        let hash = files.iter().find(|(name, _)| name.ends_with(&file));
        let (path, hash) = hash.map_or((file.as_str(), "not written"), |(path, hash)| (path, hash));
        text.push_str(&format!("| `{path}` | {holds} | `{hash}` |\n"));
    }
    text
}

fn layers_table(masks: &MaskSet) -> String {
    let mut text = "| Exposed | Layer | GDSII number | Height, um | Narrowest feature, um | Inside the outlines | Mirrored | Bias, um |\n| - | - | - | - | - | - | - | - |\n".to_owned();
    for (order, mask) in masks.masks.iter().enumerate() {
        let least = narrowest(masks, mask).first().map_or(0.0, |(width, _)| *width);
        let tone = match mask.spec.tone {
            Tone::Clear => "clear",
            Tone::Dark => "dark",
        };
        text.push_str(&format!(
            "| {} | {} | {} | {:.2} | {:.3} | {tone} | {} | {:+.2} |\n",
            order + 1,
            mask.layer.name,
            mask.layer.gds,
            mask.layer.height * UM,
            least * UM,
            if mask.spec.mirror { "yes" } else { "no" },
            mask.spec.bias * UM
        ));
    }
    text
}

/// Each limit of the process with what the chip measures against it, and who the limit is from.
fn limits_table(process: &Process, fab: &Fab, rules: &Check) -> String {
    let mut text = "| Limit | Measured on this chip | From |\n| - | - | - |\n".to_owned();
    for rule in &process.rules {
        let measured = rules.metrics.get(&rule.name);
        let basis = if fab.assumed.contains(&rule.name) {
            "assumed"
        } else {
            "published"
        };
        text.push_str(&format!(
            "| `{}` | {} | {basis} |\n",
            rule.name,
            measured.map_or("not measured".to_owned(), |value| format!("{value:.3}"))
        ));
    }
    let broken = rules.findings.iter().filter(|finding| finding.waived.is_none()).count();
    text.push_str(&format!(
        "\n{}. `microfluidics process {}` prints each limit.\n",
        match broken {
            0 => "The chip is within every limit".to_owned(),
            1 => "**The chip breaks 1 limit; see `REPORT.md`**".to_owned(),
            count => format!("**The chip breaks {count} limits; see `REPORT.md`**"),
        },
        process.name
    ));
    text
}

fn holes(chip: &Chip, process: &Process) -> String {
    let punch = process
        .punch()
        .map_or("a punch of the size the tubing needs".to_owned(), |punch| {
            format!("a {:.2} mm punch", punch * MM)
        });
    let mut text = format!(
        "{} holes, each made with {punch}. Positions are from the chip's lower left corner, seen from the feature side of the master.\n\n| Hole | x, mm | y, mm |\n| - | - | - |\n",
        chip.layout.access.len()
    );
    for (at, point) in chip.layout.access.iter().enumerate() {
        text.push_str(&format!("| {} | {:.3} | {:.3} |\n", at + 1, point.x * MM, point.y * MM));
    }
    text
}

fn slab(chip: &Chip) -> String {
    format!(
        "- **Slab:** {:.1} x {:.1} mm, {:.1} mm thick.\n- **Glass:** {:.1} x {:.1} mm, {:.2} mm thick.\n",
        chip.size[0] * MM,
        chip.size[1] * MM,
        chip.size[2] * MM,
        chip.glass[0] * MM,
        chip.glass[1] * MM,
        chip.glass[2] * MM
    )
}

fn order(
    design: &str,
    chip: &Chip,
    masks: &MaskSet,
    process: &Process,
    fab: &Fab,
    rules: &Check,
    files: &str,
) -> String {
    let mut text = format!(
        "# Order: {design} at {}\n\nPrepared for the process `{}`, whose limits were read at {} on {}.\n\n{}",
        fab.name,
        process.name,
        fab.source,
        fab.read,
        unsettled(process, fab)
    );
    text.push_str(&format!("## Files to send\n\n{files}\nThe drawings are in micrometres, as closed outlines of no width, none overlapping, with the lower left corner of the chip at the origin.\n\n"));
    text.push_str(&format!("## Layers\n\n{}\n", layers_table(masks)));
    if let Some(polarity) = &fab.polarity {
        text.push_str(&format!("**Mask polarity:** {polarity} Each closed outline encloses resist that stays on the master and becomes a channel in the cast chip.\n\n"));
    }
    text.push_str(&format!(
        "## Against {}'s limits\n\n{}\n",
        fab.name,
        limits_table(process, fab, rules)
    ));
    text.push_str(&format!(
        "## Chip\n\n{}- **Quantity:** to be stated by whoever orders.\n\n## Holes\n\n{}",
        slab(chip),
        holes(chip, process)
    ));
    text
}

fn traveler(
    design: &str,
    chip: &Chip,
    masks: &MaskSet,
    process: &Process,
    fab: &Fab,
    rules: &Check,
    files: &str,
) -> String {
    let mut text = format!(
        "# Traveler: {design} at {}\n\nPrepared for the process `{}`, whose limits were read at {} on {}. Write what was done and measured into the blank columns as the chip is made.\n\n{}",
        fab.name,
        process.name,
        fab.source,
        fab.read,
        unsettled(process, fab)
    );
    text.push_str(&format!(
        "## Files\n\n{files}\n## Layers, in the order they are exposed\n\n{}\n",
        layers_table(masks)
    ));
    for (order, mask) in masks.masks.iter().enumerate() {
        let layer = &mask.layer;
        text.push_str(&format!(
            "### {}. `{}`, {:.2} um\n\n",
            order + 1,
            layer.name,
            layer.height * UM
        ));
        if order > 0 {
            text.push_str("Align to the first layer's marks before exposing; their centres are listed under **Alignment marks**.\n\n");
        }
        let steps: Vec<_> = (process.steps.iter())
            .filter(|step| step.layer.as_deref() == Some(layer.name.as_str()))
            .collect();
        if steps.is_empty() {
            text.push_str("The process states no steps for this layer.\n\n");
        } else {
            text.push_str("| Step | What to do | Done | Notes |\n| - | - | - | - |\n");
            for step in steps {
                text.push_str(&format!("| {} | {} | | |\n", step.title, step.text));
            }
            text.push('\n');
        }
        text.push_str(&format!(
            "| Measure on the master | Drawn, um | Measured, um |\n| - | - | - |\n| Height of `{}` | {:.2} | |\n",
            layer.name,
            layer.height * UM
        ));
        for (width, name) in narrowest(masks, mask) {
            text.push_str(&format!("| Width of `{name}` | {:.3} | |\n", width * UM));
        }
        text.push('\n');
    }
    text.push_str("## Alignment marks\n\n");
    if chip.layout.fiducials.is_empty() {
        text.push_str("The chip draws no alignment marks.\n\n");
    } else {
        text.push_str("Each mark is a cross on the first layer, seen through a window on every later one. Positions are from the chip's lower left corner.\n\n| Mark | x, mm | y, mm |\n| - | - | - |\n");
        for fiducial in &chip.layout.fiducials {
            text.push_str(&format!(
                "| `{}` | {:.4} | {:.4} |\n",
                fiducial.name,
                fiducial.position.x * MM,
                fiducial.position.y * MM
            ));
        }
        text.push('\n');
    }
    text.push_str("## After the master\n\n");
    let after: Vec<_> = process.steps.iter().filter(|step| step.layer.is_none()).collect();
    if !after.is_empty() {
        text.push_str("| Step | What to do | Done | Notes |\n| - | - | - | - |\n");
        for step in after {
            text.push_str(&format!("| {} | {} | | |\n", step.title, step.text));
        }
        text.push('\n');
    }
    text.push_str(&format!("{}\n### Holes\n\n{}\n", slab(chip), holes(chip, process)));
    text.push_str(&format!(
        "## Against {}'s limits\n\n{}",
        fab.name,
        limits_table(process, fab, rules)
    ));
    text
}

/// The sheet that goes with `masks` to the fab `process` names; `None` when it names none.
/// `rules` is the check of the prepared masks against the process, and `files` each file written
/// for the process with its SHA-256.
pub fn sheet(
    design: &str,
    chip: &Chip,
    masks: &MaskSet,
    process: &Process,
    rules: &Check,
    files: &[(String, String)],
) -> Option<Sheet> {
    let fab = process.fab.as_ref()?;
    let listed = files_table(fab, masks, files);
    Some(match fab.handoff {
        Handoff::Order => Sheet {
            file: "ORDER.md",
            text: order(design, chip, masks, process, fab, rules, &listed),
        },
        Handoff::Traveler => Sheet {
            file: "TRAVELER.md",
            text: traveler(design, chip, masks, process, fab, rules, &listed),
        },
    })
}

#[cfg(test)]
mod tests {
    use crate::fixtures::chip;
    use crate::geometry::Point;
    use crate::handoff::sheet;
    use crate::mask::prepare;
    use crate::plan::Fiducial;
    use crate::process::Process;
    use crate::rules::{Features, check};

    const DECK: &str = "name = \"p\"\n[[rules]]\nname = \"w\"\nkind = \"width\"\nmin_um = 5.0\n[[rules]]\nname = \"roof\"\nkind = \"roof_span\"\nmax = 60.0\npunch_um = 750.0\n";
    const FAB: &str = "[fab]\nname = \"A fab\"\nsource = \"https://example.org/rules\"\nread = \"2026-11-23\"\nassumed = [\"roof\"]\nquestions = [\"How wide may a roof be?\"]\n";

    fn made(more: &str) -> Option<crate::handoff::Sheet> {
        let rectangles = [
            ("channel", "flow", [950.0, 500.0, 1050.0, 1500.0]),
            ("pad", "flow", [500.0, 100.0, 1500.0, 500.0]),
            ("trench", "trench", [925.0, 1000.0, 950.0, 1001.2]),
        ];
        let mut chip = chip([2.0, 2.0], &rectangles, &[[1000.0, 300.0]], &[]);
        chip.layout.fiducials.push(Fiducial {
            name: "mark.fiducial".into(),
            position: Point::new(200e-6, 1800e-6),
        });
        let process = Process::parse("p.toml", &format!("{DECK}{more}")).unwrap();
        let masks = prepare(&chip, Some(&process));
        let rules = check(&Features::prepared(&masks, &chip), &process.rules, &[]);
        let files = [
            ("masks/chip.gds".to_owned(), "aa11".to_owned()),
            ("masks/mask_1_trench.dxf".to_owned(), "bb22".to_owned()),
            ("masks/mask_2_flow.dxf".to_owned(), "cc33".to_owned()),
        ];
        sheet("a_design", &chip, &masks, &process, &rules, &files)
    }

    #[test]
    fn a_fab_that_is_sent_files_gets_an_order_naming_each_drawing_and_each_assumed_limit() {
        assert_eq!(made(""), None);
        let order = made(&format!(
            "{FAB}handoff = \"order\"\ndrawing = \"dxf\"\npolarity = \"Channels are transparent.\"\n"
        ))
        .unwrap();
        assert_eq!(order.file, "ORDER.md");
        for said in [
            "# Order: a_design at A fab",
            "1 of 2 limits are assumed, not published by A fab: roof",
            "- How wide may a roof be?",
            "| `masks/mask_1_trench.dxf` | trench | `bb22` |",
            "| `masks/mask_2_flow.dxf` | flow | `cc33` |",
            "**Mask polarity:** Channels are transparent.",
            "| `w` | 1.200 | published |",
            "**The chip breaks 1 limit; see `REPORT.md`**",
            "1 holes, each made with a 0.75 mm punch",
            "| 1 | 1.000 | 0.300 |",
            "- **Slab:** 2.0 x 2.0 mm, 3.0 mm thick.",
        ] {
            assert!(order.text.contains(said), "{said}\n{}", order.text);
        }
        assert!(!order.text.contains("chip.gds"));
    }

    #[test]
    fn a_chip_made_by_hand_gets_a_traveler_of_each_layers_steps_with_room_to_write() {
        let steps = "[[steps]]\ntitle = \"Coat\"\nlayer = \"flow\"\ntext = \"Spin the resist.\"\n[[steps]]\ntitle = \"Cast\"\ntext = \"Pour and cure.\"\n";
        let traveler = made(&format!("{FAB}handoff = \"traveler\"\ndrawing = \"gds\"\n{steps}")).unwrap();
        assert_eq!(traveler.file, "TRAVELER.md");
        for said in [
            "# Traveler: a_design at A fab",
            "| `masks/chip.gds` | every layer | `aa11` |",
            "### 1. `trench`, 1.10 um\n\nThe process states no steps for this layer.",
            "### 2. `flow`, 20.00 um\n\nAlign to the first layer's marks",
            "| Coat | Spin the resist. | | |",
            "| Height of `flow` | 20.00 | |",
            "| Width of `trench` | 1.200 | |",
            "| `mark.fiducial` | 0.2000 | 1.8000 |",
            "| Cast | Pour and cure. | | |",
            "| `roof` | 5.000 | assumed |",
        ] {
            assert!(traveler.text.contains(said), "{said}\n{}", traveler.text);
        }
        assert!(!traveler.text.contains(".dxf"));
    }
}
