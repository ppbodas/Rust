mod maths;
mod SocialScience;

fn main() {
    println!("3 + 2 = {}", maths::arithmetic::add(3, 2));
    println!("3 - 2 = {}", maths::arithmetic::subtract(3, 2));
    println!("3 * 2 = {}", maths::arithmetic::multiply(3, 2));
    println!("3 / 2 = {:?}", maths::arithmetic::divide(3.0, 2.0));

    println!("Area of circle (r=5) = {:.2}", maths::geometry::area_of_circle(5.0));
    println!("Perimeter of circle (r=5) = {:.2}", maths::geometry::perimeter_of_circle(5.0));
    println!("Area of rectangle (4x6) = {:.2}", maths::geometry::area_of_rectangle(4.0, 6.0));
    println!("Perimeter of rectangle (4x6) = {:.2}", maths::geometry::perimeter_of_rectangle(4.0, 6.0));

    println!("GDP growth = {:.2}%", SocialScience::economics::gdp_growth(110.0, 100.0));
}
