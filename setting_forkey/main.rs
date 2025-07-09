use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::ffi::CString;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::Input::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCSTR;
use chrono::Local;
use windows::Win32::Graphics::Gdi::UpdateWindow;

const CLASS_NAME: &str = "Portapad_inputter";

fn main() {
    unsafe {
        let h_instance: HINSTANCE = GetModuleHandleA(None).unwrap().into();

        let class_name_c = CString::new(CLASS_NAME).unwrap();
        let window_name_c = CString::new("Portapad_inputter").unwrap();

        let wc = WNDCLASSA {
            hInstance: h_instance,
            lpszClassName: PCSTR(class_name_c.as_ptr() as _),
            lpfnWndProc: Some(wnd_proc),
            ..zeroed()
        };
        RegisterClassA(&wc);

        let hwnd = CreateWindowExA(
            WINDOW_EX_STYLE::default(),
            PCSTR(class_name_c.as_ptr() as _),
            PCSTR(window_name_c.as_ptr() as _),
            WS_OVERLAPPEDWINDOW & !WS_VISIBLE,
            0,
            0,
            0,
            0,
            Some(HWND(null_mut())),
            Some(HMENU(null_mut())),
            Some(h_instance),
            Some(std::ptr::null_mut::<std::ffi::c_void>()),
        )
        .expect("Failed to create window");
        UpdateWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageA(&mut msg, Some(HWND(null_mut())), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
    }
}

static mut PRESS_LOG: Option<VecDeque<(String, String)>> = None;

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        println!("{}", msg);
        //256,257,258 がキーボードのイベント
        match msg {
            WM_CREATE => {
                PRESS_LOG = Some(VecDeque::with_capacity(3));
                let rid = [RAWINPUTDEVICE {
                    usUsagePage: 0x01,
                    usUsage: 0x06,     // Keyboard
                    dwFlags: RIDEV_INPUTSINK, // ウィンドウがフォーカスを失っても入力を受け取る
                    hwndTarget: hwnd, // イベント受け取り先
                }];

                let result = RegisterRawInputDevices(&rid, std::mem::size_of::<RAWINPUTDEVICE>() as u32);

            }
            WM_INPUT => {
                let mut size = 0u32;
                GetRawInputData(
                    HRAWINPUT(lparam.0 as *mut _),
                    RID_INPUT,
                    None,
                    &mut size,
                    size_of::<RAWINPUTHEADER>() as u32,
                );

                let mut buf = vec![0u8; size as usize];
                GetRawInputData(
                    HRAWINPUT(lparam.0 as *mut _),
                    RID_INPUT,
                    Some(buf.as_mut_ptr() as *mut _),
                    &mut size,
                    size_of::<RAWINPUTHEADER>() as u32,
                );

                let raw = &*(buf.as_ptr() as *const RAWINPUT);
                if raw.header.dwType == RIM_TYPEKEYBOARD.0 {
                    let device_handle = raw.header.hDevice;
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

                    // まずバッファサイズを取得
                    let mut size = 0u32;
                    GetRawInputDeviceInfoA(Some(device_handle), RIDI_DEVICENAME, None, &mut size);

                    // デバイス名取得
                    let mut buffer = vec![0u8; size as usize];
                    GetRawInputDeviceInfoA(Some(device_handle), RIDI_DEVICENAME, Some(buffer.as_mut_ptr() as _), &mut size);

                    let device_name = String::from_utf8_lossy(&buffer);
                    let devname_str = device_name.trim_end_matches(&['\0', '\r', '\n'][..]).to_string();
                    println!("{}", device_name);


                    if let Some(ref mut log) = PRESS_LOG {
                        log.push_back((devname_str, timestamp));

                        if log.len() >= 1 {
                            let _ = save_to_file(log);
                            log.clear();
                            PostQuitMessage(0);
                        }
                    }
                }
            }
            WM_DESTROY => {
                PostQuitMessage(0);
            }
            _ => {}
        }
        DefWindowProcA(hwnd, msg, wparam, lparam)
    }
}

fn save_to_file(log: &VecDeque<(String, String)>) -> std::io::Result<()> {
    println!("writen");
    let mut file = File::create("input_key_num.txt")?;
    for (hid, timestamp) in log {
        writeln!(file, "{} {}", timestamp, hid)?;
    }
    Ok(())
}