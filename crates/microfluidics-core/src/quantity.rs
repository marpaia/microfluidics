//! Values with dimensions.
//!
//! Every number in a design is a [`Quantity`]: a value in SI units and the dimension it has.
//! Arithmetic combines dimensions, so adding a length to a pressure is an error where it is
//! written.

use serde::Serialize;

/// A dimension, as powers of length, mass and time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Dim {
    pub length: i8,
    pub mass: i8,
    pub time: i8,
}

impl Dim {
    pub const NONE: Dim = Dim::new(0, 0, 0);
    pub const LENGTH: Dim = Dim::new(1, 0, 0);
    pub const TIME: Dim = Dim::new(0, 0, 1);
    pub const PRESSURE: Dim = Dim::new(-1, 1, -2);
    pub const FLOW: Dim = Dim::new(3, 0, -1);
    pub const VISCOSITY: Dim = Dim::new(-1, 1, -1);

    pub const fn new(length: i8, mass: i8, time: i8) -> Self {
        Self { length, mass, time }
    }

    /// The dimension in words, as an error message names it.
    pub fn name(self) -> String {
        match self {
            Dim::NONE => "a plain number".to_owned(),
            Dim::LENGTH => "a length".to_owned(),
            Dim::TIME => "a time".to_owned(),
            Dim::PRESSURE => "a pressure".to_owned(),
            Dim::FLOW => "a flow rate".to_owned(),
            Dim::VISCOSITY => "a viscosity".to_owned(),
            Dim {
                length: 2,
                mass: 0,
                time: 0,
            } => "an area".to_owned(),
            Dim {
                length: 3,
                mass: 0,
                time: 0,
            } => "a volume".to_owned(),
            Dim { length, mass, time } => format!("a quantity in m^{length} kg^{mass} s^{time}"),
        }
    }

    fn combine(self, other: Dim, sign: i8) -> Dim {
        Dim::new(
            self.length + sign * other.length,
            self.mass + sign * other.mass,
            self.time + sign * other.time,
        )
    }
}

/// The units a number may carry, with the SI value of one of each.
const UNITS: [(&str, f64, Dim); 17] = [
    ("nm", 1e-9, Dim::LENGTH),
    ("um", 1e-6, Dim::LENGTH),
    ("mm", 1e-3, Dim::LENGTH),
    ("Pa", 1.0, Dim::PRESSURE),
    ("mbar", 100.0, Dim::PRESSURE),
    ("kPa", 1e3, Dim::PRESSURE),
    ("psi", 6_894.757_293_168, Dim::PRESSURE),
    ("ul_min", 1e-9 / 60.0, Dim::FLOW),
    ("nl_min", 1e-12 / 60.0, Dim::FLOW),
    ("ul_h", 1e-9 / 3600.0, Dim::FLOW),
    ("Pa_s", 1.0, Dim::VISCOSITY),
    ("mPa_s", 1e-3, Dim::VISCOSITY),
    ("s", 1.0, Dim::TIME),
    ("min", 60.0, Dim::TIME),
    ("h", 3600.0, Dim::TIME),
    ("%", 0.01, Dim::NONE),
    ("", 1.0, Dim::NONE),
];

/// A value in SI units with its dimension.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Quantity {
    pub si: f64,
    pub dim: Dim,
}

impl Quantity {
    pub const fn number(value: f64) -> Self {
        Self {
            si: value,
            dim: Dim::NONE,
        }
    }

    pub const fn length(metres: f64) -> Self {
        Self {
            si: metres,
            dim: Dim::LENGTH,
        }
    }

    /// `value` of the unit `unit` names; `None` for a unit the language does not have.
    pub fn with_unit(value: f64, unit: &str) -> Option<Self> {
        let (_, scale, dim) = UNITS.iter().find(|(name, _, _)| *name == unit)?;
        Some(Self {
            si: value * scale,
            dim: *dim,
        })
    }

