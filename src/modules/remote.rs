use super::certification;

use enigo::{Enigo, Key, Keyboard, Mouse, Button, Settings, Direction};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use futures_util::{SinkExt, StreamExt};
use serde_json;
use serde::Deserialize;
use serde::Serialize;
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use notify_rust::Notification;
use winapi::um::winuser;
use std::sync::LazyLock;
use tokio::signal;
use std::{env, fs, path::PathBuf};
use tokio::time::{interval, Duration};

//認証しているかを保持する変数。false = 未認証、true = 認証済み（操作処理を受け付ける）
static mut IsCerted: bool = false;

#[derive(Serialize)]
struct IceCandidateMsg {
    candidate: String,
    sdpMid: Option<String>,
    sdpMLineIndex: Option<u16>,
}

//config.toml(設定ファイル)の型
#[derive(Deserialize, Debug)]
struct Config {
    sigserver: String,
    sec_sigserver: String,
    pc_code: String,
    privatekey: String,
    publickey: String,
}


#[derive(Deserialize, Debug)]  // JSON用の構造体
struct SigMessage {
    mtype: String,
    fromhost: String,
    body: serde_json::Value
}

#[derive(Deserialize, Debug)]  // JSON用の構造体
struct SigMessageSdp {
    mtype: String,
    fromhost: String,
    body: RTCSessionDescription
}
#[derive(Serialize)]
struct AnswerSigMessage {
    mtype: String,
    tohost: String,
    body: String,
}

#[derive(Serialize)]
struct IceCandidateInit {
    candidate: String,
    sdpMid: Option<String>,
    sdpMLineIndex: Option<u16>,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config_path = get_config_path();
    let config_str = fs::read_to_string(&config_path)
        // エラーメッセージを改善し、どのファイルが読み込めなかったかを示すようにしています
        .expect(&format!("設定ファイルの読み込みに失敗しました: {:?}", config_path));
    let setting_config: Config = toml::from_str(&config_str)
        .expect("TOML形式の設定ファイルのパースに失敗しました");
    setting_config
});

fn get_config_path() -> PathBuf {
    let mut path = env::var_os("APPDATA")
        .map(PathBuf::from)
        .expect("APPDATAが取得できませんでした");
    path.push("portapad");
    fs::create_dir_all(&path).expect("フォルダ作成失敗");
    path.push("config.toml");
    path
}

