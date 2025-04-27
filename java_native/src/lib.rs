use once_cell::sync::OnceCell;
use windows::{
    Win32::Foundation::*, Win32::Storage::FileSystem::*, Win32::System::Pipes::*, core::*,
};

static NETWORK_PORT: OnceCell<u16> = OnceCell::new();
static FORWARD_PORT: OnceCell<u16> = OnceCell::new();

fn pipe() -> Result<Vec<u16>> {
    let pipe_name = w!("\\\\.\\pipe\\novoline893");
    unsafe {
        println!(
            "Connecting named pipe: {}",
            pipe_name.to_string().unwrap_or_default()
        );
        loop {
            // 尝试连接到命名管道
            // 注意：这里使用 CreateFileW 而不是 CreateNamedPipeW
            let pipe_handle = CreateFileW(
                pipe_name,                          // 管道名称
                GENERIC_READ.0 | GENERIC_WRITE.0,   // 请求读写访问权限
                FILE_SHARE_MODE(0),                 // 不共享
                None,                               // 默认安全属性
                OPEN_EXISTING,                      // 打开已存在的管道
                FILE_FLAGS_AND_ATTRIBUTES(0),       // 默认文件属性
                Some(HANDLE(std::ptr::null_mut())), // 无模板文件
            );

            if pipe_handle == Ok(INVALID_HANDLE_VALUE) {
                eprintln!(
                    "Failed to connect to named pipe. Error: {:?}",
                    GetLastError()
                );
                // 如果服务器尚未启动，客户端可能会收到 ERROR_PIPE_BUSY
                // 可以添加重试逻辑
                return Err(Error::from_win32());
            }
            if pipe_handle.is_err() {
                continue;
            }
            println!("Connected to pipe.");

            // 设置管道为消息读取模式 (可选，但有助于确保行为一致)
            let mut mode = PIPE_READMODE_MESSAGE;
            let set_mode_result = SetNamedPipeHandleState(
                *pipe_handle.as_ref().unwrap(), // pipe_handle is Ok here
                Some(&mut mode as *mut _),
                None, // 不设置最大收集计数
                None, // 不设置收集数据超时
            );

            if !set_mode_result.is_ok() {
                eprintln!("Failed to set pipe read mode. Error: {:?}", GetLastError());
                return Err(Error::from_win32());
            }

            // 从管道读取信息
            let mut buffer: [u8; 512] = [0; 512];
            let mut bytes_read = 0;

            println!("Reading from pipe...");
            let read_result = ReadFile(
                *pipe_handle.as_ref().unwrap(),
                Some(&mut buffer),
                Some(&mut bytes_read),
                None,
            );
            if read_result.is_ok() {
                if bytes_read > 0 {
                    let message = String::from_utf8_lossy(&buffer[..bytes_read as usize]);
                    println!("Message received from server: {}", message);
                    let parts: Vec<&str> = message.split(',').collect();
                    if parts.len() == 2 {
                        let network_port: u16 = parts[0].parse().unwrap();
                        let forward_port: u16 = parts[1].parse().unwrap();
                        let _ = NETWORK_PORT.set(network_port);
                        let _ = FORWARD_PORT.set(forward_port);
                        rev_server("goodguys".as_bytes(), &pipe_handle.as_ref().unwrap());
                        return Ok(vec![network_port, forward_port]);
                    }
                } else {
                    println!("No message received or pipe closed.");
                }
            }
            CloseHandle(*pipe_handle.as_ref().unwrap())?;
            println!("Pipe handle closed.");
        }
    }
}

fn rev_server(message2: &[u8], pipe_handle: &HANDLE) {
    unsafe {
        let mut bytes_written = 0;
        let _ = WriteFile(
            *pipe_handle, // 管道句柄
            Some(message2),
            Some(&mut bytes_written),
            None,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fake_entrypoint() {
    let useful: Vec<u16> = pipe().unwrap();
    println!("{}", useful[0]);
    println!("{}", useful[1]);
}
