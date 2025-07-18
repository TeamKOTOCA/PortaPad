use std::ptr::null_mut;
use egui::mutex::Mutex;
use windows::Win32::Foundation::*;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::UI::WindowsAndMessaging::*;
use std::collections::VecDeque;
use std::mem::{size_of, zeroed};
use std::ffi::CString;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::Input::*;
use windows::core::PCSTR;
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, KEYBDINPUT, INPUT_0, VIRTUAL_KEY};
use once_cell::sync::Lazy;
use std::fs;
use std::sync::mpsc::{Sender, Receiver, channel};


static mut KEY_SENDER: Option<Sender<u32>> = None;

// グローバル変数（スレッド間で安全に使う）
static mut HOOK_HANDLE: HHOOK = HHOOK(null_mut());

pub static KEYBOARD_PATH: Lazy<String> = Lazy::new(|| {
    let mut handle = "".to_string();
    let path = dirs::config_dir()
        .expect("APPDATAが取得できませんでした")
        .join(r"Portapad\input_key_num.txt");
    
    if let Ok(contents) = fs::read_to_string(path) {
        handle = contents.trim().to_string();
    }
    handle
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyHandle(pub HANDLE);
unsafe impl Send for KeyHandle {}
unsafe impl Sync for KeyHandle {}

pub static KEYBOARD_HANDLE: Lazy<Mutex<Option<KeyHandle>>> = Lazy::new(|| {
    let handle = unsafe {
        let mut count: u32 = 0;
        if GetRawInputDeviceList(
            None,
            &mut count,
            std::mem::size_of::<RAWINPUTDEVICELIST>() as u32,
        ) == u32::MAX {
            return Mutex::new(None);
        }

        let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
        if GetRawInputDeviceList(
            Option::Some(list.as_mut_ptr()),
            &mut count,
            std::mem::size_of::<RAWINPUTDEVICELIST>() as u32,
        ) == u32::MAX {
            return Mutex::new(None);
        }

        for dev in list {
            let mut name_len: u32 = 0;
            GetRawInputDeviceInfoA(
                Some(dev.hDevice),
                RIDI_DEVICENAME, 
                None, 
                &mut name_len);

            let mut name_buf = vec![0u8; name_len as usize];
            if GetRawInputDeviceInfoA(
                Some(dev.hDevice),
                RIDI_DEVICENAME,
                Some(name_buf.as_mut_ptr() as *mut _),
                &mut name_len,
            ) != u32::MAX {
                let name = String::from_utf8_lossy(&name_buf)
                    .trim_end_matches('\0')
                    .to_string();
                if name.contains(KEYBOARD_PATH.as_str()) {
                    return Mutex::new(Some(KeyHandle(dev.hDevice)));
                }
            }
        }

        Mutex::new(None)
    };

    handle
});


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
                    dwFlags: RIDEV_INPUTSINK | RIDEV_NOLEGACY, // ウィンドウがフォーカスを失っても入力を受け取る
                    hwndTarget: hwnd, // イベント受け取り先
                }];

                unsafe {
                    RegisterRawInputDevices(&rid,  std::mem::size_of::<RAWINPUTDEVICE>() as u32)
                        .expect("RawInput登録失敗");
                }
                return LRESULT(0)
            }
            WM_INPUT => {
                let mut size = 0;
                GetRawInputData(HRAWINPUT(lparam.0 as *mut _), RID_INPUT, None, &mut size, std::mem::size_of::<RAWINPUTHEADER>() as u32);
                let mut buffer = vec![0u8; size as usize];
                GetRawInputData(HRAWINPUT(lparam.0 as *mut _), RID_INPUT, Some(buffer.as_mut_ptr() as *mut _), &mut size, std::mem::size_of::<RAWINPUTHEADER>() as u32);


                let mut select_key = KEYBOARD_HANDLE.lock().unwrap();

                let raw: &RAWINPUT = &*(buffer.as_ptr() as *const RAWINPUT);
                // handle を使って処理
                if raw.header.dwType == RIM_TYPEKEYBOARD.0 {
                    let dev = raw.header.hDevice;
                    if select_key == KeyHandle(dev) {
                        let data = unsafe { raw.data.keyboard };
                        let flags = if data.Flags & (RI_KEY_BREAK as u16) != 0 {
                            KEYEVENTF_KEYUP
                        } else {
                            return LRESULT(0);
                        };

                        let input = INPUT {
                            r#type: INPUT_KEYBOARD,
                            Anonymous: INPUT_0 {
                                ki: KEYBDINPUT {
                                    wVk: VIRTUAL_KEY(data.VKey),
                                    wScan: data.MakeCode,
                                    dwFlags: flags,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        };

                        unsafe {
                            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                        }
                    }
                }
                return LRESULT(1)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return LRESULT(0)
            }
            _ => {
                // 何もせずにデフォルトへ
                return DefWindowProcA(hwnd, msg, wparam, lparam); // ← 最後も LRESULT
            }
        }
        DefWindowProcA(hwnd, msg, wparam, lparam)
    }
}


const CLASS_NAME: &str = "portapadwindow";
#[tokio::main]
async fn main() {
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