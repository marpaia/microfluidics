//! The boundary of a union of rectangles.
//!
//! A drawing for someone who makes masks holds each region of a layer once: closed outlines, no
//! two of which overlap. Rectangles that touch or overlap are one region, and its boundary is
//! found on the grid their own edges make. A region with a hole is one outline that runs in to
//! the hole along a cut and back out the same way, so that it touches itself and crosses nothing.

use std::collections::BTreeMap;

use crate::geometry::{Point, Rect};
use crate::rules::pairs::root;

/// A corner on the grid masks are written on, in whole nanometres.
type Corner = (i64, i64);

/// A corner by the grid lines that cross at it: the column, then the row.
type Place = (usize, usize);

const NM: f64 = 1e9;

/// A rectangle's sides in whole nanometres: `[x0, y0, x1, y1]`.
fn sides(rect: &Rect) -> [i64; 4] {
    let nm = |metres: f64| (metres * NM).round() as i64;
    [nm(rect.min.x), nm(rect.min.y), nm(rect.max.x), nm(rect.max.y)]
}

/// The rectangles in sets, each set those that are joined along an edge or overlap, directly or
/// through others. Rectangles of no area are left out.
fn regions(rectangles: &[Rect]) -> Vec<Vec<[i64; 4]>> {
    let held: Vec<[i64; 4]> = (rectangles.iter().map(sides))
        .filter(|held| held[0] < held[2] && held[1] < held[3])
        .collect();
    let mut order: Vec<usize> = (0..held.len()).collect();
    order.sort_by_key(|at| held[*at][0]);
    let mut parents: Vec<usize> = (0..held.len()).collect();
    for (place, &a) in order.iter().enumerate() {
        for &b in &order[place + 1..] {
            if held[b][0] > held[a][2] {
                break;
            }
            // How much of each axis the two share: a length along both is an overlap, a length
            // along one and a meeting along the other is a shared edge.
            let x = held[a][2].min(held[b][2]) - held[a][0].max(held[b][0]);
            let y = held[a][3].min(held[b][3]) - held[a][1].max(held[b][1]);
            if (x > 0 && y >= 0) || (y > 0 && x >= 0) {
                let (a, b) = (root(&mut parents, a), root(&mut parents, b));
                parents[a] = b;
            }
        }
    }
    // Each set with the place of its first rectangle, which the sets are then put in order by,
    // so that the same drawing is written each time.
    let mut sets: BTreeMap<usize, (usize, Vec<[i64; 4]>)> = BTreeMap::new();
    for (at, sides) in held.iter().enumerate() {
        let set = sets.entry(root(&mut parents, at)).or_insert((at, Vec::new()));
        set.1.push(*sides);
    }
    let mut sets: Vec<(usize, Vec<[i64; 4]>)> = sets.into_values().collect();
    sets.sort_by_key(|(first, _)| *first);
    sets.into_iter().map(|(_, set)| set).collect()
}

/// One region on the grid its rectangles' edges make: the grid's lines, and which of the cells
/// between them a rectangle covers.
struct Grid {
    xs: Vec<i64>,
    ys: Vec<i64>,
    covered: Vec<bool>,
}

impl Grid {
    fn of(region: &[[i64; 4]]) -> Self {
        let lines = |low: usize, high: usize| {
            let mut lines: Vec<i64> = region.iter().flat_map(|sides| [sides[low], sides[high]]).collect();
            lines.sort_unstable();
            lines.dedup();
            lines
        };
        let (xs, ys) = (lines(0, 2), lines(1, 3));
        let (across, up) = (xs.len() - 1, ys.len() - 1);
        // Each rectangle adds one at its lower left cell and takes it away past its far sides;
        // summed along both axes, a cell holds how many rectangles cover it.
        let mut count = vec![0i32; (across + 1) * (up + 1)];
        let at = |lines: &[i64], value: i64| lines.partition_point(|line| *line < value);
        for sides in region {
            let (i0, i1) = (at(&xs, sides[0]), at(&xs, sides[2]));
            let (j0, j1) = (at(&ys, sides[1]), at(&ys, sides[3]));
            count[j0 * (across + 1) + i0] += 1;
            count[j0 * (across + 1) + i1] -= 1;
            count[j1 * (across + 1) + i0] -= 1;
            count[j1 * (across + 1) + i1] += 1;
        }
        let mut covered = vec![false; across * up];
        let mut below = vec![0i32; across + 1];
        for j in 0..up {
            let mut along = 0;
            for i in 0..across {
                along += count[j * (across + 1) + i];
                below[i] += along;
                covered[j * across + i] = below[i] > 0;
            }
        }
        Self { xs, ys, covered }
    }

    fn across(&self) -> usize {
        self.xs.len() - 1
    }

    fn up(&self) -> usize {
        self.ys.len() - 1
    }

    /// Whether the cell at column `i` and row `j` is covered; a cell off the grid is not.
    fn holds(&self, i: isize, j: isize) -> bool {
        let inside = i >= 0 && j >= 0 && (i as usize) < self.across() && (j as usize) < self.up();
        inside && self.covered[j as usize * self.across() + i as usize]
    }

