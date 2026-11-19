//! Least squares within bounds: one small routine, used by closure.

use crate::linalg::solve_dense;

/// What a search found.
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    /// Where it ended, each variable within 0..1.
    pub x: Vec<f64>,
    pub residuals: Vec<f64>,
    pub iterations: usize,
}

fn cost(residuals: &[f64]) -> f64 {
    residuals.iter().map(|residual| residual * residual).sum()
}

/// Minimise the sum of squared residuals over variables each kept within 0..1, by the
/// Levenberg-Marquardt method with derivatives taken by stepping each variable by its `step`.
/// `residuals` returns `None` for a point that is not admissible; such a point is never moved
/// to. `None` when the start itself is not admissible.
pub fn least_squares(
    start: &[f64],
    steps: &[f64],
    residuals: &mut dyn FnMut(&[f64]) -> Option<Vec<f64>>,
    iterations: usize,
) -> Option<Fit> {
    let mut x = start.to_vec();
    let mut held = residuals(&x)?;
    let mut damping = 1e-2;
    let mut taken = 0;
    while taken < iterations && cost(&held) > 1e-18 {
        taken += 1;
        // The derivative of every residual with respect to each variable, stepping whichever
        // way stays within bounds and is admissible. Which ways are open is kept: a variable is
        // not asked to move where it cannot go.
        let mut jacobian = vec![vec![0.0; x.len()]; held.len()];
        let mut open = vec![(false, false); x.len()];
        for variable in 0..x.len() {
            for (up, step) in [(true, steps[variable]), (false, -steps[variable])] {
                let mut moved = x.clone();
                moved[variable] += step;
                if !(0.0..=1.0).contains(&moved[variable]) {
                    continue;
                }
                if let Some(there) = residuals(&moved) {
                    if up {
                        open[variable].0 = true;
                    } else {
                        open[variable].1 = true;
                    }
                    if jacobian.iter().all(|row| row[variable] == 0.0) {
                        for (row, (after, before)) in there.iter().zip(&held).enumerate() {
                            jacobian[row][variable] = (after - before) / step;
                        }
                    }
                }
            }
        }
        let size = x.len();
        let mut normal = vec![vec![0.0; size]; size];
        let mut gradient = vec![0.0; size];
        for (row, residual) in jacobian.iter().zip(&held) {
            for a in 0..size {
                gradient[a] -= row[a] * residual;
                for b in 0..size {
                    normal[a][b] += row[a] * row[b];
                }
            }
        }
        // A variable that the descent would push where it cannot go is held where it is.
        for variable in 0..size {
            let (up, down) = open[variable];
            if (gradient[variable] > 0.0 && !up) || (gradient[variable] < 0.0 && !down) || (!up && !down) {
                gradient[variable] = 0.0;
                normal[variable].fill(0.0);
                normal.iter_mut().for_each(|row| row[variable] = 0.0);
                normal[variable][variable] = 1.0;
            }
        }
        // Raise the damping until a step is admissible and lowers the cost.
        let mut moved = false;
        while damping < 1e9 {
            let mut damped = normal.clone();
            for (at, row) in damped.iter_mut().enumerate() {
                row[at] += damping * row[at].max(1e-12);
            }
            let trial: Option<Vec<f64>> = solve_dense(damped, gradient.clone()).map(|step| {
                x.iter()
                    .zip(&step)
                    .map(|(value, change)| (value + change).clamp(0.0, 1.0))
                    .collect()
            });
            match trial.as_ref().and_then(|trial| Some((trial, residuals(trial)?))) {
                Some((trial, there)) if cost(&there) < cost(&held) => {
                    let gained = cost(&held) - cost(&there);
                    (x, held) = (trial.clone(), there);
                    damping = (damping / 5.0).max(1e-9);
                    moved = gained > 1e-12 * cost(&held).max(1e-30);
                    break;
                }
                _ => damping *= 5.0,
            }
        }
        if !moved {
            break;
        }
    }
    Some(Fit {
        x,
        residuals: held,
        iterations: taken,
    })
}

#[cfg(test)]
mod tests {
    use crate::optimise::least_squares;

    #[test]
    fn a_curved_fit_is_found_within_bounds() {
        // Fit x0 * exp(-3 x1 t) to values made with x = (0.6, 0.4).
        let times = [0.0, 0.5, 1.0, 2.0];
        let model = |x: &[f64], t: f64| x[0] * (-3.0 * x[1] * t).exp();
        let mut residuals = |x: &[f64]| Some(times.iter().map(|t| model(x, *t) - model(&[0.6, 0.4], *t)).collect());
        let fit = least_squares(&[0.2, 0.9], &[1e-4, 1e-4], &mut residuals, 100).unwrap();
        assert!(
            (fit.x[0] - 0.6).abs() < 1e-6 && (fit.x[1] - 0.4).abs() < 1e-6,
            "{fit:?}"
        );
    }

    #[test]
    fn the_search_stops_at_a_bound_and_never_enters_what_is_not_admissible() {
        // The least of (x - 1.5)^2 within 0..1 is at 1.
        let mut beyond = |x: &[f64]| Some(vec![x[0] - 1.5]);
        assert!((least_squares(&[0.2], &[1e-4], &mut beyond, 100).unwrap().x[0] - 1.0).abs() < 1e-9);
        // Nothing above 0.7 is admissible: the search ends at or below it.
        let mut walled = |x: &[f64]| (x[0] <= 0.7).then(|| vec![x[0] - 0.9]);
        let fit = least_squares(&[0.2], &[1e-4], &mut walled, 100).unwrap();
        assert!(fit.x[0] <= 0.7 && fit.x[0] > 0.6, "{fit:?}");
        assert!(least_squares(&[0.9], &[1e-4], &mut walled, 100).is_none());
    }
}
