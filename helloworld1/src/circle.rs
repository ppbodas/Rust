use crate::area::Area;

pub struct Circle {
    radius: f64,
}

impl Circle {
    pub fn new(radius: f64) -> Circle {
        Circle { radius }
    }

    pub fn get_radius(&self) -> f64 {
        self.radius
    }
}

// Implement Area trait for circle
impl Area for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }
}