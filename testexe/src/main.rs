use std::mem::transmute;
use tokio::io::AsyncWriteExt;
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::runtime::Runtime;
use windows::Win32::Foundation::FARPROC;
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::core::PCWSTR;
use windows::core::s;
use windows::core::w;

use once_cell::sync::OnceCell;

static NETWORK_PORT: OnceCell<u16> = OnceCell::new();
static FORWARD_PORT: OnceCell<u16> = OnceCell::new();
const PIPE_NAME: &str = r"\\.\pipe\novoline893";
async fn aaaaaa() {
    let mut client = loop {
        match ClientOptions::new().open(PIPE_NAME) {
            Ok( temp) => { break temp }
            Err(_) => {}
        }
    };

    let mut buffer = [0u8; 512];
    loop {
        match client.try_read(&mut buffer) {
            Ok(0) => {
                // Connection closed by client.
                println!("Pipe client disconnected before sending 'goodguys'.");
                break; // Exit the read loop
            }
            Ok(n) => {
                let message = String::from_utf8_lossy(&buffer[..n]);
                println!("Received from pipe client: {}", message);
                let parts: Vec<&str> = message.split(',').collect();
                if parts.len() == 2 {
                    let network_port: u16 = parts[0].parse().unwrap();
                    let forward_port: u16 = parts[1].parse().unwrap();
                    let _ = NETWORK_PORT.set(network_port);
                    let _ = FORWARD_PORT.set(forward_port);
                    client.write_all("goodjobguy".as_bytes());
                    break;
                }
            }
            Err(_) => {}
        }
    }


}
fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(aaaaaa());

    // 构造DLL路径
    // let wide: PCWSTR = w!("java_native.dll");
    // unsafe {
    //     let hinst: HMODULE = LoadLibraryW(wide).unwrap();
    //     if hinst.0 == std::ptr::null_mut() {
    //         eprintln!("无法加载DLL: {}", wide.to_string().unwrap());
    //         return;
    //     }
    //     let func: FARPROC = GetProcAddress(hinst, s!("fake_entrypoint"));
    //     let fake_entrypoint: extern "C" fn() = transmute(func);
    //     fake_entrypoint();
    //     let _ = FreeLibrary(hinst);
    // }
}
