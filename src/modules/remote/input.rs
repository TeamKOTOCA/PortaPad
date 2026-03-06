use super::config::CONFIG;
use crate::modules::certification;
use enigo::{Button, Direction, Enigo, Key, Keyboard, Mouse};
use once_cell::sync::Lazy;
use std::{
    env,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};
use tokio::sync::Mutex;
use webrtc::data_channel::RTCDataChannel;

pub static IS_CERTED: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

#[derive(Clone)]
pub struct InputHandler {
    enigo: Arc<Mutex<Enigo>>,
    left_mouse_down: Arc<Mutex<bool>>,
    right_mouse_down: Arc<Mutex<bool>>,
    dc_for_send: Arc<RTCDataChannel>,
}

impl InputHandler {
    pub fn new(
        enigo: Arc<Mutex<Enigo>>,
        left_mouse_down: Arc<Mutex<bool>>,
        right_mouse_down: Arc<Mutex<bool>>,
        dc_for_send: Arc<RTCDataChannel>,
    ) -> Self {
        Self {
            enigo,
            left_mouse_down,
            right_mouse_down,
            dc_for_send,
        }
    }

    pub async fn handle_message(&self, msg_data: &[u8]) {
        let text = String::from_utf8_lossy(msg_data);
        let first_two: String = text.chars().take(2).collect();
        let no_first: String = text.chars().skip(2).collect();

        if IS_CERTED.load(Ordering::Relaxed) {
            let mut enigo = self.enigo.lock().await;
            match first_two.as_str() {
                "pg" => {
                    // プレイヤー操作が入る予定
                }
                "mb" => match no_first.as_str() {
                    "0" => {
                        if let Err(e) = enigo.button(Button::Left, Direction::Click) {
                            eprintln!("Left mouse click error: {:?}", e);
                        }
                    }
                    "1" => {
                        if let Err(e) = enigo.button(Button::Right, Direction::Click) {
                            eprintln!("Right mouse click error: {:?}", e);
                        }
                    }
                    _ => eprintln!("Unknown mouse button action: {}", no_first),
                },
                "mm" => {
                    let parts: Vec<&str> = no_first.split(',').collect();
                    let part_x: i32 = parts
                        .get(0)
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(0)
                        * 3;
                    let part_y: i32 = parts
                        .get(1)
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(0)
                        * 3;
                    if let Err(e) = enigo.move_mouse(part_x, part_y, enigo::Coordinate::Rel) {
                        eprintln!("Mouse move relative error: {:?}", e);
                    }
                }
                "mp" => {
                    let parts: Vec<&str> = no_first.split(',').collect();
                    let part_x_int: i32 = parts
                        .get(0)
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0) as i32;
                    let part_y_int: i32 = parts
                        .get(1)
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0) as i32;
                    if let Err(e) = enigo.move_mouse(part_x_int, part_y_int, enigo::Coordinate::Abs)
                    {
                        eprintln!("Mouse move to error: {:?}", e);
                    }
                }
                "md" => {
                    if let Err(e) = enigo.button(Button::Left, Direction::Press) {
                        eprintln!("Mouse down error: {:?}", e);
                    }
                    let parts: Vec<&str> = no_first.split(',').collect();
                    let part_x: i32 = parts
                        .get(0)
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(0)
                        * 3;
                    let part_y: i32 = parts
                        .get(1)
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(0)
                        * 3;
                    if let Err(e) = enigo.move_mouse(part_x, part_y, enigo::Coordinate::Rel) {
                        eprintln!("Mouse drag move error: {:?}", e);
                    }
                }
                "ms" => {
                    let parts: Vec<&str> = no_first.split(',').collect();
                    let part_x_int: i32 = parts
                        .get(0)
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(0)
                        / 6;
                    let part_y_int: i32 = parts
                        .get(1)
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(0)
                        / 6;
                    if let Err(e) = enigo.scroll(part_x_int, enigo::Axis::Horizontal) {
                        eprintln!("Horizontal scroll error: {:?}", e);
                    }
                    if let Err(e) = enigo.scroll(part_y_int, enigo::Axis::Vertical) {
                        eprintln!("Vertical scroll error: {:?}", e);
                    }
                }
                "mu" => {
                    let mut locked_left = self.left_mouse_down.lock().await;
                    let mut locked_right = self.right_mouse_down.lock().await;
                    if *locked_left {
                        println!("Left mouse up");
                        if let Err(e) = enigo.button(Button::Left, Direction::Release) {
                            eprintln!("Left mouse up error: {:?}", e);
                        }
                        *locked_left = false;
                    }
                    if *locked_right {
                        println!("Right mouse up");
                        if let Err(e) = enigo.button(Button::Right, Direction::Release) {
                            eprintln!("Right mouse up error: {:?}", e);
                        }
                        *locked_right = false;
                    }
                }
                "kp" => {
                    let key = string_to_key(&no_first);
                    if let Err(e) = enigo.key(key, Direction::Click) {
                        eprintln!("Key press error: {:?}", e);
                    }
                }
                "ku" => {
                    let key = string_to_key(&no_first);
                    if let Err(e) = enigo.key(key, Direction::Release) {
                        eprintln!("Key up error: {:?}", e);
                    }
                }
                "kd" => {
                    let key = string_to_key(&no_first);
                    if let Err(e) = enigo.key(key, Direction::Press) {
                        eprintln!("Key down error: {:?}", e);
                    }
                }
                _ => {
                    eprintln!("Unknown command prefix: {}", first_two);
                }
            }
        } else if first_two == "cc" {
            let _ = Command::new("taskkill")
                .args(["/IM", "certqr.exe", "/F"])
                .output();
        } else {
            match certification::certification(
                no_first,
                CONFIG.privatekey.clone(),
                CONFIG.publickey.clone(),
                CONFIG.pc_code.clone(),
            ) {
                Ok(()) => {
                    println!("認証に成功しました");
                    IS_CERTED.store(true, Ordering::Relaxed);
                }
                Err(_) => {
                    eprintln!("認証に失敗しました");
                    if let Err(e) = self.dc_for_send.send_text("cb".to_string()).await {
                        eprintln!("DataChannel send error: {:?}", e);
                    }
                    self.spawn_certqr();
                }
            }
        }
    }

    fn spawn_certqr(&self) {
        tokio::spawn(async move {
            let exe_path = env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join("certqr.exe");
            let mut child = Command::new(exe_path)
                .spawn()
                .expect("certqr が起動できません");
            thread::spawn(move || {
                let _ = child.wait();
            });
        });
    }
}

