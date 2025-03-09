use tokio::sync::mpsc;
use mini_redis::client;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

mod command;
use command::Command;


#[tokio::main]
async fn main() {
    // Create mpsc channel
    let (tx, mut rx) = mpsc::channel(100);

    let manager = tokio::spawn(async move {
        create_receiver(rx).await;
    });

    let (t1, t2) = create_command_sender(tx);

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}

fn create_command_sender(tx: Sender<Command>) -> (JoinHandle<()>, JoinHandle<()>) {
    let tx2 = tx.clone();

    let t1 = create_get_command_sender(tx);

    let t2 = create_set_command_sender(tx2);

    (t1, t2)
}

fn create_set_command_sender(tx2: Sender<Command>) -> JoinHandle<()> {
    let t2 = tokio::spawn(async move {
        let cmd = Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
        };

        tx2.send(cmd).await.unwrap();
    });
    t2
}

fn create_get_command_sender(tx: Sender<Command>) -> JoinHandle<()> {
    // Spawn two tasks, one gets a key, the other sets a key
    let t1 = tokio::spawn(async move {
        let cmd = Command::Get {
            key: "foo".to_string(),
        };

        tx.send(cmd).await.unwrap();
    });
    t1
}

async fn create_receiver(mut rx: Receiver<Command>) {
    // Establish a connection to the server
    let mut client = client::connect("127.0.0.1:6379").await.unwrap();

    // Start receiving messages
    while let Some(cmd) = rx.recv().await {
        use Command::*;

        match cmd {
            Get { key } => {
                let res = client.get(&key).await;
                if let Ok(Some(val)) = res {
                    println!("got value for {}: {:?}", &key, val);
                } else {
                    println!("key not found");
                }
            }
            Set { key, val } => {
                client.set(&key, val).await;
            }
        }
    }
}