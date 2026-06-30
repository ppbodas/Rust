mod data_structures;
use data_structures::point::Point;

fn main() {
    println!("Hello, world!");

    let p = data_structures::point::Point::new(1.0, 2.0);
    println!("p: {}", p);

    println!("p: {:?}", p);

    // Create another data_structures with x = 4, y = 10
    let p2 = Point::new(4.0, 10.0);

    // Create Line between p and p2
    let line = data_structures::point::Line::new(p, p2);

    println!("line Length: {}", line.len());


    // Create user
    let user = data_structures::user::User::new("John", "Doe@gmail.com");


    let s =  String::from("Hello");
    let t = &s;
    let ref m = s;

    println!("{}", s);
    println!("{}", t);
    println!("{}", m);

}
