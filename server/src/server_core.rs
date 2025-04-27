// 声明这个模块需要使用外部 crates
use futures::try_join;
use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// 异步函数：处理单个客户端连接和后端连接之间的数据转发
async fn handle_server_connection(mut client_stream: TcpStream, backend_addr: SocketAddr) {
    // 连接到后端服务
    let mut backend_stream = match TcpStream::connect(&backend_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Failed to connect to backend {}: {}", backend_addr, e);
            // 关闭客户端连接
            let _ = client_stream.shutdown().await;
            return;
        }
    };
    println!("Connected to backend {}", backend_addr);

    // 分割客户端和后端连接流为读写半部分
    let (mut client_read, mut client_write) = client_stream.split();
    let (mut backend_read, mut backend_write) = backend_stream.split();

    // 创建两个 futures：一个从客户端读并写到后端，另一个从后端读并写到客户端
    let client_to_backend = io::copy(&mut client_read, &mut backend_write);
    let backend_to_client = io::copy(&mut backend_read, &mut client_write);

    // 并发地运行这两个 futures，并在它们都成功完成或任一个失败时返回结果
    match try_join!(client_to_backend, backend_to_client) {
        Ok((client_bytes, backend_bytes)) => {
            println!(
                "Server: Connection closed. {} bytes client->backend, {} bytes backend->client",
                client_bytes, backend_bytes
            );
        }
        Err(e) => {
            eprintln!("Server: Connection closed with error: {}", e);
        }
    }

    // 当任一 copy 操作完成（连接关闭或发生错误），通常连接会适当地关闭。
    // 手动关闭写端也是一个选择，但 copy 通常会处理。
}

// 公共异步函数：运行服务器的主要监听循环
// 这个函数将在 main.rs 中由运行时调用
pub async fn run_server(
    listen_addr: SocketAddr,
    backend_addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    // 绑定到监听地址
    let listener = TcpListener::bind(&listen_addr).await?;
    println!("Server listening on {}", listen_addr);
    println!("Forwarding to backend {}", backend_addr);

    // 循环接受新的连接
    loop {
        // accept 是一个异步操作，等待传入连接
        let (client_stream, client_addr) = listener.accept().await?;
        println!("Server: Accepted connection from {}", client_addr);

        // 为每个新的客户端连接 spawn 一个异步任务
        // 注意：spawn 需要在一个运行时上下文中运行，这个上下文由外部的 block_on 提供
        tokio::spawn(handle_server_connection(client_stream, backend_addr));
    }
}
