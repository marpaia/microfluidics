//! Points, rectangles and directions in the chip plane. Lengths are metres.

use serde::Serialize;

/// Lengths closer than this are equal (one picometre).
pub const TOLERANCE: f64 = 1e-12;

/// The grid every drawn coordinate lies on: one nanometre, the unit masks are written in.
pub const GRID: f64 = 1e-9;

/// A length moved to the grid. A length midway between two grid points goes to the upper one,
/// however it was arrived at, so that two ways of working out one edge give one edge.
pub fn snapped(metres: f64) -> f64 {
    (metres / GRID + 0.5 + 1e-6).floor() * GRID
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// This point moved to the grid.
    pub fn snapped(self) -> Self {
        Self::new(snapped(self.x), snapped(self.y))
    }

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The point turned counterclockwise about the origin by whole quarter turns.
    pub fn turned(self, quarter_turns: u8) -> Point {
        match quarter_turns % 4 {
            0 => self,
            1 => Point::new(-self.y, self.x),
            2 => Point::new(-self.x, -self.y),
            _ => Point::new(self.y, -self.x),
        }
    }

    pub fn moved(self, by: Point) -> Point {
        Point::new(self.x + by.x, self.y + by.y)
    }
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    pub(crate) fn interiors_overlap(&self, other: &Rect) -> bool {
        self.min.x < other.max.x - TOLERANCE
            && other.min.x < self.max.x - TOLERANCE
            && self.min.y < other.max.y - TOLERANCE
            && other.min.y < self.max.y - TOLERANCE
    }

    pub(crate) fn contains(&self, other: &Rect) -> bool {
        self.min.x <= other.min.x + TOLERANCE
            && self.min.y <= other.min.y + TOLERANCE
            && other.max.x <= self.max.x + TOLERANCE
            && other.max.y <= self.max.y + TOLERANCE
    }
}

/// The outward direction of a port: the way fluid leaves a structure through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    PlusX,
    PlusY,
    MinusX,
    MinusY,
}

impl Direction {
    const COUNTERCLOCKWISE: [Direction; 4] = [Direction::PlusX, Direction::PlusY, Direction::MinusX, Direction::MinusY];

    pub fn turned(self, quarter_turns: u8) -> Self {
        let at = Self::COUNTERCLOCKWISE
            .iter()
            .position(|direction| *direction == self)
            .unwrap_or(0);
        Self::COUNTERCLOCKWISE[(at + usize::from(quarter_turns)) % 4]
    }

    pub fn opposite(self) -> Self {
        self.turned(2)
    }

    /// One metre along the direction.
    pub fn unit(self) -> Point {
        match self {
            Direction::PlusX => Point::new(1.0, 0.0),
            Direction::PlusY => Point::new(0.0, 1.0),
            Direction::MinusX => Point::new(-1.0, 0.0),
            Direction::MinusY => Point::new(0.0, -1.0),
        }
    }
}

/// The bounding rectangle of a set of points.
pub(crate) fn bounds(points: impl Iterator<Item = Point>) -> Rect {
    let mut rect = Rect {
        min: Point::new(f64::INFINITY, f64::INFINITY),
        max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };
    for point in points {
        rect.min = Point::new(rect.min.x.min(point.x), rect.min.y.min(point.y));
        rect.max = Point::new(rect.max.x.max(point.x), rect.max.y.max(point.y));
    }
    rect
}
