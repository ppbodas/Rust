use std::collections::HashMap;
use bytes::Bytes;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use mini_redis::{Connection, Frame};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;


#[tokio::main]
async fn main()  {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    println!("Listening on port 6379");
    let db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("Inside Loop: {}", addr);

        let db = db.clone();

        // Create multiple tasks to handle multiple connections
        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}

async fn process(socket: TcpStream, db: Db) {
    use mini_redis::Command::{self, Get, Set};
    use std::collections::HashMap;

    let mut connection = Connection::new(socket);

    // The connection could be closed at any time by the client
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                let mut db = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone().into())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("unimplemented command: {:?}", cmd),
        };
        // Write the response back to the client
        connection.write_frame(&response).await.unwrap();
    }
}
