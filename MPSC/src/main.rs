use std::sync::mpsc;

fn main() {
    // Create MPSC channel
    let (tx, rx) = mpsc::channel();

    let tx2 = tx.clone();

    // Create thread
    let _ = std::thread::spawn(move || {
        // Create vector
        let vals = vec![
            String::from("hi"),
            String::from("from"),
            String::from("the"),
            String::from("thread"),
        ];
        // Send message to main thread
        for val in vals {
            tx.send(val).unwrap();
            // Wait for 1 second
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    // Create one more thread
    let _ = std::thread::spawn(move || {
        // Create vector
        let vals = vec![
            String::from("more"),
            String::from("messages"),
            String::from("for"),
            String::from("you"),
        ];
        // Send message to main thread
        for val in vals {
            tx2.send(val).unwrap();
            // Wait for 1 second
            std::thread::sleep(std::time::Duration::from_secs(1));

        }
    });

    for received in rx {
        println!("Got: {}", received);
    }
}
