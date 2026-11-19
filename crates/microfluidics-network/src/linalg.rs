//! The little linear algebra the crate needs.

/// Solve `matrix * x = right` by elimination with partial pivoting; `None` when singular.
pub(crate) fn solve_dense(mut matrix: Vec<Vec<f64>>, mut right: Vec<f64>) -> Option<Vec<f64>> {
    let size = right.len();
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |most, value| most.max(value.abs()));
    for column in 0..size {
        let pivot = (column..size).max_by(|a, b| matrix[*a][column].abs().total_cmp(&matrix[*b][column].abs()))?;
        if matrix[pivot][column].abs() <= 1e-13 * scale {
            return None;
        }
        matrix.swap(column, pivot);
        right.swap(column, pivot);
        let pivot_row = matrix[column].clone();
        for row in column + 1..size {
            let factor = matrix[row][column] / pivot_row[column];
            if factor != 0.0 {
                for (value, above) in matrix[row].iter_mut().zip(&pivot_row).skip(column) {
                    *value -= factor * above;
                }
                right[row] -= factor * right[column];
            }
        }
    }
    for row in (0..size).rev() {
        let known: f64 = (row + 1..size).map(|across| matrix[row][across] * right[across]).sum();
        right[row] = (right[row] - known) / matrix[row][row];
    }
    Some(right)
}
