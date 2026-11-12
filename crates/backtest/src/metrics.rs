//! PnL, calibration, Sharpe.

pub fn sharpe(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let var =
        returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    if var <= 0.0 {
        return 0.0;
    }
    mean / var.sqrt()
}

pub fn win_rate(wins: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        wins as f64 / total as f64
    }
}