pub async fn remote_main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio::sync::Notify;

    let notify = Arc::new(Notify::new());
    let notify_for_dc = notify.clone();

    let (tx, mut rx) = mpsc::channel::<(i32, i32)>(100);

    //MediaEngine: 音声/映像のコーデック設定
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;

    //APIBuilderでWebRTCインスタンスを作成
    let api = APIBuilder::new()
        .with_media_engine(m)
        .build();

    //ICEサーバー設定
    let config = RTCConfiguration {
        ice_servers: vec![],
        ..Default::default()
    };

    //fromhost共有用
    let fromhost_shared = Arc::new(Mutex::new(None::<String>));

    // WebSocket接続開始
    let ws_stream_result = connect_async("wss://".to_string() + &CONFIG.sigserver).await;
    let (ws_stream, _) = match ws_stream_result {
        Ok(result) => result,
        Err(e) => {
            eprintln!("WebSocket接続に失敗しました: {:?}", e);
            let fallback_ws_stream_result = connect_async("wss://".to_string() + &CONFIG.sec_sigserver).await;
            match fallback_ws_stream_result {
                Ok(result) => {
                    println!("フォールバックサーバーへのWebSocket接続に成功しました。");
                    result
                },
                Err(fallback_e) => {
                    eprintln!("フォールバックWebSocket接続にも失敗しました: {:?}", fallback_e);
                    return Err(fallback_e.into());
                }
            }
        }
    };

    let (mut write, mut read) = ws_stream.split();

    write.send(Message::Text("host".to_string())).await?;

    // PeerConnection作成
    let peer_connection = match api.new_peer_connection(config).await {
        Ok(pc) => pc,
        Err(e) => {
            eprintln!("エラー: {:?}", e);
            return Err(e.into());
        }
    };
    
    let write = Arc::new(Mutex::new(write));
    let write_clone = Arc::clone(&write);
    let fromhost_clone = Arc::clone(&fromhost_shared);
    let write_for_close = write.clone();

    let write_clone_for_ping = write.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let ping_msg = Message::Ping(b"heartbeat".to_vec());
            if let Err(e) = write_clone_for_ping.lock().await.send(ping_msg).await {
                eprintln!("Ping送信エラー: {:?}", e);
                break;
            }
            println!("Ping送信！");
        }
    });


    peer_connection.on_ice_candidate(Box::new(move |candidate| {
    let write = write_clone.clone();
    let fromhost = Arc::clone(&fromhost_clone);
    Box::pin(async move {
        if let Some(c) = candidate {
            // webrtc-rs の to_json() を使う
            let c_json = match c.to_json() {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("ICE to_json エラー: {:?}", e);
                    return;
                }
            };

            let msg = IceCandidateMsg {
                candidate: c_json.candidate,
                sdpMid: c_json.sdp_mid,
                sdpMLineIndex: c_json.sdp_mline_index.map(|v| v as u16), // ← u16へ変換
            };

            let body = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("ICE候補シリアライズ失敗: {:?}", e);
                    return;
                }
            };

            let tohost = {
                let fromhost_guard = fromhost.lock().await;
                fromhost_guard.clone().unwrap_or_else(|| "unknown".to_string())
            };

            let reply = AnswerSigMessage {
                mtype: "ice".to_string(),
                tohost,
                body,
            };

            let json = match serde_json::to_string(&reply) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("WebSocketメッセージ生成失敗: {:?}", e);
                    return;
                }
            };

            if let Err(e) = write.lock().await.send(Message::Text(json)).await {
                eprintln!("送信失敗: {:?}", e);
            }
        }
    })
    
}));

    let left_m= Arc::new(Mutex::new(false));
    let right_m= Arc::new(Mutex::new(false));
    let left_m_m = left_m.clone();
    let right_m_m = right_m.clone();

    // Enigoインスタンスを一度だけ作成し、ArcとMutexでラップする
    let enigo_mutex = Arc::new(Mutex::new(Enigo::new(&Settings::default()).unwrap()));

    peer_connection.on_data_channel(Box::new(move |dc| {
        println!("DataChannel received: {}", dc.label());
        let right_m = Arc::clone(&right_m_m);
        let left_m = Arc::clone(&left_m_m);
        let notify = notify_for_dc.clone();
        let write = write_for_close.clone();
        let enigo_mutex = Arc::clone(&enigo_mutex); // MutexをArcで共有する

        Box::pin(async move {
            // クローンして move で使う
            let write = write.clone();
            let dc_clone = Arc::clone(&dc);
            let dc_clone_for_ms = Arc::clone(&dc);
            let dc_for_send = Arc::clone(&dc);
            let right_m = Arc::clone(&right_m);
            let left_m = Arc::clone(&left_m);
            dc.on_open(Box::new(move || {
                println!("DataChannel opened!");
                Notification::new()
                    .summary("接続通知")
                    .body("クライアントと接続されました")
                    .timeout(8000)
                    .appname("PortapadSystem")
                    .show()
                    .unwrap();
                // WebSocket切断処理（非同期なので tokio::spawn などで起動）
                let write = write.clone();
                tokio::spawn(async move {
                    println!("WebSocketを閉じます");
                    if let Err(e) = write.lock().await.close().await {
                        eprintln!("WebSocketクローズエラー: {:?}", e);
                    }
                });

                //認証用PCコードを送る
                tokio::spawn(async move {
                    if let Err(e) = dc_clone.send_text(format!("ca{}", CONFIG.pc_code)).await {
                        eprintln!("PCコード送信エラー: {:?}", e);
                    }
                });

                Box::pin(async move {
                    let width = unsafe { winuser::GetSystemMetrics(winuser::SM_CXSCREEN) };
                    let height = unsafe { winuser::GetSystemMetrics(winuser::SM_CYSCREEN) };
                    dc_clone_for_ms.send_text(format!("ms{},{}", width, height)).await.unwrap();
                    //ms -> モニターサイズの略
                })
            }));
            let notify = notify.clone();
            dc.on_close(Box::new(move || {
                println!("DataChannel closed!");
                notify.notify_one();

                Box::pin(async move {
                    println!("DataChannel close完了");
                })
            }));
    
            let enigo_clone = Arc::clone(&enigo_mutex); // MutexをArcで共有する

            dc.on_message(Box::new(move |msg| {

                println!("Received: {:?}", String::from_utf8_lossy(&msg.data));
                let msg_data = msg.data.clone();
                let right_m = Arc::clone(&right_m);
                let left_m = Arc::clone(&left_m);
                let dc_for_send = Arc::clone(&dc_for_send);
                let enigo_clone = Arc::clone(&enigo_clone);
                Box::pin(async move {
                // MutexをロックしてEnigoインスタンスにアクセス
                let mut enigo = enigo_clone.lock().await;
                    
                    let text = String::from_utf8_lossy(&msg_data);
                    let first_two: String = text.chars().take(2).collect();
                    let no_first: String = text.chars().skip(2).collect();

                    unsafe{
                        if IsCerted {
                            match first_two.as_str() {
                                "pg" => {
                                    // 多分ページ？だったはず。クライアントがどのページを触ってたか。使わないっぽい
                                }
                                "mb" => {
                                    match no_first.as_str() {
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
                                        _ => {
                                            eprintln!("Unknown mouse button action: {}", no_first);
                                        }
                                    }
                                }
                                "mm" => {
                                    let parts: Vec<&str> = no_first.split(',').collect();
                                    let part_x: i32 = parts.get(0).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0) * 3;
                                    let part_y: i32 = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0) * 3;
                                    if let Err(e) = enigo.move_mouse(part_x, part_y, enigo::Coordinate::Rel) {
                                        eprintln!("Mouse move relative error: {:?}", e);
                                    }
                                }
                                "mp" => {
                                    let parts: Vec<&str> = no_first.split(',').collect();
                                    let part_x_int: i32 = parts.get(0).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0) as i32;
                                    let part_y_int: i32 = parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0) as i32;
                                    if let Err(e) = enigo.move_mouse(part_x_int, part_y_int, enigo::Coordinate::Abs) {
                                        eprintln!("Mouse move to error: {:?}", e);
                                    }
                                }
                                "md" => {
                                    if let Err(e) = enigo.button(Button::Left, Direction::Press) {
                                        eprintln!("Mouse down error: {:?}", e);
                                    }
                                    let parts: Vec<&str> = no_first.split(',').collect();
                                    let part_x: i32 = parts.get(0).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0) * 3;
                                    let part_y: i32 = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0) * 3;
                                    if let Err(e) = enigo.move_mouse(part_x, part_y, enigo::Coordinate::Rel) {
                                        eprintln!("Mouse drag move error: {:?}", e);
                                    }
                                }
                                "ms" => {
                                    let parts: Vec<&str> = no_first.split(',').collect();
                                    let part_x_int: i32 = parts.get(0).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0) / 6;
                                    let part_y_int: i32 = parts.get(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0) / 6;
                                    if let Err(e) = enigo.scroll(part_x_int, enigo::Axis::Horizontal) {
                                        eprintln!("Horizontal scroll error: {:?}", e);
                                    }
                                    if let Err(e) = enigo.scroll(part_y_int, enigo::Axis::Vertical) {
                                        eprintln!("Vertical scroll error: {:?}", e);
                                    }
                                }
                                "mu" => {
                                    let mut locked_left = left_m.lock().await;
                                    let mut locked_right = right_m.lock().await;
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
                                    // どのパターンにもマッチしない場合の処理
                                    // エラーログ出力や、無視するなどの対応を検討
                                    eprintln!("Unknown command prefix: {}", first_two);
                                }
                            }
                            
                        }else{
                            //認証処理
                            //第一引数から、送られてきた認証コード、設定されてるプライベートキー、設定されてるパブリックキー、PCコード
                            match certification::certification(no_first, CONFIG.privatekey.clone(), CONFIG.publickey.clone(), CONFIG.pc_code.clone()) {
                                Ok(()) => {
                                    println!("✅ 署名検証に成功しました！");
                                    IsCerted = true;
                                }
                                Err(code) => {
                                    eprintln!("❌ 検証失敗");
                                    println!("send_text status: {:?}", dc_for_send.ready_state());
                                    if let Err(e) = dc_for_send.send_text("cb".to_string()).await {
                                        eprintln!("送信エラー: {:?}", e);
                                    }
                                    tokio::spawn(async move {
                                        certification::makeQR(CONFIG.privatekey.clone());
                                    });
                                }
                            }
                        }
                    }
                })
            }));
        })
    }));


    while let Some(msg) = read.next().await {
        let text = msg?.into_text()?;
        println!("{}", text);
        if text.trim().is_empty() {
            continue; // 空メッセージはスキップ
        }
        let signal: SigMessage = match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("JSON parse error: {e}");
                continue;
            }
        };
        println!("{}", signal.mtype);

        // fromhost を書き込み
        {
            let mut fromhost_lock = fromhost_shared.lock().await;
            *fromhost_lock = Some(signal.fromhost.clone());
        }

        if signal.mtype == "sdp" {
            println!("sdpきた");
            let sdpsignal: SigMessageSdp = serde_json::from_str(&text)?;

            peer_connection.set_remote_description(sdpsignal.body).await?;
            let answer = peer_connection.create_answer(None).await?;
            peer_connection.set_local_description(answer.clone()).await?;

            let answerstring= serde_json::to_string(&answer)?;
            // WebSocket経由で返す
            let reply = AnswerSigMessage {
                mtype: "sdpoffer".to_string(),
                tohost: sdpsignal.fromhost,
                body: answerstring,
            };
            let json = serde_json::to_string(&reply)?;
            write.lock().await.send(Message::Text(json)).await?;
        }else if signal.mtype == "ice" {
            print!("iceきた");
                let candidate_init: RTCIceCandidateInit = serde_json::from_value(signal.body)?;
                peer_connection.add_ice_candidate(candidate_init).await?;
        }else if signal.mtype == "myname" {
            let json = serde_json::to_string(&signal.body).unwrap();
            Notification::new()
                .summary("接続コード")
                .body(&json)
                .timeout(8000)
                .appname("PortapadSystem")
                .show()
                .unwrap();
        }
    }

    tokio::select! {
    _ = notify.notified() => {
        println!("DataChannel切断通知を受けました");
    }
    _ = signal::ctrl_c() => {
        println!("Ctrl+Cを受け取りました");
    }
    }
    Ok(())
}

