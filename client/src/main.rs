mod client_core;

use std::error::Error;
use std::net::SocketAddr;
use tokio::{net::TcpListener, runtime::Runtime}; // 引入 Runtime 类型

use windows::{
    Win32::Foundation::*, Win32::Storage::FileSystem::*, Win32::System::Pipes::*, core::*,
};

// 这是同步的 main 函数，没有 #[tokio::main] 宏
fn main() -> std::result::Result<(), Box<dyn Error>> {
    let rt = Runtime::new()?;

    let listener2 = rt.block_on(async { TcpListener::bind("127.0.0.1:0").await })?;
    let listener = rt.block_on(async { TcpListener::bind("127.0.0.1:0").await })?;
    let _ = pipe_listener(
        listener2.local_addr()?.port(),
        listener.local_addr()?.port(),
    );
    // 从环境变量或命令行参数获取代理服务器地址
    let server_addr = "127.0.0.1:8080"; // 反向代理服务器地址
    let server_addr: SocketAddr = server_addr.parse()?;
    rt.block_on(client_core::run_client(&listener, server_addr))?;

    println!("Client: Exiting synchronous main function.");

    Ok(())
}

fn pipe_listener(network_port: u16, forward_port: u16) -> Result<()> {
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