fn string_to_key(s: &str) -> Key {
    match s {
        "Enter" | "Return" => Key::Return,
        "Backspace" => Key::Backspace,
        "Tab" => Key::Tab,
        "Escape" => Key::Escape,
        "Space" => Key::Space,
        "CapsLock" => Key::CapsLock,
        "Shift" => Key::Shift,
        "LShift" => Key::LShift,
        "RShift" => Key::RShift,
        "Control" => Key::Control,
        "LControl" => Key::LControl,
        "RControl" => Key::RControl,
        "Alt" => Key::Alt,
        "LAlt" => Key::Alt,
        "RAlt" => Key::Alt,
        "Meta" => Key::Meta,
        "LMeta" => Key::Meta,
        "RMeta" => Key::Meta,
        "Kana" => Key::Kana,
        "Convert" => Key::Convert,
        "NonConvert" => Key::NonConvert,
        "HanZen" => Key::Kanji,
        "UpArrow" => Key::UpArrow,
        "DownArrow" => Key::DownArrow,
        "LeftArrow" => Key::LeftArrow,
        "RightArrow" => Key::RightArrow,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "Numpad0" => Key::Numpad0,
        "Numpad1" => Key::Numpad1,
        "Numpad2" => Key::Numpad2,
        "Numpad3" => Key::Numpad3,
        "Numpad4" => Key::Numpad4,
        "Numpad5" => Key::Numpad5,
        "Numpad6" => Key::Numpad6,
        "Numpad7" => Key::Numpad7,
        "Numpad8" => Key::Numpad8,
        "Numpad9" => Key::Numpad9,
        "Add" => Key::Add,
        "Subtract" | "-" => Key::Subtract,
        "Multiply" | "*" => Key::Multiply,
        "Divide" | "/" => Key::Divide,
        "Decimal" | "." => Key::Decimal,
        "PrintScr" => Key::PrintScr,
        "Pause" => Key::Pause,
        "Delete" => Key::Delete,
        "Insert" => Key::Insert,
        "Home" => Key::Home,
        "End" => Key::End,
        "NumLock" => Key::Numlock,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "VolumeUp" => Key::VolumeUp,
        "VolumeDown" => Key::VolumeDown,
        "VolumeMute" => Key::VolumeMute,
        "MediaPlayPause" => Key::MediaPlayPause,
        "MediaNextTrack" => Key::MediaNextTrack,
        "MediaPrevTrack" => Key::MediaPrevTrack,
        s if s.len() == 1 => {
            if let Some(ch) = s.chars().next() {
                Key::Unicode(ch)
            } else {
                eprintln!("不明なキー: {:?}", s);
                Key::Space
            }
        }
        _ => {
            eprintln!("不明なキー名: {}", s);
            Key::Space
        }
    }
}
