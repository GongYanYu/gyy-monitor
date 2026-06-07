// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  #[cfg(all(target_os = "windows", not(debug_assertions)))]
  check_and_elevate();

  app_lib::run();
}

#[cfg(all(target_os = "windows", not(debug_assertions)))]
fn check_and_elevate() {
    use std::env;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows_sys::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    unsafe {
        if IsUserAnAdmin() == 0 {
            println!("[UAC] Process is not running as admin. Requesting elevation...");
            if let (Ok(current_exe), Ok(current_dir)) = (env::current_exe(), env::current_dir()) {
                let current_exe_w: Vec<u16> = current_exe
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let current_dir_w: Vec<u16> = current_dir
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let args: Vec<String> = env::args().skip(1).collect();
                let args_str = args.join(" ");
                let args_w: Vec<u16> = std::ffi::OsString::from(args_str)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let operation_w: Vec<u16> = "runas"
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();

                println!("[UAC] Launching elevated instance: {:?} in {:?}", current_exe, current_dir);
                let instance = ShellExecuteW(
                    0,
                    operation_w.as_ptr(),
                    current_exe_w.as_ptr(),
                    if args.is_empty() { ptr::null() } else { args_w.as_ptr() },
                    current_dir_w.as_ptr(),
                    SW_SHOWNORMAL,
                );
                println!("[UAC] ShellExecuteW result code: {}", instance);

                if instance > 32 {
                    println!("[UAC] Elevation requested successfully. Exiting current process.");
                    std::process::exit(0);
                } else {
                    println!("[UAC] ShellExecuteW failed!");
                }
            }
        } else {
            println!("[UAC] Process is already running as admin.");
        }
    }
}
