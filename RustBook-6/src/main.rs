fn main() {
    println!("Hello, world!");


    let value = Some(5);
    let addition = plus_one(value);

    println!("{:?}", addition.unwrap());
}

fn plus_one(x: Option<i32>) -> Option<i32> {
    match x {
        None => None,
        Some(i) => Some(i + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plus_one() {
        let value = Some(5);
        let addition = plus_one(value);

        assert_eq!(addition.unwrap(), 6);
    }
}

