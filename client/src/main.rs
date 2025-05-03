mod client_core;
mod api;

use std::error::Error;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::net::windows::named_pipe::{ServerOptions};
use tokio::{net::TcpListener, runtime::Runtime};

use windows::{Win32::Foundation::*, Win32::Storage::FileSystem::*, Win32::System::Pipes::*};

#[derive(Serialize, Deserialize, Debug)]
struct NetworkInfo {
    continent: String,
    country: String,
    owner: String,
    isp: String,
    prov: String,
    city: String,
    district: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Point {
    code: String,
    data: NetworkInfo,
}

fn main() -> Result<(), Box<dyn Error>> {
    let rt = Runtime::new()?;
    let listener2 = rt.block_on(async { TcpListener::bind("127.0.0.1:0").await })?;
    let listener = rt.block_on(async { TcpListener::bind("127.0.0.1:0").await })?;
    rt.block_on(async {
        new_pipes_server(
            listener2.local_addr().unwrap().port(),
            listener.local_addr().unwrap().port(),
        )
        .await
    });
    println!("Client: Exiting synchronous main function.");

    Ok(())
}

const PIPE_NAME: &str = r"\\.\pipe\novoline893";

async fn new_pipes_server(network_port: u16, forward_port: u16) {
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME)
        .unwrap();
    
    let server_task = tokio::spawn(async move {
        server.connect().await.unwrap();
        println!("Pipe client connected.");
        let mut client = server;
        let message = format!("{},{}", network_port, forward_port);
        client.write_all(message.as_bytes()).await.unwrap();
        println!("Sent ports to pipe client: {}", message);
        
        let mut buffer = [0u8; 512];
        loop {
            match client.try_read(&mut buffer) {
                Ok(0) => {
                    println!("Pipe client disconnected.");
                    break;
                }
                Ok(n) => {
                    let received_message = String::from_utf8_lossy(&buffer[..n]);
                    println!("Received from pipe client: {}", received_message);
                    if received_message == "goodjobguy" {
                        println!("magic received. Closing pipe connection.");
                        client.flush().await.unwrap();
                        client.shutdown().await.unwrap();
                        break;
                    }
                }
                Err(_) => {}
            }
        }
    });
    if let Err(e) = server_task.await {
        eprintln!("Pipe server task panicked or was cancelled: {}", e);
    } else {
        println!("Pipe server task completed successfully.");
    }
}
