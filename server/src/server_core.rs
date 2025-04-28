// 声明这个模块需要使用外部 crates
use futures::try_join;
use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{self};
use tokio::net::TcpStream;
use tokio_kcp::{KcpConfig, KcpListener, KcpStream}; // 移除 UdpSocket，tokio-kcp 会处理

// 异步函数：处理单个 KCP 客户端连接和 TCP 后端连接之间的数据转发
async fn handle_server_connection(client_stream: KcpStream, backend_addr: SocketAddr) {
    // 连接到后端服务 (仍然使用 TCP)
    let mut backend_stream = match TcpStream::connect(&backend_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Failed to connect to backend {}: {}", backend_addr, e);
            // KcpStream 可能没有 shutdown，需要查阅 API，这里假设它会自动清理
            return;
        }
    };
    println!("Server: Connected to backend {} via TCP", backend_addr);

    // 分割 KCP 客户端连接和 TCP 后端连接流为读写半部分
    // 假设 KcpStream 实现了 AsyncRead/AsyncWrite，可以使用 io::split
    let (mut client_read, mut client_write) = io::split(client_stream); // tokio_kcp::KcpStream 支持 split
    let (mut backend_read, mut backend_write) = backend_stream.split();

    // 创建两个 futures：一个从 KCP 客户端读并写到 TCP 后端，另一个从 TCP 后端读并写到 KCP 客户端
    let client_to_backend = io::copy(&mut client_read, &mut backend_write);
    let backend_to_client = io::copy(&mut backend_read, &mut client_write);

    // 并发地运行这两个 futures
    match try_join!(client_to_backend, backend_to_client) {
        Ok((client_bytes, backend_bytes)) => {
            println!(
                "Server: KCP Connection closed. {} bytes client->backend, {} bytes backend->client",
                client_bytes, backend_bytes
            );
        }
        Err(e) => {
            eprintln!("Server: KCP Connection closed with error: {}", e);
        }
    }
}

// 公共异步函数：运行服务器的主要监听循环 (监听 KCP)
// 这个函数将在 main.rs 中由运行时调用
pub async fn run_server(
    listen_addr: SocketAddr,  // 服务器监听地址 (用于 KCP)
    backend_addr: SocketAddr, // 后端服务地址 (用于 TCP)
) -> Result<(), Box<dyn Error>> {
    println!("Server listening on {} (KCP)", listen_addr);
    println!("Forwarding to backend {} (TCP)", backend_addr);

    // 创建 KCP 监听器
    // tokio-kcp 的 bind 会自动处理 UDP socket
    let mut listener = KcpListener::bind(KcpConfig::default(), listen_addr).await?;

    // 循环接受新的 KCP 连接
    loop {
        // accept 是一个异步操作，等待传入的 KCP 连接
        // 返回的可能是 (KcpStream, SocketAddr)，具体取决于 kcp crate API
        let (client_stream, client_addr) = listener.accept().await?;
        println!("Server: Accepted KCP connection from {}", client_addr);

        // 为每个新的 KCP 客户端连接 spawn 一个异步任务
        tokio::spawn(handle_server_connection(client_stream, backend_addr));
    }
}
