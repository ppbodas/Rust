// Define Point struct
// Implement display trait

#[derive(Debug, PartialEq)]
pub struct Point {
    x: f64,
    y: f64,
}

impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Point {
    pub  fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }
}

// Define Line struct
pub struct Line {
    start: Point,
    end: Point,
}

impl Line {
    pub(crate) fn new(p0: Point, p1: Point) -> Line {
        Line {
            start: p0,
            end: p1,
        }
    }
}

impl Line {
    pub fn len(&self) -> f64 {
        let dx = self.start.x - self.end.x;
        let dy = self.start.y - self.end.y;
        (dx * dx + dy * dy).sqrt()
    }
}
