use tokio::sync::mpsc;
use tokio::sync::oneshot;
use mini_redis::client;
use bytes::Bytes;


/// Multiple different commands are multiplexed over a single channel
#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    }
}

// Provided by the requester and used by the manager task to send the 
// command response back to the requester.
type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

// Create the channel
#[tokio::main]
async fn main() {
    // Create a new channel with a capacity of at most 32
    let (tx, mut rx) = mpsc::channel(32);

    // It is not possible to clone the receiver of an mpsc channel

    // Sending from multiple tasks is done by cloning the Sender.
    let tx2 = tx.clone();

    // Spawn 2 tasks, one gets a key, the other sets a key
    let t1 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "hello".to_string(),
            resp: resp_tx,
        };
        
        // Send the GET request
        tx.send(cmd).await.unwrap(); 

        // Await the response
        let res = resp_rx.await;
        println!("GOT = {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "foo".to_string(), 
            val: "bar".into(),
            resp: resp_tx,
        };

        // Send the SET request
        tx2.send(cmd).await.unwrap();

        // Await the response
        let res = resp_rx.await;
        println!("GOT  = {:?}", res);
    });

    // Spawn manager task that processes messages from the channel
    
    // The move keyword is used to **move** ownership of 'rx' into the task
    let manager = tokio::spawn(async move {
        // Establish a connection to the server
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        // Start receiving messages
        while let Some(cmd) = rx.recv().await {
            use Command::*;

            match cmd {
                Get { key, resp } => {
                    let res = client.get(&key).await;
                    // Ignore errors
                    let _ = resp.send(res);
                }
                Set { key, val, resp } => {
                    let res = client.set(&key, val).await;
                    // Ignore errors
                    // send on a oneshot channel will always fail or succeed without any form of waiting.
                    let _ = resp.send(res);
                }
            }
        }
    });

    // Await the join handles 
    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