    /// The quantity `text` writes, such as `412.6um`, `-3` or `5%`; `None` when it is not one.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        // The number is the longest start of the text that reads as one.
        let split = (0..=text.len())
            .rev()
            .filter(|at| text.is_char_boundary(*at))
            .find(|at| text[..*at].parse::<f64>().is_ok())?;
        let value = text[..split].parse::<f64>().ok()?;
        value
            .is_finite()
            .then(|| Self::with_unit(value, &text[split..]))
            .flatten()
    }

    /// The names of every unit, for an error message.
    pub fn unit_names() -> String {
        let names: Vec<&str> = UNITS
            .iter()
            .map(|(name, _, _)| *name)
            .filter(|name| !name.is_empty())
            .collect();
        names.join(", ")
    }

    pub fn plus(self, other: Quantity) -> Result<Quantity, String> {
        self.same(other, "adds")?;
        Ok(Quantity {
            si: self.si + other.si,
            dim: self.dim,
        })
    }

    pub fn minus(self, other: Quantity) -> Result<Quantity, String> {
        self.same(other, "subtracts")?;
        Ok(Quantity {
            si: self.si - other.si,
            dim: self.dim,
        })
    }

    pub fn times(self, other: Quantity) -> Quantity {
        Quantity {
            si: self.si * other.si,
            dim: self.dim.combine(other.dim, 1),
        }
    }

    pub fn over(self, other: Quantity) -> Quantity {
        Quantity {
            si: self.si / other.si,
            dim: self.dim.combine(other.dim, -1),
        }
    }

    /// Raised to a plain whole power.
    pub fn powi(self, exponent: i8) -> Quantity {
        let dim = Dim::new(
            self.dim.length * exponent,
            self.dim.mass * exponent,
            self.dim.time * exponent,
        );
        Quantity {
            si: self.si.powi(i32::from(exponent)),
            dim,
        }
    }

    /// Fails unless `other` has this quantity's dimension; `verb` says what was being done.
    pub fn same(self, other: Quantity, verb: &str) -> Result<(), String> {
        if self.dim == other.dim {
            Ok(())
        } else {
            Err(format!("this {verb} {} and {}", self.dim.name(), other.dim.name()))
        }
    }

    /// The SI value, when the quantity has dimension `dim`; otherwise what `what` should have been.
    pub fn expect(self, dim: Dim, what: &str) -> Result<f64, String> {
        if self.dim == dim {
            Ok(self.si)
        } else {
            Err(format!("{what} must be {}, not {}", dim.name(), self.dim.name()))
        }
    }

    /// The quantity as it would be written in a design: in the usual unit of its dimension.
    pub fn written(self) -> String {
        let (unit, scale) = match self.dim {
            Dim::LENGTH => ("um", 1e-6),
            Dim::PRESSURE => ("mbar", 100.0),
            Dim::FLOW => ("ul_min", 1e-9 / 60.0),
            Dim::VISCOSITY => ("mPa_s", 1e-3),
            Dim::TIME => ("s", 1.0),
            _ => ("", 1.0),
        };
        let value = self.si / scale;
        // Twelve significant figures, then whatever shorter text reads back to the same number.
        let text = format!("{value:.12e}")
            .parse::<f64>()
            .map_or_else(|_| value.to_string(), |rounded| rounded.to_string());
        format!("{text}{unit}")
    }
}

#[cfg(test)]
mod tests {
    use crate::quantity::{Dim, Quantity};

    #[test]
    fn units_convert_to_si_with_their_dimension() {
        let width = Quantity::with_unit(1.2, "um").unwrap();
        assert!((width.si - 1.2e-6).abs() < 1e-18 && width.dim == Dim::LENGTH);
        assert_eq!(Quantity::with_unit(100.0, "mbar").unwrap().si, 1e4);
        assert_eq!(Quantity::with_unit(5.0, "%").unwrap(), Quantity::number(0.05));
        assert!(Quantity::with_unit(1.0, "furlong").is_none());
    }

    #[test]
    fn arithmetic_keeps_track_of_dimensions() {
        let (length, pressure) = (Quantity::length(2e-6), Quantity::with_unit(1.0, "mbar").unwrap());
        assert_eq!(length.plus(pressure).unwrap_err(), "this adds a length and a pressure");
        assert_eq!(length.times(length).dim.name(), "an area");
        assert_eq!(length.over(length), Quantity::number(1.0));
        assert_eq!(length.powi(3).dim.name(), "a volume");
        assert_eq!(
            length.expect(Dim::PRESSURE, "the drive").unwrap_err(),
            "the drive must be a pressure, not a length"
        );
    }

    #[test]
    fn a_quantity_is_read_from_its_written_form() {
        for text in ["412.6um", "-3", "5%", "0.7mPa_s", "10mbar", "2.5e-3mm", "1ul_min"] {
            let read = Quantity::parse(text).unwrap();
            // Its written form reads back as the same value, to the last few bits.
            let again = Quantity::parse(&read.written()).unwrap();
            assert!(
                again.dim == read.dim && (again.si - read.si).abs() <= 1e-12 * read.si.abs(),
                "{text}"
            );
        }
        assert_eq!(
            Quantity::parse("412.6um").unwrap(),
            Quantity::with_unit(412.6, "um").unwrap()
        );
        assert!(
            Quantity::parse("1um 2um").is_none() && Quantity::parse("far").is_none() && Quantity::parse("").is_none()
        );
        assert!(Quantity::parse("3furlong").is_none() && Quantity::parse("infum").is_none());
    }

    #[test]
    fn a_quantity_is_written_in_the_usual_unit_of_its_dimension() {
        assert_eq!(Quantity::length(412.6e-6).written(), "412.6um");
        assert_eq!(Quantity::number(8.0).written(), "8");
        let written = Quantity::length(1.2e-6).written();
        assert_eq!(written, "1.2um");
    }
}
