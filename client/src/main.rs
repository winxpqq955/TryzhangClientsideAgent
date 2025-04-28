mod client_core;

use std::error::Error;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::windows::named_pipe::ServerOptions;
use tokio::{net::TcpListener, runtime::Runtime};
// 引入 Runtime 类型

use windows::core::w;
use windows::{Win32::Foundation::*, Win32::Storage::FileSystem::*, Win32::System::Pipes::*};

// 这是同步的 main 函数，没有 #[tokio::main] 宏
fn main() -> Result<(), Box<dyn Error>> {
    let rt = Runtime::new()?;

    let listener2 = rt.block_on(async { TcpListener::bind("127.0.0.1:0").await })?;
    let listener = rt.block_on(async { TcpListener::bind("127.0.0.1:0").await })?;
    // rt.block_on(async {
    //     new_pipes_server(
    //         listener2.local_addr().unwrap().port(),
    //         listener.local_addr().unwrap().port(),
    //     )
    //     .await
    // });

    let server_addr = "127.0.0.1:8080"; // 反向代理服务器地址
    let server_addr: SocketAddr = server_addr.parse()?;
    rt.block_on(client_core::run_client(&listener, server_addr))?;

    println!("Client: Exiting synchronous main function.");

    Ok(())
}

const PIPE_NAME: &str = r"\\.\pipe\novoline893";

async fn new_pipes_server(network_port: u16, forward_port: u16) {
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(PIPE_NAME)
        .unwrap();

    // Spawn the server logic as a task.
    let server_task = tokio::spawn(async move {
        // Wait for a client to connect.
        server.connect().await.unwrap();
        println!("Pipe client connected.");

        // Use the connected client instance.
        let mut client = server;

        // Write the ports information.
        let message = format!("{},{}", network_port, forward_port);
        client.write_all(message.as_bytes()).await.unwrap();
        println!("Sent ports to pipe client: {}", message);

        // Loop to read messages from the client.
        let mut buffer = [0u8; 512];
        loop {
            match client.try_read(&mut buffer) {
                Ok(0) => {
                    // Connection closed by client.
                    println!("Pipe client disconnected before sending 'goodguys'.");
                    break; // Exit the read loop
                }
                Ok(n) => {
                    let received_message = String::from_utf8_lossy(&buffer[..n]);
                    println!("Received from pipe client: {}", received_message);
                    if received_message == "goodguys" {
                        println!("'goodguys' received. Closing pipe connection.");
                        // Optional: Flush and shutdown can be added if needed.
                        // client.flush().await.unwrap();
                        // client.shutdown().await.unwrap();
                        break; // Exit the read loop
                    }
                    // If not "goodguys", continue reading.
                }
                Err(e) => {
                    eprintln!("Failed to read from pipe: {}", e);
                    break; // Exit on error
                }
            }
        }
        // The client instance (pipe connection) is dropped here.
        println!("Pipe connection handling finished.");
        // Return a value or () from the task.
    });

    // Wait for the spawned task to complete. This makes `new_pipes_server` block.
    if let Err(e) = server_task.await {
        eprintln!("Pipe server task panicked or was cancelled: {}", e);
    } else {
        println!("Pipe server task completed successfully.");
    }
}
fn pipe_listener(network_port: u16, forward_port: u16) -> Result<(), Box<dyn Error>> {
    unsafe {
        let pipe_name = w!("\\\\.\\pipe\\novoline893"); // 使用宽字符宏定义管道名称
        println!(
            "Creating named pipe: {}",
            pipe_name.to_string().unwrap_or_default()
        );
        // 等待客户端连接
        'connection_loop: loop {
            let pipe_handle = CreateNamedPipeW(
                pipe_name,                                             // 管道名称
                PIPE_ACCESS_DUPLEX,                                    // 读写访问
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT, // 消息类型，消息读取模式，阻塞模式
                PIPE_UNLIMITED_INSTANCES,                              // 最大实例数
                512,                                                   // 输出缓冲区大小
                512,                                                   // 输入缓冲区大小
                0,                                                     // 默认超时时间
                None,                                                  // 默认安全属性
            );

            if pipe_handle == INVALID_HANDLE_VALUE {
                eprintln!("Failed to create named pipe. Error: {:?}", GetLastError());
                return Err(windows::core::Error::from_win32().into());
            }

            println!("Pipe created. Waiting for client connection...");
            let connected = ConnectNamedPipe(pipe_handle, None); // 使用 None 表示同步操作
            if !connected.is_ok() && GetLastError() != ERROR_PIPE_CONNECTED {
                eprintln!("Failed to connect to client. Error: {:?}", GetLastError());
                CloseHandle(pipe_handle)?;
                continue;
            }

            // 向管道写入信息
            let message_string = format!("{},{}", network_port, forward_port);
            let mut bytes_written = 0;
            let write_result = WriteFile(
                pipe_handle,
                Some(message_string.as_bytes()),
                Some(&mut bytes_written),
                None,
            );

            if !write_result.is_ok() {
                eprintln!("Failed to write to pipe. Error: {:?}", GetLastError());
                CloseHandle(pipe_handle)?;
                return Err(windows::core::Error::from_win32().into());
            }

            println!(
                "Message sent to client: {}",
                String::from_utf8_lossy(message_string.as_bytes())
            );
            'msg: loop {
                println!("Waiting Read Msg from Client...");
                // 从管道读取信息
                let mut buffer: [u8; 512] = [0; 512];
                let mut bytes_read = 0;
                let read_result =
                    ReadFile(pipe_handle, Some(&mut buffer), Some(&mut bytes_read), None);
                if read_result.is_ok() {
                    if bytes_read > 0 {
                        let message = String::from_utf8_lossy(&buffer[..bytes_read as usize]);
                        println!("Message received from client: {}", message);
                        if message == "goodguys" {
                            // 断开连接并关闭句柄
                            println!("Disconnecting pipe...");
                            DisconnectNamedPipe(pipe_handle)?;
                            println!("Closing pipe handle...");
                            CloseHandle(pipe_handle)?;
                            println!("Pipe closed.");
                            break 'connection_loop;
                        }
                    } else {
                        println!("No message received or pipe closed.");
                    }
                }
                break 'msg;
            }
        }
    }
    Ok(())
}
