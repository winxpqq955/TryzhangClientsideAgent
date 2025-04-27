// 声明这个模块需要使用外部 crates
use futures::try_join;
use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// 异步函数：处理单个本地连接和代理服务器连接之间的数据转发
async fn handle_client_connection(mut local_stream: TcpStream, server_addr: SocketAddr) {
    // 连接到反向代理服务器
    let mut server_stream = match TcpStream::connect(&server_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Failed to connect to proxy server {}: {}", server_addr, e);
            // 关闭本地连接
            let _ = local_stream.shutdown().await;
            return;
        }
    };
    println!("Client: Connected to proxy server {}", server_addr);

    // 分割本地连接和服务器连接流为读写半部分
    let (mut local_read, mut local_write) = local_stream.split();
    let (mut server_read, mut server_write) = server_stream.split();

    // 创建两个 futures：一个从本地连接读并写到服务器，另一个从服务器读并写到本地连接
    let local_to_server = io::copy(&mut local_read, &mut server_write);
    let server_to_local = io::copy(&mut server_read, &mut local_write);

    // 并发地运行这两个 futures，并在它们都成功完成或任一个失败时返回结果
    match try_join!(local_to_server, server_to_local) {
        Ok((local_bytes, server_bytes)) => {
            println!(
                "Client: Connection closed. {} bytes local->server, {} bytes server->local",
                local_bytes, server_bytes
            );
        }
        Err(e) => {
            eprintln!("Client: Connection closed with error: {}", e);
        }
    }
}

// 公共异步函数：运行客户端的主要监听循环
// 这个函数将在 client.rs 中由运行时调用
pub async fn run_client(
    listener: &TcpListener,
    server_addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    // 绑定到本地监听地址
    println!(
        "Client listening on {}:{}",
        listener.local_addr()?.ip(),
        listener.local_addr()?.port()
    );
    println!("Forwarding connections to proxy server {}", server_addr);

    // 循环接受新的本地连接
    loop {
        // accept 是一个异步操作，等待传入连接
        let (local_stream, local_addr) = listener.accept().await?;
        println!("Client: Accepted local connection from {}", local_addr);

        // 为每个新的本地连接 spawn 一个异步任务
        // 注意：spawn 需要在一个运行时上下文中运行，这个上下文由外部的 block_on 提供
        tokio::spawn(handle_client_connection(local_stream, server_addr));
    }
}
