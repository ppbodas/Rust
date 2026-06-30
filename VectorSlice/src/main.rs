fn main() {
    println!("Hello, world!");

    let mut v = vec![1, 2, 3, 4, 5];
    let s = slice_of_vector(&mut v, 0, 2);
    println!("{:?}", s);

    // Print type of s
    print_type_of(&s);
    print_type_of(&v);

    // Create vector of names
    let mut names = vec!["Alice", "Bob", "Charlie"];
    let s = slice_of_vector(&mut names, 0, 2);
    s[0] = "Prathmesh";
    println!("{:?}", s);
    print_type_of(&s);
    print_type_of(&names);
    println!("{:?}", names);




}

use std::any::type_name;

fn print_type_of<T>(_: &T) {
    println!("{}", type_name::<T>());
}



// Return slice of vector. Take generics as input. Input should take index

fn slice_of_vector<T>(v: &mut Vec<T>, start: usize, end: usize) -> &mut [T] {
    &mut v[start..end]
}
