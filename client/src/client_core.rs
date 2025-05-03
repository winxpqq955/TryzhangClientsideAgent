// 声明这个模块需要使用外部 crates
use futures::try_join;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_kcp::{KcpConfig, KcpStream}; // 移除 UdpSocket，tokio-kcp 会处理

// JWT声明结构体
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // 主题 (通常是用户ID)
    exp: usize,  // 过期时间 (Unix时间戳)
                 // 可以根据需要添加更多字段
}

// 异步函数：处理单个本地连接和代理服务器连接之间的数据转发
async fn handle_client_connection(mut local_stream: TcpStream, server_addr: SocketAddr) {
    // 使用 tokio-kcp 连接到反向代理服务器
    // tokio-kcp 的 connect 会自动处理 UDP socket
    let mut server_stream = match KcpStream::connect(&KcpConfig::default(), server_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!(
                "Failed to connect to proxy server {} via KCP: {}",
                server_addr, e
            );
            // 关闭本地连接
            let _ = local_stream.shutdown().await;
            return;
        }
    };
    println!("Client: Connected to proxy server {} via KCP", server_addr);

    // 生成JWT令牌 (在实际应用中，这个用户ID应该从配置或认证系统获取)
    let token = "your_jwt_token_here";

    // 发送JWT令牌
    if let Err(e) = send_auth_token(&mut server_stream, &token).await {
        eprintln!("Failed to send authentication token: {}", e);
        let _ = local_stream.shutdown().await;
        return;
    }

    // 等待认证响应
    let mut auth_response = [0u8; 1];
    if let Err(e) = server_stream.read_exact(&mut auth_response).await {
        eprintln!("Failed to receive authentication response: {}", e);
        let _ = local_stream.shutdown().await;
        return;
    }

    // 检查认证是否成功
    if auth_response[0] != 1 {
        eprintln!("Authentication failed");
        let _ = local_stream.shutdown().await;
        return;
    }

    println!("Client: Authentication successful");

    // 分割本地 TCP 连接和 KCP 服务器连接流为读写半部分
    // KcpStream 可能不直接支持 split()，需要确认 kcp crate 的 API
    // 假设 KcpStream 实现了 AsyncRead 和 AsyncWrite
    let (mut local_read, mut local_write) = local_stream.split();
    let (mut server_read, mut server_write) = io::split(server_stream); // 使用 tokio::io::split 来分割 KcpStream

    // 创建两个 futures：一个从本地连接读并写到服务器，另一个从服务器读并写到本地连接
    let local_to_server = io::copy(&mut local_read, &mut server_write);
    let server_to_local = io::copy(&mut server_read, &mut local_write);

    // 并发地运行这两个 futures
    match try_join!(local_to_server, server_to_local) {
        Ok((local_bytes, server_bytes)) => {
            println!(
                "Client: KCP Connection closed. {} bytes local->server, {} bytes server->local",
                local_bytes, server_bytes
            );
        }
        Err(e) => {
            eprintln!("Client: KCP Connection closed with error: {}", e);
        }
    }
}

// 发送认证令牌
async fn send_auth_token(stream: &mut KcpStream, token: &str) -> io::Result<()> {
    // 发送令牌长度 (4字节)
    let token_len = token.len() as u32;
    stream.write_all(&token_len.to_be_bytes()).await?;

    // 发送令牌内容
    stream.write_all(token.as_bytes()).await?;

    Ok(())
}

// 公共异步函数：运行客户端的主要监听循环 (监听本地 TCP)
// 这个函数将在 client.rs 中由运行时调用
pub async fn run_client(
    listener: &TcpListener,  // 仍然监听本地 TCP
    server_addr: SocketAddr, // 代理服务器地址 (用于 KCP 连接)
) -> Result<(), Box<dyn Error>> {
    println!(
        "Client listening on {}:{} (TCP)",
        listener.local_addr()?.ip(),
        listener.local_addr()?.port()
    );
    println!(
        "Forwarding connections to proxy server {} via KCP",
        server_addr
    );

    // 循环接受新的本地 TCP 连接
    loop {
        let (local_stream, local_addr) = listener.accept().await?;
        println!("Client: Accepted local TCP connection from {}", local_addr);

        // 为每个新的本地连接 spawn 一个异步任务，使用 KCP 连接到服务器
        tokio::spawn(handle_client_connection(local_stream, server_addr));
    }
}
