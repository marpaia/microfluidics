//! Lettering drawn as rectangles, for naming chips on a wafer.

use crate::geometry::Point;
use crate::rectangle::Rectangle;

/// The rows of each letter, top first, five dots wide.
const GLYPHS: [(char, [u8; 7]); 38] = [
    ('A', [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    ('B', [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
    ('C', [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
    ('D', [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
    ('E', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
    ('F', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
    ('G', [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111]),
    ('H', [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    ('I', [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('J', [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
    ('K', [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
    ('L', [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
    ('M', [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
    ('N', [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
    ('O', [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('P', [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
    ('Q', [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
    ('R', [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
    ('S', [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
    ('T', [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
    ('U', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('V', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
    ('W', [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
    ('X', [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
    ('Y', [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
    ('Z', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
    ('0', [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
    ('1', [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('2', [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
    ('3', [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110]),
    ('4', [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
    ('5', [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
    ('6', [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
    ('7', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
    ('8', [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
    ('9', [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
    ('-', [0, 0, 0, 0b11111, 0, 0, 0]),
    ('_', [0, 0, 0, 0, 0, 0, 0b11111]),
];

/// How wide `text` is drawn at `height`: each letter five dots and a dot between, of seven high.
pub(crate) fn width(text: &str, height: f64) -> f64 {
    let letters = text.chars().count() as f64;
    (6.0 * letters - 1.0).max(0.0) * height / 7.0
}

/// `text` as rectangles on `layer`, `height` tall, its lower left corner at `origin`. Letters are
/// drawn as capitals; anything that is not a letter, a digit, `-` or `_` is left blank.
pub(crate) fn text(name: &str, layer: &str, text: &str, origin: Point, height: f64) -> Vec<Rectangle> {
    let dot = height / 7.0;
    let mut rectangles = Vec::new();
    for (place, letter) in text.chars().enumerate() {
        let Some((_, rows)) = GLYPHS.iter().find(|(held, _)| *held == letter.to_ascii_uppercase()) else {
            continue;
        };
        let left = origin.x + place as f64 * 6.0 * dot;
        for (row, bits) in rows.iter().enumerate() {
            let bottom = origin.y + (6 - row) as f64 * dot;
            // Each run of dots in a row is one rectangle.
            let mut column = 0;
            while column < 5 {
                let lit = |column: usize| bits >> (4 - column) & 1 == 1;
                if !lit(column) {
                    column += 1;
                    continue;
                }
                let start = column;
                while column < 5 && lit(column) {
                    column += 1;
                }
                let (min, max) = (
                    Point::new(left + start as f64 * dot, bottom),
                    Point::new(left + column as f64 * dot, bottom + dot),
                );
                rectangles.push(Rectangle::rect(
                    &format!("{name}.{}", rectangles.len()),
                    layer,
                    min,
                    max,
                ));
            }
        }
    }
    rectangles
}

#[cfg(test)]
mod tests {
    use crate::fixtures::close;
    use crate::geometry::Point;
    use crate::label::{text, width};

    #[test]
    fn letters_are_rows_of_rectangles_within_their_box() {
        // T is a bar and six dots of stem; I is two bars and five; a space is nothing.
        let drawn = text("label", "flow", "t i", Point::new(1e-3, 2e-3), 700e-6);
        assert_eq!(drawn.len(), 7 + 7);
        assert!(close(width("t i", 700e-6), 17.0 * 100e-6));
        let area: f64 = drawn.iter().map(|rectangle| rectangle.area()).sum();
        assert!(close(area, (11.0 + 11.0) * 100e-6 * 100e-6));
        for rectangle in &drawn {
            let bounds = rectangle.bounds();
            assert!(bounds.min.x >= 1e-3 - 1e-12 && bounds.max.x <= 1e-3 + 17.0 * 100e-6 + 1e-12);
            assert!(bounds.min.y >= 2e-3 - 1e-12 && bounds.max.y <= 2e-3 + 700e-6 + 1e-12);
        }
        assert!(text("label", "flow", "?", Point::new(0.0, 0.0), 700e-6).is_empty());
    }
}
