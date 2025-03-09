fn main() {
    println!("Hello, world!");

    // Create a critical section to protect shared data
    let data = std::sync::Arc::new(std::sync::Mutex::new(0));

    // Create a vector to hold the join handles
    let mut handles = vec![];

    // Create 10 threads
    for _ in 0..10 {
        let data = data.clone();
        let handle = std::thread::spawn(move || {
            // Lock the data
            let mut data = data.lock().unwrap();

            // Increment the data
            *data += 1;
        });

        // Store the join handle
        handles.push(handle);
    }

    // Wait for all threads to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // Lock the data
    let data = data.lock().unwrap();

    // Print the data
    println!("Data: {}", *data);
    
}
