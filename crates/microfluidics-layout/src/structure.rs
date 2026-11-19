//! Structures and translates: geometry stated once and placed many times.

use serde::Serialize;

use crate::geometry::Point;
use crate::rectangle::Rectangle;

/// Geometry in its own frame, referenced by translates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Structure {
    pub name: String,
    pub rectangles: Vec<Rectangle>,
}

/// `count` placements of a structure, each `step` from the last.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Translates {
    /// The name its elements carry: element `i` is `name[i]`.
    pub name: String,
    pub structure: String,
    /// Where the structure's origin goes in the first placement.
    pub origin: Point,
    pub step: Point,
    pub count: u32,
    /// How far the structure is turned counterclockwise, in whole quarter turns.
    pub quarter_turns: u8,
    /// The index of the first element.
    pub first: i64,
}

impl Translates {
    /// Where the structure's origin goes in placement `at`, counted from zero.
    pub fn origin_of(&self, at: u32) -> Point {
        Point::new(
            self.origin.x + f64::from(at) * self.step.x,
            self.origin.y + f64::from(at) * self.step.y,
        )
    }

    /// The rectangles of every placement of `structure`.
    pub fn rectangles<'a>(&'a self, structure: &'a Structure) -> impl Iterator<Item = Rectangle> + 'a {
        (0..self.count).flat_map(move |at| {
            structure.rectangles.iter().map(move |rectangle| {
                let mut placed = rectangle.placed(self.quarter_turns, self.origin_of(at));
                placed.name = format!("{}[{}]", self.name, self.first + i64::from(at));
                placed
            })
        })
    }

    /// The row turned about the origin by whole quarter turns, then moved, with `prefix` before its name.
    pub(crate) fn placed(&self, prefix: &str, quarter_turns: u8, by: Point) -> Translates {
        Translates {
            name: format!("{prefix}{}", self.name),
            structure: self.structure.clone(),
            origin: self.origin.turned(quarter_turns).moved(by),
            step: self.step.turned(quarter_turns),
            count: self.count,
            quarter_turns: (self.quarter_turns + quarter_turns) % 4,
            first: self.first,
        }
    }
}
