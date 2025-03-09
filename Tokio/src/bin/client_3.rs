use mini_redis::client;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    // Create client to connect to server
    let mut client = client::connect("127.0.0.1:6379").await.unwrap();

    // Get value from server
    let result = client.get("hello").await.unwrap();

    // Set value to server
    client.set("hello", "world".into()).await.unwrap();

    println!("got value from the server; result={:?}", result);
}