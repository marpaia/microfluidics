//! A chip: a slab with a layout cut into it.

use serde::Serialize;

use microfluidics_core::model::{EdgeSpec, LayerStack};

use crate::error::LayoutError;
use crate::geometry::{Point, Rect, TOLERANCE, bounds};
use crate::plan::Layout;
use crate::rectangle::Rectangle;
use crate::structure::Translates;

/// A rectangular slab with a layout cut into one face, bonded to glass. The chip occupies
/// `[0, size[0]] x [0, size[1]]` in plan and is `size[2]` thick.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chip {
    pub size: [f64; 3],
    /// The width, depth and thickness of the glass the slab is bonded to, at the glass's middle
    /// with their sides parallel.
    pub glass: [f64; 3],
    pub layers: LayerStack,
    pub layout: Layout,
    /// Every way fluid passes: each structure's edges, and each routed channel.
    pub edges: Vec<EdgeSpec>,
    /// The pairs of ports the design places together.
    pub joints: Vec<(String, String)>,
}

/// A rectangle of its own, or a whole row, with its bounds.
enum Item<'a> {
    Rectangle(&'a Rectangle),
    Translates(&'a Translates),
}

impl Chip {
    /// A chip holding `layout`, on `glass`: the slab fits on the glass, every feature has area,
    /// lies within the outline and leaves a roof, and no two features overlap. A feature may lie
    /// wholly within one of a taller layer, which covers it: the cast chip has the taller one
    /// there.
    pub fn new(size: [f64; 3], glass: [f64; 3], layers: LayerStack, layout: Layout) -> Result<Self, LayoutError> {
        if layout.feature_count() == 0 {
            return Err(LayoutError::Empty);
        }
        if size[0] > glass[0] + TOLERANCE || size[1] > glass[1] + TOLERANCE {
            let mm = |metres: f64| (metres * 1e6).round() / 1e3;
            return Err(LayoutError::OffGlass {
                slab_mm: [mm(size[0]), mm(size[1])],
                glass_mm: [mm(glass[0]), mm(glass[1])],
            });
        }
        if let Some(layer) = layers.iter().find(|layer| layer.height >= size[2] - TOLERANCE) {
            let (height_um, thickness_um) = (layer.height * 1e6, size[2] * 1e6);
            return Err(LayoutError::TooDeep {
                layer: layer.name.clone(),
                height_um,
                thickness_um,
            });
        }
        let chip = Self {
            size,
            glass,
            layers,
            layout,
            edges: Vec::new(),
            joints: Vec::new(),
        };
        chip.check_features()?;
        Ok(chip)
    }

    /// The rectangles an item stands for.
    fn rectangles(&self, item: &Item) -> Vec<Rectangle> {
        match item {
            Item::Rectangle(rectangle) => vec![(*rectangle).clone()],
            Item::Translates(row) => self
                .layout
                .structures
                .get(&row.structure)
                .map_or_else(Vec::new, |structure| row.rectangles(structure).collect()),
        }
    }

    fn check_features(&self) -> Result<(), LayoutError> {
        let outline = Rect {
            min: Point::new(0.0, 0.0),
            max: Point::new(self.size[0], self.size[1]),
        };
        // Arrays are checked as wholes, and opened only where their bounds meet something else:
        // a hundred thousand trenches in a row are one item here.
        let mut items: Vec<(Rect, Item)> = Vec::new();
        for rectangle in &self.layout.rectangles {
            items.push((rectangle.bounds(), Item::Rectangle(rectangle)));
        }
        for row in &self.layout.translates {
            let Some(structure) = self.layout.structures.get(&row.structure) else {
                continue;
            };
            let ends = [0, row.count.saturating_sub(1)].map(|at| {
                let origin = row.origin_of(at);
                structure
                    .rectangles
                    .iter()
                    .flat_map(|rectangle| rectangle.placed(row.quarter_turns, origin).polygon)
                    .collect::<Vec<Point>>()
            });
            items.push((bounds(ends.into_iter().flatten()), Item::Translates(row)));
            // Within an row, one placement must clear the next.
            if row.count > 1 {
                let placed = |at: u32| {
                    structure
                        .rectangles
                        .iter()
                        .map(move |rectangle| rectangle.placed(row.quarter_turns, row.origin_of(at)))
                };
                for first in placed(0) {
                    if placed(1).any(|second| first.bounds().interiors_overlap(&second.bounds())) {
                        let name = |at: i64| format!("{}[{at}]", row.name);
                        return Err(LayoutError::Overlap(name(row.first), name(row.first + 1)));
                    }
                }
            }
        }
        for (rect, item) in &items {
            let name = match item {
                Item::Rectangle(rectangle) => rectangle.name.clone(),
                Item::Translates(row) => row.name.clone(),
            };
            if rect.width() <= TOLERANCE || rect.height() <= TOLERANCE {
                return Err(LayoutError::Degenerate(name));
            }
            if !outline.contains(rect) {
                return Err(LayoutError::OutsideChip(name));
            }
        }
        items.sort_by(|a, b| a.0.min.x.total_cmp(&b.0.min.x));
        for (at, (rect, item)) in items.iter().enumerate() {
            for (other_rect, other) in &items[at + 1..] {
                if other_rect.min.x >= rect.max.x - TOLERANCE {
                    break;
                }
                if !rect.interiors_overlap(other_rect) {
                    continue;
                }
                let (own, others) = (self.rectangles(item), self.rectangles(other));
                // A feature wholly within one of a taller layer is covered by it.
                let covered = |a: &Rectangle, b: &Rectangle| {
                    let (a_high, b_high) = (self.layers.height(&a.layer), self.layers.height(&b.layer));
                    let (inner, outer) = if a_high < b_high { (a, b) } else { (b, a) };
                    (a_high - b_high).abs() > TOLERANCE && outer.bounds().contains(&inner.bounds())
                };
                let clash = |a: &Rectangle, b: &Rectangle| a.bounds().interiors_overlap(&b.bounds()) && !covered(a, b);
                for a in &own {
                    if let Some(b) = others.iter().find(|b| clash(a, b)) {
                        return Err(LayoutError::Overlap(a.name.clone(), b.name.clone()));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use microfluidics_core::model::{LayerSpec, LayerStack};

    use crate::chip::Chip;
    use crate::error::LayoutError;
    use crate::geometry::Point;
    use crate::plan::Layout;
    use crate::rectangle::Rectangle;

    fn chip(rectangles: &[(&str, &str, [f64; 4])]) -> Result<Chip, LayoutError> {
        let layer = |name: &str, height: f64, gds: u16| LayerSpec {
            name: name.to_owned(),
            height,
            gds,
        };
        let um = 1e-6;
        let layout = Layout {
            rectangles: (rectangles.iter())
                .map(|(name, layer, at)| {
                    let (min, max) = (Point::new(at[0] * um, at[1] * um), Point::new(at[2] * um, at[3] * um));
                    Rectangle::rect(name, layer, min, max)
                })
                .collect(),
            ..Layout::default()
        };
        let layers = LayerStack::new(vec![layer("trench", 1.1e-6, 1), layer("flow", 20e-6, 2)]).unwrap();
        Chip::new([2e-3, 2e-3, 3e-3], [5e-3, 5e-3, 1e-3], layers, layout)
    }

    #[test]
    fn features_may_not_overlap_unless_a_taller_one_covers_a_shallower() {
        let window = ("window", "flow", [100.0, 100.0, 300.0, 300.0]);
        // A shallow feature wholly within a taller one is covered by it.
        assert!(chip(&[window, ("cross", "trench", [150.0, 190.0, 250.0, 210.0])]).is_ok());
        let overlap = |result: Result<Chip, LayoutError>| matches!(result, Err(LayoutError::Overlap(_, _)));
        // One that reaches out of it is not, nor is a taller one within a shallower, nor two of
        // one layer.
        assert!(overlap(chip(&[
            window,
            ("cross", "trench", [50.0, 190.0, 250.0, 210.0])
        ])));
        assert!(overlap(chip(&[
            ("wide", "trench", [100.0, 100.0, 300.0, 300.0]),
            ("post", "flow", [150.0, 150.0, 250.0, 250.0])
        ])));
        assert!(overlap(chip(&[
            window,
            ("again", "flow", [150.0, 150.0, 250.0, 250.0])
        ])));
    }
}
