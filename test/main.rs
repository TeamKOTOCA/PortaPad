use std::ptr::null_mut;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::WindowsAndMessaging::CallNextHookEx;
use windows::Win32::UI::WindowsAndMessaging::HC_ACTION;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::ffi::CString;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::Input::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCSTR;
use chrono::Local;
use windows::Win32::Graphics::Gdi::UpdateWindow;
use tokio::time::{sleep, Duration};

use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::fs;

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::sync::mpsc::{Sender, Receiver, channel};


static mut KEY_SENDER: Option<Sender<u32>> = None;

// グローバル変数（スレッド間で安全に使う）
static mut HOOK_HANDLE: HHOOK = HHOOK(null_mut());


pub static KEYBOARD_HANDLE: Lazy<String> = Lazy::new(|| {
    let mut handle = "".to_string();
    let path = dirs::config_dir()
        .expect("APPDATAが取得できませんでした")
        .join(r"Portapad\input_key_num.txt");
    
    if let Ok(contents) = fs::read_to_string(path) {
        handle = contents.trim().to_string();
    }
    handle
});

// キーボードフックコールバック
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kbd_struct = *(lparam.0 as *const KBDLLHOOKSTRUCT);

        // キーが押されたとき
        if wparam.0 as u32 == WM_KEYDOWN {
            let vk_code = kbd_struct.vkCode;
            /*
            if vk_code == 0x41 { // 'A'キーを止める
                println!("Aキーを黙殺！");
                return LRESULT(1); // イベントを止める
            }
            */
            while let Ok(vk_code) = rx.recv() {
                    println!("処理対象のキーボード入力: {}", vk_code);
            }
        }
    }

    // 次のフックに処理を渡す
    CallNextHookEx(None, code as i32, wparam, lparam)
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

                    // まずバッファサイズを取得
                    let mut size = 0u32;
                    GetRawInputDeviceInfoA(Some(device_handle), RIDI_DEVICENAME, None, &mut size);

                    // デバイス名取得
                    let mut buffer = vec![0u8; size as usize];
                    GetRawInputDeviceInfoA(Some(device_handle), RIDI_DEVICENAME, Some(buffer.as_mut_ptr() as _), &mut size);

                    let device_name = String::from_utf8_lossy(&buffer);
                    println!("{}", device_name);
                    if device_name.trim_end_matches(&['\0', '\r', '\n'][..]) == KEYBOARD_HANDLE.as_str() {
                        println!("いいねぇ");
                        if let Some(sender) = unsafe { KEY_SENDER.as_ref() } {
                            sender.send(1).ok();
                        }
                    }else{
                        if let Some(sender) = unsafe { KEY_SENDER.as_ref() } {
                            sender.send(0).ok();
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


const CLASS_NAME: &str = "portapadwindow";
#[tokio::main]
async fn main() {
    // Globalhook処理
    let globalhookroop = tokio::spawn(async {
        unsafe {
            // 現在のプロセスのモジュールハンドルを取得
            let h_instance = GetModuleHandleW(None).unwrap().into();

            // キーボードフックをセット
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), Some(h_instance), 0);
            match hook {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("SetWindowsHookExW に失敗しました。");
                    return;
                }
            };
            HOOK_HANDLE = hook.unwrap();

            println!("キーボードフック開始中...");

            // メッセージループ
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, Some(HWND(null_mut())), 0, 0).into() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // 終了時にフックを解除
            UnhookWindowsHookEx(HOOK_HANDLE);
        }
    });

    // rawinput処理
    let rawinputroop = tokio::spawn(async {
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
    });

    // メインスレッドがすぐ終わらないように待つ
    let _ = tokio::join!(globalhookroop, rawinputroop);
}