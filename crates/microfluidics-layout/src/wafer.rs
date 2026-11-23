//! A wafer: chips stepped across it, each named, with marks to dice and align by.

use std::collections::BTreeMap;

use serde::Deserialize;

use microfluidics_core::model::{LayerSpec, LayerStack};

use crate::geometry::{Point, Rect};
use crate::label;
use crate::mask::{Mask, MaskSet};
use crate::rectangle::Rectangle;
use crate::structure::Structure;

/// The height of a chip's name, the length and width of a dicing tick and of an alignment
/// cross's arm, and how far a tick stands off a chip's corner; all in metres.
const LABEL_HEIGHT: f64 = 500e-6;
const TICK: (f64, f64) = (400e-6, 50e-6);
const CROSS: (f64, f64) = (1e-3, 50e-6);
const STAND_OFF: f64 = 100e-6;

/// A cross about `centre` as three rectangles named `name`: its upright, and the arm to either
/// side. `margin` is how much wider and longer it is than the wafer's cross on every side.
fn mark(name: &str, layer: &str, centre: Point, margin: f64) -> Vec<Rectangle> {
    let (half, thin) = (CROSS.0 / 2.0 + margin, CROSS.1 / 2.0 + margin);
    let arm = |part: &str, min: [f64; 2], max: [f64; 2]| {
        Rectangle::rect(
            &format!("{name}_{part}"),
            layer,
            Point::new(centre.x + min[0], centre.y + min[1]),
            Point::new(centre.x + max[0], centre.y + max[1]),
        )
    };
    vec![
        arm("upright", [-thin, -half], [thin, half]),
        arm("left", [-half, -thin], [-thin, thin]),
        arm("right", [thin, -thin], [half, thin]),
    ]
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum WaferError {
    #[error("{file}: {message}")]
    File { file: String, message: String },
    #[error("the wafer places `{0}`, but no masks were given for it")]
    Unknown(String),
    #[error("{0}")]
    Layers(String),
    #[error(
        "{label} does not fit: its corner is {reach:.1} mm from the wafer's centre, and features must stay within {within:.1} mm"
    )]
    DoesNotFit { label: String, reach: f64, within: f64 },
}

/// One design stepped in a grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The design's file, as the wafer file names it.
    pub design: String,
    pub columns: u32,
    pub rows: u32,
}

