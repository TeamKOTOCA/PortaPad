use enigo::*;
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
use tokio::signal;
use std::{env, fs, path::PathBuf};
use tokio::time::{interval, Duration};

use webrtc::ice_transport::ice_candidate::RTCIceCandidate;

#[derive(Serialize)]
struct IceCandidateMsg {
    candidate: String,
    sdpMid: Option<String>,
    sdpMLineIndex: Option<u16>,
}

#[derive(Deserialize, Debug)]
struct Config {
    sigserver: String,
    sec_sigserver: String,
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

    let config_path = get_config_path();
    let config_str = fs::read_to_string(&config_path)
        .expect("読み込み失敗");
    let setting_config: Config = toml::from_str(&config_str)
        .expect("TOMLエラー");

    // WebSocket接続開始
    let ws_stream_result = connect_async("wss://".to_string() + &setting_config.sigserver).await;
    let (ws_stream, _) = match ws_stream_result {
        Ok(result) => result,
        Err(e) => {
            eprintln!("WebSocket接続に失敗しました: {:?}", e);
            let fallback_ws_stream_result = connect_async("wss://".to_string() + &setting_config.sec_sigserver).await;
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

    peer_connection.on_data_channel(Box::new(move |dc| {
        println!("DataChannel received: {}", dc.label());
        let right_m = Arc::clone(&right_m_m);
        let left_m = Arc::clone(&left_m_m);
        let notify = notify_for_dc.clone();
        let write = write_for_close.clone();

        Box::pin(async move {
            // クローンして move で使う
            let write = write.clone();
            let dc_clone = Arc::clone(&dc);
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

                Box::pin(async move {
                    let width = unsafe { winuser::GetSystemMetrics(winuser::SM_CXSCREEN) };
                    let height = unsafe { winuser::GetSystemMetrics(winuser::SM_CYSCREEN) };
                    dc_clone.send_text(format!("ms{},{}", width, height)).await.unwrap();
                    //ms -> モニターサイズの略
                })
            }));
            let notify = notify.clone();
            dc.on_close(Box::new(move || {
                println!("DataChannel closed!");
                Notification::new()
                    .summary("切断通知")
                    .body("クライアントとの接続が切断されました")
                    .timeout(8000)
                    .appname("PortapadSystem")
                    .show()
                    .unwrap();
                    notify.notify_one();

                Box::pin(async move {
                    println!("DataChannel close完了");
                })
            }));

            let dc_clone2 = Arc::clone(&dc);
            dc.on_message(Box::new(move |msg| {

                println!("Received: {:?}", String::from_utf8_lossy(&msg.data));
                let msg_data = msg.data.clone();
                let right_m = Arc::clone(&right_m);
                let left_m = Arc::clone(&left_m);
                Box::pin(async move{
                    //操作モジュールのenigo初期化
                    let mut enigo = Enigo::new();
                    
                    let text = String::from_utf8_lossy(&msg_data);
                    let first_two: String = text.chars().take(2).collect();
                    let no_first: String = text.chars().skip(2).collect();
                    if first_two == "pg"{

                    }else if first_two == "mb"{
                        if no_first == "0"{
                            enigo.mouse_click(MouseButton::Left);
                        }else if no_first == "1" {
                            enigo.mouse_click(MouseButton::Right);
                        }
                    }else if first_two == "mm" {
                        let parts: Vec<&str> = no_first.split(',').collect();
                        let part_x: &&str = parts.get(0).unwrap_or(&"0");
                        let part_y: &&str = parts.get(1).unwrap_or(&"0");
                        let part_x_int: i32 = part_x.parse::<i32>().unwrap() * 3;
                        let part_y_int: i32 = part_y.parse::<i32>().unwrap() * 3;
                        enigo.mouse_move_relative(part_x_int, part_y_int);
                    }else if first_two == "mp"{
                            let parts: Vec<&str> = no_first.split(',').collect();
                            let part_x = parts.get(0).copied().unwrap_or("0");
                            let part_y = parts.get(1).copied().unwrap_or("0");
                            let part_x_int = part_x.parse::<f64>().unwrap_or(0.0) as i32;
                            let part_y_int = part_y.parse::<f64>().unwrap_or(0.0) as i32;
                            enigo.mouse_move_to(part_x_int, part_y_int);
                    }else if first_two == "md"{
                        enigo.mouse_down(MouseButton::Left);
                        let parts: Vec<&str> = no_first.split(',').collect();
                        let part_x: &&str = parts.get(0).unwrap_or(&"0");
                        let part_y: &&str = parts.get(1).unwrap_or(&"0");
                        let part_x_int: i32 = part_x.parse::<i32>().unwrap() * 3;
                        let part_y_int: i32 = part_y.parse::<i32>().unwrap() * 3;
                        enigo.mouse_move_relative(part_x_int, part_y_int);
                    }else if first_two == "ms"{
                        let parts: Vec<&str> = no_first.split(',').collect();
                        let part_x = parts.get(0).unwrap_or(&"0");
                        let part_y = parts.get(1).unwrap_or(&"0");
                        let part_x_int: i32 = part_x.parse::<i32>().unwrap() / 6;
                        let part_y_int: i32 = part_y.parse::<i32>().unwrap() / 6;
                        enigo.mouse_scroll_x(part_x_int);
                        enigo.mouse_scroll_y(part_y_int);
                    }else if first_two == "mu"{
                            let mut locked_left = left_m.lock().await;
                            let mut locked_right = right_m.lock().await;
                        if *locked_left {
                            println!("aa");
                            enigo.mouse_up(MouseButton::Left);
                            *locked_left = false;
                        }
                        if *locked_right {
                            println!("as");
                            enigo.mouse_up(MouseButton::Right);
                            *locked_right = false;
                        }
                    }else if first_two == "kp"{
                        let key = string_to_key(&no_first);
                        enigo.key_click(key);
                    }else if first_two == "ku"{
                        let key = string_to_key(&no_first);
                        enigo.key_up(key);
                    }else if first_two == "kd"{
                        let key = string_to_key(&no_first);
                        enigo.key_down(key);
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
        "Return" => Key::Return,
        "Backspace" => Key::Backspace,
        "Tab" => Key::Tab,
        "Escape" => Key::Escape,
        "Space" => Key::Space,
        "CapsLock" => Key::CapsLock,
        "Shift" => Key::Shift,
        "Control" => Key::Control,
        "Alt" => Key::Alt,
        "Meta" => Key::Meta,
        "UpArrow" => Key::UpArrow,
        "DownArrow" => Key::DownArrow,
        "LeftArrow" => Key::LeftArrow,
        "RightArrow" => Key::RightArrow,
        // 単一文字ならレイアウトキーとして解釈
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            Key::Layout(ch)
        }
        _ => panic!("Unsupported key string: {}", s),
    }
}