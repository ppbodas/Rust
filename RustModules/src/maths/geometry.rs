pub fn area_of_circle(r: f64) -> f64 {
    std::f64::consts::PI * r * r
}

pub fn perimeter_of_circle(r: f64) -> f64 {
    2.0 * std::f64::consts::PI * r
}

pub fn area_of_rectangle(width: f64, height: f64) -> f64 {
    width * height
}

pub fn perimeter_of_rectangle(width: f64, height: f64) -> f64 {
    2.0 * (width + height)
}
