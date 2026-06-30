use std::sync::{Arc, Mutex};

fn main() {
    println!("Hello, world!");

    // Create a Mutex

    let counter = Arc::new(Mutex::new(5));

    let mut handles = vec![];
    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        let handle = std::thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Result: {}", *counter.lock().unwrap());
}