    fn corner(&self, i: usize, j: usize) -> Corner {
        (self.xs[i], self.ys[j])
    }

    /// The area covered, in square nanometres.
    fn area(&self) -> i128 {
        let mut area = 0;
        for j in 0..self.up() {
            for i in 0..self.across() {
                if self.covered[j * self.across() + i] {
                    area += i128::from(self.xs[i + 1] - self.xs[i]) * i128::from(self.ys[j + 1] - self.ys[j]);
                }
            }
        }
        area
    }

    /// The boundary as closed loops of corners, each walked with what is covered on its left:
    /// counterclockwise round the outside, clockwise round a hole.
    fn loops(&self) -> Vec<Vec<Corner>> {
        // Every side of a covered cell that faces a cell not covered, from the grid corner it
        // leaves: the way it runs (east, north, west, south as 0 to 3) and the corner it reaches.
        let mut leaving: BTreeMap<Place, Vec<(u8, Place)>> = BTreeMap::new();
        for j in 0..self.up() {
            for i in 0..self.across() {
                let (ci, cj) = (i as isize, j as isize);
                if !self.holds(ci, cj) {
                    continue;
                }
                let mut side = |open: bool, way: u8, from: (usize, usize), to: (usize, usize)| {
                    if open {
                        leaving.entry(from).or_default().push((way, to));
                    }
                };
                side(!self.holds(ci, cj - 1), 0, (i, j), (i + 1, j));
                side(!self.holds(ci + 1, cj), 1, (i + 1, j), (i + 1, j + 1));
                side(!self.holds(ci, cj + 1), 2, (i + 1, j + 1), (i, j + 1));
                side(!self.holds(ci - 1, cj), 3, (i, j + 1), (i, j));
            }
        }
        let mut loops = Vec::new();
        while let Some((&start, sides)) = leaving.iter().next() {
            let first = sides[0];
            let (mut way, mut at) = first;
            leaving.entry(start).and_modify(|sides| {
                sides.remove(0);
            });
            let mut corners = vec![start];
            // The first corner left to walk from is never one where two cells meet only at a
            // corner, so the walk is done when it is back there.
            while at != start {
                corners.push(at);
                let Some(sides) = leaving.get_mut(&at) else {
                    break;
                };
                // Where two cells meet only at a corner, the walk turns left and stays with the
                // cell it is on, so that no outline crosses itself.
                let turn = |next: u8| (4 + next - way) % 4;
                let chosen = (0..sides.len())
                    .min_by_key(|held| match turn(sides[*held].0) {
                        1 => 0,
                        0 => 1,
                        3 => 2,
                        _ => 3,
                    })
                    .unwrap_or(0);
                let (next_way, next) = sides.remove(chosen);
                (way, at) = (next_way, next);
            }
            leaving.retain(|_, sides| !sides.is_empty());
            loops.push(straightened(
                corners.into_iter().map(|(i, j)| self.corner(i, j)).collect(),
            ));
        }
        loops
    }
}

/// A loop without the corners that lie along a straight side.
fn straightened(corners: Vec<Corner>) -> Vec<Corner> {
    let count = corners.len();
    let turns = |at: &usize| {
        let (before, here, after) = (
            corners[(at + count - 1) % count],
            corners[*at],
            corners[(at + 1) % count],
        );
        (here.0 - before.0) * (after.1 - here.1) != (here.1 - before.1) * (after.0 - here.0)
    };
    (0..count).filter(turns).map(|at| corners[at]).collect()
}

/// Twice the area a loop encloses: more than zero when it runs counterclockwise.
fn twice_area(corners: &[Corner]) -> i128 {
    let count = corners.len();
    (0..count)
        .map(|at| {
            let (a, b) = (corners[at], corners[(at + 1) % count]);
            i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1)
        })
        .sum()
}

/// Join each hole to the loop that holds it: from the hole's lowest leftmost corner a cut runs
/// left to the nearest side, and the outline goes in along the cut, round the hole, and back.
fn cut(loops: Vec<Vec<Corner>>) -> Vec<Vec<Corner>> {
    let (mut outer, mut holes): (Vec<_>, Vec<_>) = loops.into_iter().partition(|held| twice_area(held) > 0);
    let lowest_left = |hole: &Vec<Corner>| hole.iter().copied().min().unwrap_or_default();
    holes.sort_by_key(lowest_left);
    for hole in holes {
        let from = lowest_left(&hole);
        // The nearest upright side to the left that passes the corner's height.
        let mut nearest: Option<(i64, usize, usize)> = None;
        for (held, corners) in outer.iter().enumerate() {
            for at in 0..corners.len() {
                let (a, b) = (corners[at], corners[(at + 1) % corners.len()]);
                let passes = a.0 == b.0 && a.0 < from.0 && a.1.min(b.1) <= from.1 && from.1 < a.1.max(b.1);
                if passes && nearest.is_none_or(|(x, _, _)| a.0 > x) {
                    nearest = Some((a.0, held, at));
                }
            }
        }
        let Some((x, held, at)) = nearest else {
            // A hole with nothing to its left is no hole of this region; it is kept as it is.
            outer.push(hole);
            continue;
        };
        let start = hole.iter().position(|corner| *corner == from).unwrap_or(0);
        let landing = (x, from.1);
        let mut joined: Vec<Corner> = outer[held][..=at].to_vec();
        joined.push(landing);
        joined.extend((0..=hole.len()).map(|step| hole[(start + step) % hole.len()]));
        joined.push(landing);
        joined.extend_from_slice(&outer[held][at + 1..]);
        joined.dedup();
        outer[held] = joined;
    }
    outer
}

