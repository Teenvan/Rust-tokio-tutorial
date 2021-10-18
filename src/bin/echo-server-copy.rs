use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6142").await.unwrap();

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
        //     // Copy data here
        //     let (mut rd, mut wr) = socket.split();

        //     if io::copy(&mut rd, &mut wr).await.is_err() {
        //         eprintln!("failed to copy");
        //     }

        // Manually copying

        // The strategy is to read some data from the socket into a 
        // buffer then write the contents of the buffer back to the socket.
        let mut buf = vec![0; 1024];
        
        loop {
            match socket.read(&mut buf).await {
                // Return value of ok(0) signifies that the remote has closed.
                Ok(0) => return,
                Ok(n) => {
                    // Copy the data back to socket
                    if socket.write_all(&buf[..n]).await.is_err() {
                        // Unexpected socket error. There isn't much we can
                        // do here so just stop processing.
                        return;
                    }
                }
                Err(_) => {
                    // Unexpected socket error. There isn't much we can
                    // do here so just stop processing.
                    return;
                }
            }
        }

        });
        
        
    }
}