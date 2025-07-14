use std::ptr::null_mut;
use std::thread;
use std::sync::Mutex;
use std::time::Duration;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::WindowsAndMessaging::CallNextHookEx;
use windows::Win32::UI::WindowsAndMessaging::HC_ACTION;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

// グローバル変数（スレッド間で安全に使う）
static mut HOOK_HANDLE: HHOOK = HHOOK(null_mut());

// キーボードフックコールバック
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kbd_struct = *(lparam.0 as *const KBDLLHOOKSTRUCT);

        // キーが押されたとき
        if wparam.0 as u32 == WM_KEYDOWN {
            let vk_code = kbd_struct.vkCode;
            if vk_code == 0x41 { // 'A'キーを止める
                println!("Aキーを黙殺！");
                return LRESULT(1); // イベントを止める
            }
            println!("キーコード: {}", vk_code);
        }
    }

    // 次のフックに処理を渡す
    CallNextHookEx(None, code as i32, wparam, lparam)
}

fn main() {
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
}