/// The boundary of the union of `rectangles`, whose sides lie along the axes: closed outlines
/// that do not overlap, each with what it encloses on its left. A union with a hole is one
/// outline that touches itself along a cut.
pub fn outlines(rectangles: &[Rect]) -> Vec<Vec<Point>> {
    let mut found = Vec::new();
    for region in regions(rectangles) {
        for corners in cut(Grid::of(&region).loops()) {
            let points = corners.iter().map(|(x, y)| Point::new(*x as f64 / NM, *y as f64 / NM));
            found.push(points.collect());
        }
    }
    found
}

/// The area of the union of `rectangles`, found cell by cell, in square metres.
pub fn union_area(rectangles: &[Rect]) -> f64 {
    let area: i128 = (regions(rectangles).iter()).map(|region| Grid::of(region).area()).sum();
    area as f64 / (NM * NM)
}

#[cfg(test)]
mod tests {
    use crate::fixtures::close;
    use crate::geometry::{Point, Rect};
    use crate::outline::{outlines, union_area};
    use crate::rectangle::area;

    /// A rectangle from its sides in micrometres.
    fn um(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect {
            min: Point::new(x0 * 1e-6, y0 * 1e-6),
            max: Point::new(x1 * 1e-6, y1 * 1e-6),
        }
    }

    fn enclosed(outlines: &[Vec<Point>]) -> f64 {
        outlines.iter().map(|outline| area(outline)).sum()
    }

    #[test]
    fn rectangles_that_touch_or_overlap_are_one_outline() {
        // A channel that overlaps the pad it enters, and a turn that shares an edge with it.
        let drawn = [
            um(0.0, 0.0, 900.0, 900.0),
            um(400.0, 800.0, 500.0, 2000.0),
            um(500.0, 1900.0, 1500.0, 2000.0),
        ];
        let found = outlines(&drawn);
        assert_eq!((found.len(), found[0].len()), (1, 10));
        assert!(close(union_area(&drawn), (810_000.0 + 110_000.0 + 100_000.0) * 1e-12));
        assert!(close(enclosed(&found), union_area(&drawn)));
        // What is covered is on the left: the outline runs counterclockwise.
        let signed: f64 = (0..10)
            .map(|at| found[0][at].x * found[0][(at + 1) % 10].y - found[0][(at + 1) % 10].x * found[0][at].y)
            .sum();
        assert!(signed > 0.0);
    }

    #[test]
    fn a_region_with_a_hole_is_one_outline_that_touches_itself_along_a_cut() {
        // Two trunks joined by two lanes: a ring.
        let ring = [
            um(0.0, 0.0, 1000.0, 100.0),
            um(0.0, 900.0, 1000.0, 1000.0),
            um(0.0, 100.0, 100.0, 900.0),
            um(900.0, 100.0, 1000.0, 900.0),
        ];
        let found = outlines(&ring);
        assert_eq!(found.len(), 1);
        // Four outer corners, the cut's outer end met going in and coming out, and the hole's
        // four corners with the one the cut reaches met twice.
        assert_eq!(found[0].len(), 11);
        assert!(close(union_area(&ring), (1_000_000.0 - 640_000.0) * 1e-12));
        assert!(close(enclosed(&found), union_area(&ring)));
        let cut = found[0]
            .iter()
            .filter(|point| close(point.y, 100e-6) && point.x < 150e-6);
        assert_eq!(cut.count(), 4);
    }

    #[test]
    fn rectangles_apart_or_meeting_only_at_a_corner_stay_apart() {
        let teeth: Vec<Rect> = (0..1000)
            .map(|at| um(0.0, 4.0 * f64::from(at), 25.0, 4.0 * f64::from(at) + 1.2))
            .collect();
        let found = outlines(&teeth);
        assert_eq!(found.len(), 1000);
        assert!(found.iter().all(|outline| outline.len() == 4));
        assert!(close(union_area(&teeth), 1000.0 * 30.0 * 1e-12));

        let corner = [um(0.0, 0.0, 10.0, 10.0), um(10.0, 10.0, 20.0, 20.0)];
        assert_eq!(outlines(&corner).len(), 2);
        // One rectangle drawn twice, and one inside another, are drawn once.
        let twice = [
            um(0.0, 0.0, 10.0, 10.0),
            um(0.0, 0.0, 10.0, 10.0),
            um(2.0, 2.0, 4.0, 4.0),
        ];
        assert_eq!(outlines(&twice).len(), 1);
        assert!(close(union_area(&twice), 100e-12));
        assert!(outlines(&[]).is_empty() && union_area(&[]) == 0.0);
    }
}
