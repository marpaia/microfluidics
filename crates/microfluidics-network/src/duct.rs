//! Flow in a straight rectangular duct. This is the only flow physics in the crate.
//!
//! Flow is taken as steady, laminar and fully developed, which holds at the Reynolds numbers of
//! these chips. Bends, junctions and entrances add nothing here.

use std::f64::consts::PI;

/// The density of water, in kilograms a cubic metre, for the Reynolds number.
pub const WATER_DENSITY: f64 = 1000.0;

/// How much of a parallel-plate duct's flow a rectangle of sides `long` and `short` carries.
fn fill(long: f64, short: f64) -> f64 {
    let series: f64 = (1..40)
        .step_by(2)
        .map(f64::from)
        .map(|n| (n * PI * long / (2.0 * short)).tanh() / n.powi(5))
        .sum();
    1.0 - 192.0 * short / (PI.powi(5) * long) * series
}

/// The pressure drop per unit flow rate and viscosity of a duct, in 1/m^3: the exact series for
/// a rectangle.
pub fn resistance_per_viscosity(width: f64, height: f64, length: f64) -> f64 {
    let (long, short) = (width.max(height), width.min(height));
    12.0 * length / (long * short.powi(3) * fill(long, short))
}

/// The shear stress on the middle of a duct's wider wall, in pascals: that of parallel plates
/// under the duct's true pressure gradient, which is an upper estimate for a rectangle.
pub fn wall_shear(viscosity: f64, flow: f64, width: f64, height: f64) -> f64 {
    let (long, short) = (width.max(height), width.min(height));
    6.0 * viscosity * flow.abs() / (long * short.powi(2) * fill(long, short))
}

/// The Reynolds number of a duct, on its hydraulic diameter.
pub fn reynolds(density: f64, viscosity: f64, flow: f64, width: f64, height: f64) -> f64 {
    let hydraulic = 2.0 * width * height / (width + height);
    density * flow.abs() * hydraulic / (viscosity * width * height)
}

#[cfg(test)]
mod tests {
    use crate::duct::{resistance_per_viscosity, reynolds, wall_shear};

    #[test]
    fn the_series_gives_the_known_limits() {
        // A square duct of side a: 28.45 L / a^4.
        let square = resistance_per_viscosity(50e-6, 50e-6, 1e-3) * 50e-6_f64.powi(4) / 1e-3;
        assert!((square - 28.454).abs() < 0.01, "{square}");
        // A very wide duct: 12 L / (w h^3), as between parallel plates.
        let wide = resistance_per_viscosity(10e-3, 10e-6, 1e-3) * 10e-3 * 10e-6_f64.powi(3) / 1e-3;
        assert!((wide - 12.0).abs() < 0.01, "{wide}");
        // The two sides may be given either way round.
        assert_eq!(
            resistance_per_viscosity(100e-6, 20e-6, 1e-3),
            resistance_per_viscosity(20e-6, 100e-6, 1e-3)
        );
    }

    #[test]
    fn shear_and_reynolds_number_are_those_of_the_textbook_cases() {
        // Between wide plates: 6 mu Q / (w h^2).
        let shear = wall_shear(1e-3, 1e-10, 10e-3, 10e-6);
        assert!(
            (shear / (6.0 * 1e-3 * 1e-10 / (10e-3 * 1e-10)) - 1.0).abs() < 1e-3,
            "{shear}"
        );
        // Water at 1 uL/min in a 100 x 20 um duct: far below 1.
        let slow = reynolds(1000.0, 1e-3, 1e-9 / 60.0, 100e-6, 20e-6);
        assert!((slow - 0.2778).abs() < 1e-3, "{slow}");
    }
}