fn string_to_key(s: &str) -> Key {
    match s {
        // 一般キー
        "Enter" => Key::Return,
        "Return" => Key::Return,
        "Backspace" => Key::Backspace,
        "Tab" => Key::Tab,
        "Escape" => Key::Escape,
        "Space" => Key::Space,
        "CapsLock" => Key::CapsLock,

        // 左右の修飾キー
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

        //日本語キーボード独特のキー
        "Kana" => Key::Kana,
        "Convert" => Key::Convert,
        "NonConvert" => Key::NonConvert,
        "HanZen" => Key::Kanji,

        // カーソルキー
        "UpArrow" => Key::UpArrow,
        "DownArrow" => Key::DownArrow,
        "LeftArrow" => Key::LeftArrow,
        "RightArrow" => Key::RightArrow,

        // ファンクションキー
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

        // テンキー
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
        "Subtract" => Key::Subtract,
        "Multiply" => Key::Multiply,
        "Divide" => Key::Divide,
        "Decimal" => Key::Decimal,

        // メディアキーとその他特殊キー
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

        // 記号キー
        "+" => Key::Add,
        "-" => Key::Subtract,
        "*" => Key::Multiply,
        "/" => Key::Divide,
        "." => Key::Decimal,

        // 1文字のキー
        s if s.len() == 1 => {
            if let Some(ch) = s.chars().next() {
                    Key::Unicode(ch)
            } else {
                eprintln!("⚠️ 1文字が取得できませんでした: {:?}", s);
                Key::Space
            }
        }

        // 未知のキー
        _ => {
            eprintln!("❌ サポートされていないキー文字列: {}", s);
            Key::Space
        }
    }
}