fn main() {
    println!("Hello, world!");

    // Create a counter which can be incremented by multiple threads
    let counter = std::sync::Arc::new(std::sync::Mutex::new(0));

    // Create a vector to store the threads
    let mut threads = Vec::new();
    for _ in 0..10 {
        // Clone the counter for each thread
        let counter = counter.clone();
        // Create a new thread
        let thread = std::thread::spawn(move || {
            // Increment the counter
            let mut counter = counter.lock().unwrap();
            *counter += 1;
        });
        // Add the thread to the vector
        threads.push(thread);
    }

    for thread in threads {
        thread.join().unwrap();
    }
    println!("Counter: {}", *counter.lock().unwrap());

}
