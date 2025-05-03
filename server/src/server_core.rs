// 声明这个模块需要使用外部 crates
use futures::try_join;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_kcp::{KcpConfig, KcpListener, KcpStream}; // 移除 UdpSocket，tokio-kcp 会处理

// JWT声明结构体
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // 主题 (通常是用户ID)
    exp: usize,  // 过期时间 (Unix时间戳)
                 // 可以根据需要添加更多字段
}

// 定义JWT验证错误
#[derive(Debug)]
enum AuthError {
    InvalidToken,
    TokenExpired,
    IoError(io::Error),
}

impl From<io::Error> for AuthError {
    fn from(err: io::Error) -> Self {
        AuthError::IoError(err)
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        AuthError::InvalidToken
    }
}

// 验证JWT令牌
fn validate_token(token: &str) -> Result<Claims, AuthError> {
    // 在实际应用中，这个密钥应该从安全的配置中获取
    let secret = b"your_secret_key";

    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;

    Ok(token_data.claims)
}

// 异步函数：处理单个 KCP 客户端连接和 TCP 后端连接之间的数据转发
async fn handle_server_connection(mut client_stream: KcpStream, backend_addr: SocketAddr) {
    // 首先读取并验证JWT令牌
    let auth_result = authenticate_client(&mut client_stream).await;

    match auth_result {
        Ok(claims) => {
            println!("Server: Authentication successful for user: {}", claims.sub);
            // 继续处理连接
            process_connection(client_stream, backend_addr).await;
        }
        Err(e) => {
            match e {
                AuthError::InvalidToken => eprintln!("Server: Invalid JWT token"),
                AuthError::TokenExpired => eprintln!("Server: JWT token expired"),
                AuthError::IoError(e) => eprintln!("Server: IO error during authentication: {}", e),
            }
            // 不处理连接，函数返回后连接会被关闭
        }
    }
}

// 验证客户端身份
async fn authenticate_client(client_stream: &mut KcpStream) -> Result<Claims, AuthError> {
    // 读取JWT令牌长度 (4字节)
    let mut len_bytes = [0u8; 4];
    client_stream.read_exact(&mut len_bytes).await?;
    let token_len = u32::from_be_bytes(len_bytes) as usize;

    // 读取JWT令牌
    let mut token_bytes = vec![0u8; token_len];
    client_stream.read_exact(&mut token_bytes).await?;

    // 转换为字符串
    let token = String::from_utf8(token_bytes).map_err(|_| AuthError::InvalidToken)?;

    // 验证令牌
    let claims = validate_token(&token)?;

    // 发送验证成功响应
    client_stream.write_all(&[1u8]).await?;

    Ok(claims)
}

// 处理已验证的连接
async fn process_connection(client_stream: KcpStream, backend_addr: SocketAddr) {
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
