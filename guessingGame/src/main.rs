fn main() {
    println!("Hello, world! Enter number");

    // Generate random number in 1 to 100
    let random_number = (rand::random::<i32>() % 100 + 1).abs();


    // Loop over. Compare input number with random number if number is smaller tell user to enter larger value
    // if number is larger tell user to enter smaller value
    // if number is equal to random number tell user that he won

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");
        let input: i32 = input.trim().parse().expect("Please type a number!");

        if input < random_number {
            // Take integer as input
            println!("You entered: {}", input);
            println!("Enter larger value");
        } else if input > random_number {
            println!("Enter smaller value");
        } else {
            println!("You won");
            break;
        }

    }


}
