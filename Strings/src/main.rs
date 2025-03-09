fn main() {
    println!("Hello, world!");

    let full_name = read_full_name();
    println!("Hello, {}!", full_name);

    let (first_name, last_name) = split_full_name(&full_name);
    println!("Your first name is {} and your last name is {}.", first_name, last_name);

    println!("Type of first name is {:?}", print_type_of(&first_name));
    println!("Type of last name is {:?}", print_type_of(&last_name));
    println!("Type of full: name is {:?}", print_type_of(&full_name));

    println!("Good Bye {}!", full_name);

    // Iterate over full name characters
    for c in full_name.chars() {
        println!("{}", c);
    }
}

fn print_type_of<T>(_: &T) -> &str{
    use std::any::type_name;
    return type_name::<T>();
}

// Split full name into first and last name
fn split_full_name(full_name: &str) -> (&str, &str) {
    let mut parts = full_name.split_whitespace();
    let first_name = parts.next().unwrap();
    let last_name = parts.next().unwrap();
    (first_name, last_name)
}

// Read full name from console
fn read_full_name() -> String {
    let mut full_name = String::new();
    println!("Please enter your full name:");
    std::io::stdin().read_line(&mut full_name).expect("Failed to read line");
    full_name.trim().to_string()
}
