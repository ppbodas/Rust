use crate::area::Area;

mod circle;
mod area;


fn main() {
    println!("Hello, world!");

    // Create circle object
    let circle = circle::Circle::new(5.0);

    // Print circle object
    // println!("Circle: {:?}", circle);

    println!("Circle area: {}", circle.area());
}
