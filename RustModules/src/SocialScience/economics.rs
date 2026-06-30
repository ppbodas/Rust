
pub fn gdp_growth(current: f64, previous: f64) -> f64 {
    ((current - previous) / previous) * 100.0
}
