// 声明使用 server_core 模块
mod server_core;

use std::error::Error;
use std::net::SocketAddr;
use tokio::runtime::Runtime;

fn main() -> Result<(), Box<dyn Error>> {
    let listen_addr = "0.0.0.0:19132";
    let backend_addr = "127.0.0.1:25565";

    let listen_addr: SocketAddr = listen_addr.parse()?;
    let backend_addr: SocketAddr = backend_addr.parse()?;

    let rt = Runtime::new()?;
    println!("Server: Tokio runtime created.");

    rt.block_on(server_core::run_server(listen_addr, backend_addr))?;

    Ok(())
}
