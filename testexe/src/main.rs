use std::mem::transmute;

use windows::Win32::Foundation::FARPROC;
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::core::PCWSTR;
use windows::core::s;
use windows::core::w;

fn main() {
    // 构造DLL路径
    let wide: PCWSTR = w!("java_native.dll");
    unsafe {
        // 加载DLL
        let hinst: HMODULE = LoadLibraryW(wide).unwrap();
        if hinst.0 == std::ptr::null_mut() {
            eprintln!("无法加载DLL: {}", wide.to_string().unwrap());
            return;
        }
        let func: FARPROC = GetProcAddress(hinst, s!("fake_entrypoint"));
        let fake_entrypoint: extern "C" fn() = transmute(func);
        fake_entrypoint();
        let _ = FreeLibrary(hinst);
    }
}