/// What a wafer file states. Lengths are metres.
#[derive(Debug, Clone, PartialEq)]
pub struct WaferSpec {
    pub name: String,
    /// The process the wafer is made in, among those its designs name; needed when they name
    /// more than one.
    pub process: Option<String>,
    pub diameter: f64,
    /// How far from the wafer's edge nothing is drawn.
    pub edge_exclusion: f64,
    /// The gap between chips, where the wafer is cut.
    pub street: f64,
    /// Blocks of chips, the first lowest on the wafer.
    pub blocks: Vec<Block>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Written {
    name: String,
    process: Option<String>,
    diameter_mm: f64,
    edge_exclusion_mm: f64,
    street_mm: f64,
    chips: Vec<WrittenBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenBlock {
    design: String,
    columns: u32,
    rows: u32,
}

impl WaferSpec {
    /// Read a wafer from the text of the file named `file`.
    pub fn parse(file: &str, text: &str) -> Result<Self, WaferError> {
        let fault = |message: String| WaferError::File {
            file: file.to_owned(),
            message,
        };
        let written: Written = toml::from_str(text).map_err(|error| fault(error.to_string()))?;
        if written.diameter_mm <= 0.0 || written.edge_exclusion_mm < 0.0 || written.street_mm < 0.0 {
            return Err(fault(
                "the diameter must be more than zero, and the edge exclusion and street not less".to_owned(),
            ));
        }
        if let Some(empty) = written.chips.iter().find(|block| block.columns == 0 || block.rows == 0) {
            return Err(fault(format!("`{}` is placed in no columns or no rows", empty.design)));
        }
        let block = |block: WrittenBlock| Block {
            design: block.design,
            columns: block.columns,
            rows: block.rows,
        };
        Ok(Self {
            name: written.name,
            process: written.process,
            diameter: written.diameter_mm * 1e-3,
            edge_exclusion: written.edge_exclusion_mm * 1e-3,
            street: written.street_mm * 1e-3,
            blocks: written.chips.into_iter().map(block).collect(),
        })
    }
}

/// One chip on the wafer.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub design: String,
    /// The name drawn beneath it: its design and its number on the wafer.
    pub label: String,
    /// Where the chip's lower left corner lies.
    pub origin: Point,
}

/// A wafer's masks, and where each chip lies on them.
#[derive(Debug, Clone, PartialEq)]
pub struct Wafer {
    pub masks: MaskSet,
    pub placements: Vec<Placed>,
    /// How many marks for aligning the wafer were drawn: those that do not lie on a chip.
    pub alignment_marks: usize,
}

/// The layers of every chip together, each with one height and number.
fn layers(chips: &[(&str, &MaskSet)]) -> Result<LayerStack, WaferError> {
    let mut held: Vec<LayerSpec> = Vec::new();
    for (design, set) in chips {
        for layer in set.layers.iter() {
            match held.iter().find(|other| other.name == layer.name) {
                Some(other) if other != layer => {
                    return Err(WaferError::Layers(format!(
                        "layer `{}` of `{design}` is {:.2} um high with GDSII number {}, and elsewhere on the wafer it is {:.2} um with {}",
                        layer.name,
                        layer.height * 1e6,
                        layer.gds,
                        other.height * 1e6,
                        other.gds
                    )));
                }
                Some(_) => {}
                None => held.push(layer.clone()),
            }
        }
    }
    LayerStack::new(held).map_err(WaferError::Layers)
}

/// Step every chip of `spec` across its wafer. `chips` holds the prepared masks of each design
/// the wafer names. Chips are named beneath in the deepest layer, with ticks at their corners to
/// dice by. Two marks, either side of the wafer, align it: a cross on the shallowest layer and
/// a window over it on every other.
pub fn assemble(spec: &WaferSpec, chips: &BTreeMap<String, MaskSet>) -> Result<Wafer, WaferError> {
    let mut blocks: Vec<(&Block, &MaskSet)> = Vec::new();
    for block in &spec.blocks {
        let set = chips
            .get(&block.design)
            .ok_or_else(|| WaferError::Unknown(block.design.clone()))?;
        blocks.push((block, set));
    }
    let named: Vec<(&str, &MaskSet)> = blocks
        .iter()
        .map(|(block, set)| (block.design.as_str(), *set))
        .collect();
    let layers = layers(&named)?;
    let mut masks: Vec<Mask> = Vec::new();
    for layer in layers.iter() {
        let spec = (named.iter()).find_map(|(_, set)| set.masks.iter().find(|mask| mask.layer.name == layer.name));
        masks.push(Mask {
            layer: layer.clone(),
            spec: spec.map_or_else(
                || crate::maskspec::MaskSpec::plain(&layer.name),
                |mask| mask.spec.clone(),
            ),
            rectangles: Vec::new(),
            translates: Vec::new(),
            extended: 0,
        });
    }
    let (Some(shallowest), Some(deepest)) = (layers.iter().next().cloned(), layers.iter().last().cloned()) else {
        return Err(WaferError::Layers("the wafer's chips have no layers".to_owned()));
    };

    // Blocks are stacked upward and centred on the wafer, each block centred on its own.
    let extent = |block: &Block, set: &MaskSet| {
        let across = f64::from(block.columns) * set.size[0] + f64::from(block.columns - 1) * spec.street;
        let up = f64::from(block.rows) * set.size[1] + f64::from(block.rows - 1) * spec.street;
        (across, up)
    };
    let total: f64 = blocks.iter().map(|(block, set)| extent(block, set).1).sum::<f64>()
        + spec.street * (blocks.len().max(1) - 1) as f64;
    let centre = spec.diameter / 2.0;
    let within = centre - spec.edge_exclusion;
    let mut structures: BTreeMap<String, Structure> = BTreeMap::new();
    let mut placements: Vec<Placed> = Vec::new();
    let mut outlines: Vec<Rect> = Vec::new();
    let mut bottom = centre - total / 2.0;
    for (block, set) in &blocks {
        let (across, up) = extent(block, set);
        // A chip is named after its design's file, without the directory or the extension.
        let file = block.design.rsplit('/').next().unwrap_or(&block.design);
        let title = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        for (row, column) in (0..block.rows).flat_map(|row| (0..block.columns).map(move |column| (row, column))) {
            let origin = Point::new(
                centre - across / 2.0 + f64::from(column) * (set.size[0] + spec.street),
                bottom + f64::from(row) * (set.size[1] + spec.street),
            );
            let label = format!("{title}-{}", placements.len() + 1);
            let outline = Rect {
                min: origin,
                max: Point::new(origin.x + set.size[0], origin.y + set.size[1]),
            };
            let reach = [outline.min.x, outline.max.x]
                .iter()
                .flat_map(|x| [outline.min.y, outline.max.y].map(|y| (x - centre).hypot(y - centre)))
                .fold(0.0, f64::max);
            if reach > within + 1e-12 {
                return Err(WaferError::DoesNotFit {
                    label,
                    reach: reach * 1e3,
                    within: within * 1e3,
                });
            }
            for mask in &set.masks {
                let Some(target) = masks.iter_mut().find(|held| held.layer.name == mask.layer.name) else {
                    continue;
                };
                target.extended += mask.extended;
                target
                    .rectangles
                    .extend(mask.rectangles.iter().map(|rectangle| Rectangle {
                        name: format!("{label}.{}", rectangle.name),
                        layer: rectangle.layer.clone(),
                        polygon: rectangle.polygon.iter().map(|point| point.moved(origin)).collect(),
                    }));
                for row in &mask.translates {
                    let Some(structure) = set.structures.get(&row.structure) else {
                        continue;
                    };
                    // Structures of one name are one only when they are alike.
                    let name = match structures.get(&structure.name) {
                        Some(held) if held != structure => format!("{title}_{}", structure.name),
                        _ => structure.name.clone(),
                    };
                    structures.entry(name.clone()).or_insert_with(|| Structure {
                        name: name.clone(),
                        rectangles: structure.rectangles.clone(),
                    });
                    let mut placed = row.clone();
                    (placed.name, placed.structure, placed.origin) =
                        (format!("{label}.{}", row.name), name, row.origin.moved(origin));
                    target.translates.push(placed);
                }
            }
            outlines.push(outline);
            placements.push(Placed {
                design: block.design.clone(),
                label,
                origin,
            });
        }
        bottom += up + spec.street;
    }

    // Beneath each chip its name; at each corner a tick either way, standing off in the street.
    let mut drawn: Vec<Rectangle> = Vec::new();
    for (placed, outline) in placements.iter().zip(&outlines) {
        let height = LABEL_HEIGHT.min(spec.street * 0.6).max(0.0);
        // A name too long for its chip is drawn smaller.
        let room = (outline.width() - 2.0 * (2.0 * STAND_OFF + TICK.0)).max(0.0);
        let height = height * (room / label::width(&placed.label, height).max(room)).min(1.0);
        if height > 0.0 {
            let at = Point::new(
                outline.min.x + STAND_OFF + TICK.0 + STAND_OFF,
                outline.min.y - (spec.street + height) / 2.0,
            );
            drawn.extend(label::text(
                &format!("{}.name", placed.label),
                &deepest.name,
                &placed.label,
                at,
                height,
            ));
        }
        if spec.street < 2.0 * STAND_OFF + TICK.1 {
            continue;
        }
        for (corner, (x, y)) in [
            (outline.min.x, outline.min.y),
            (outline.max.x, outline.min.y),
            (outline.min.x, outline.max.y),
            (outline.max.x, outline.max.y),
        ]
        .into_iter()
        .enumerate()
        {
            // Out from the chip either way: -1 at its lower or left side, +1 at its upper or right.
            let (out_x, out_y) = (
                if corner % 2 == 0 { -1.0 } else { 1.0 },
                if corner < 2 { -1.0 } else { 1.0 },
            );
            let span =
                |from: f64, out: f64, length: f64| (from.min(from + out * length), from.max(from + out * length));
            let (near_x, near_y) = (x + out_x * STAND_OFF, y + out_y * STAND_OFF);
            let ((ax0, ax1), (ay0, ay1)) = (span(x, -out_x, TICK.0), span(near_y, out_y, TICK.1));
            let ((bx0, bx1), (by0, by1)) = (span(near_x, out_x, TICK.1), span(y, -out_y, TICK.0));
            let tick = |way: &str, min: Point, max: Point| {
                Rectangle::rect(
                    &format!("{}.tick[{corner}].{way}", placed.label),
                    &deepest.name,
                    min,
                    max,
                )
            };
            drawn.push(tick("along", Point::new(ax0, ay0), Point::new(ax1, ay1)));
            drawn.push(tick("up", Point::new(bx0, by0), Point::new(bx1, by1)));
        }
    }
    if let Some(mask) = masks.iter_mut().find(|mask| mask.layer.name == deepest.name) {
        mask.rectangles.extend(drawn);
    }

    // A mark either side of the wafer on its horizontal centre line, where no chip lies: a
    // cross on the shallowest layer, which is exposed first, and on every other layer a window
    // of the same shape through which the cross is seen.
    let mut alignment_marks = 0;
    for (side, x) in [
        ("west", spec.edge_exclusion + CROSS.0),
        ("east", spec.diameter - spec.edge_exclusion - CROSS.0),
    ] {
        let clear = outlines.iter().all(|outline| {
            x + CROSS.0 < outline.min.x
                || x - CROSS.0 > outline.max.x
                || centre + CROSS.0 < outline.min.y
                || centre - CROSS.0 > outline.max.y
        });
        if !clear {
            continue;
        }
        alignment_marks += 1;
        for mask in &mut masks {
            let first = mask.layer.name == shallowest.name;
            // A window is wider than the cross by the cross's own width on every side.
            let margin = if first { 0.0 } else { CROSS.1 };
            let name = format!("wafer.{side}.{}", if first { "cross" } else { "window" });
            mask.rectangles
                .extend(mark(&name, &mask.layer.name, Point::new(x, centre), margin));
        }
    }
    let masks = MaskSet {
        size: [spec.diameter, spec.diameter],
        layers,
        structures,
        masks,
    };
    Ok(Wafer {
        masks,
        placements,
        alignment_marks,
    })
}
