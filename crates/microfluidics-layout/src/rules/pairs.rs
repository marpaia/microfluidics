//! Which features are near each other, and which touch.

use crate::geometry::{Point, Rect, TOLERANCE};
use crate::rectangle::Rectangle;

/// The gap between two rectangles along each axis; zero where their extents meet or overlap.
fn gaps(a: &Rect, b: &Rect) -> (f64, f64) {
    let along = |a_min: f64, a_max: f64, b_min: f64, b_max: f64| (b_min - a_max).max(a_min - b_max).max(0.0);
    (
        along(a.min.x, a.max.x, b.min.x, b.max.x),
        along(a.min.y, a.max.y, b.min.y, b.max.y),
    )
}

/// Whether two rectangles share a length of edge, so that fluid passes between them.
pub(crate) fn joined(a: &Rect, b: &Rect) -> bool {
    let shared = |a_min: f64, a_max: f64, b_min: f64, b_max: f64| a_max.min(b_max) - a_min.max(b_min);
    let (x, y) = (
        shared(a.min.x, a.max.x, b.min.x, b.max.x),
        shared(a.min.y, a.max.y, b.min.y, b.max.y),
    );
    (x.abs() <= TOLERANCE && y > TOLERANCE) || (y.abs() <= TOLERANCE && x > TOLERANCE)
}

/// What one pass over nearby pairs of rectangles finds.
pub(crate) struct Pairs {
    /// Pairs of one layer that do not touch, no further apart than any rule looks: `(gap, a, b)`.
    pub(crate) close: Vec<(f64, usize, usize)>,
    /// Pairs that share an edge or overlap, on any layers.
    pub(crate) touching: Vec<(usize, usize)>,
    /// The smallest gap between two features of one layer.
    pub(crate) nearest: Option<f64>,
}

pub(crate) fn pairs(rectangles: &[Rectangle], bounds: &[Rect], reach: f64) -> Pairs {
    let mut order: Vec<usize> = (0..rectangles.len()).collect();
    order.sort_by(|a, b| bounds[*a].min.x.total_cmp(&bounds[*b].min.x));
    let mut found = Pairs {
        close: Vec::new(),
        touching: Vec::new(),
        nearest: None,
    };
    for (place, &a) in order.iter().enumerate() {
        for &b in &order[place + 1..] {
            // Until some gap is known, every pair is looked at; after that, only pairs nearer
            // than it or within the rules' reach.
            if bounds[b].min.x - bounds[a].max.x > reach.max(found.nearest.unwrap_or(f64::INFINITY)) {
                break;
            }
            let (x, y) = gaps(&bounds[a], &bounds[b]);
            if x <= TOLERANCE && y <= TOLERANCE {
                if joined(&bounds[a], &bounds[b]) || bounds[a].interiors_overlap(&bounds[b]) {
                    found.touching.push((a, b));
                }
                continue;
            }
            if rectangles[a].layer != rectangles[b].layer {
                continue;
            }
            let gap = x.hypot(y);
            found.nearest = Some(found.nearest.map_or(gap, |nearest| nearest.min(gap)));
            if gap <= reach {
                found.close.push((gap, a, b));
            }
        }
    }
    found
}

/// How far a shallow feature reaches into a deeper one it touches: the distance from the edge
/// it crosses to its far end inside. `None` when it lies wholly inside.
pub(crate) fn penetration(shallow: &Rect, deep: &Rect) -> Option<f64> {
    let crossings = [
        (shallow.min.x < deep.min.x - TOLERANCE, shallow.max.x - deep.min.x),
        (shallow.max.x > deep.max.x + TOLERANCE, deep.max.x - shallow.min.x),
        (shallow.min.y < deep.min.y - TOLERANCE, shallow.max.y - deep.min.y),
        (shallow.max.y > deep.max.y + TOLERANCE, deep.max.y - shallow.min.y),
    ];
    crossings
        .iter()
        .filter(|(crosses, _)| *crosses)
        .map(|(_, depth)| depth.max(0.0))
        .reduce(f64::min)
}

/// Whether `point` lies in or on `rect`.
pub(crate) fn within(rect: &Rect, point: Point) -> bool {
    point.x >= rect.min.x - TOLERANCE
        && point.x <= rect.max.x + TOLERANCE
        && point.y >= rect.min.y - TOLERANCE
        && point.y <= rect.max.y + TOLERANCE
}

/// The representative of the set `at` belongs to.
pub(crate) fn root(parents: &mut [usize], mut at: usize) -> usize {
    while parents[at] != at {
        parents[at] = parents[parents[at]];
        at = parents[at];
    }
    at
}
