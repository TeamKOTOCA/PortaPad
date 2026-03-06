mod audio;
mod config;
mod input;

use audio::{build_pcmu_track, start_system_audio_capture};
use config::CONFIG;
use enigo::{Enigo, Settings};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use input::InputHandler;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::{Duration, interval};
use tokio::{net::TcpStream, signal};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};
use webrtc::{
    api::{APIBuilder, media_engine::MediaEngine},
    data_channel::RTCDataChannel,
    ice_transport::ice_candidate::RTCIceCandidateInit,
    peer_connection::{
        configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription,
    },
    track::track_local::TrackLocal,
};
use winapi::um::winuser;

type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;

#[derive(Serialize)]
struct IceCandidateMsg {
    candidate: String,
    sdpMid: Option<String>,
    sdpMLineIndex: Option<u16>,
}

#[derive(Deserialize, Debug)]
struct SigMessage {
    mtype: String,
    fromhost: String,
    body: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct SigMessageSdp {
    mtype: String,
    fromhost: String,
    body: RTCSessionDescription,
}

#[derive(Serialize)]
struct AnswerSigMessage {
    mtype: String,
    tohost: String,
    body: String,
}

pub async fn remote_main() -> Result<(), Box<dyn std::error::Error>> {
    let notify = Arc::new(Notify::new());
    let notify_for_dc = notify.clone();

    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let api = APIBuilder::new().with_media_engine(media_engine).build();

    let config = RTCConfiguration {
        ice_servers: vec![],
        ..Default::default()
    };

    let (ws_stream, _) = match connect_async(format!("wss://{}", CONFIG.sigserver)).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("WebSocket 接続エラー: {:?}", err);
            match connect_async(format!("wss://{}", CONFIG.sec_sigserver)).await {
                Ok(stream) => {
                    println!("予備サーバーへの接続に成功しました");
                    stream
                }
                Err(fallback) => {
                    eprintln!("予備サーバーへの接続にも失敗: {:?}", fallback);
                    return Err(fallback.into());
                }
            }
        }
    };

    let (mut write, mut read) = ws_stream.split();
    write.send(Message::Text("host".to_string())).await?;

    let peer_connection = api.new_peer_connection(config).await?;

    let audio_track = build_pcmu_track();
    let connection_track: Arc<dyn TrackLocal + Send + Sync> = audio_track.clone();
    if let Err(err) = peer_connection.add_track(connection_track).await {
        eprintln!("音声トラック追加失敗: {:?}", err);
    }
    if let Err(err) = start_system_audio_capture(Arc::clone(&audio_track)) {
        eprintln!("音声キャプチャ開始失敗: {:?}", err);
    }

    let write = Arc::new(Mutex::new(write));
    let write_clone = Arc::clone(&write);
    let fromhost_shared = Arc::new(Mutex::new(None::<String>));
    let fromhost_for_ice = Arc::clone(&fromhost_shared);

    let enigo_mutex = Arc::new(Mutex::new(Enigo::new(&Settings::default()).unwrap()));
    let left_mouse = Arc::new(Mutex::new(false));
    let right_mouse = Arc::new(Mutex::new(false));

    tokio::spawn(ping_task(Arc::clone(&write)));

    peer_connection.on_ice_candidate(Box::new(move |candidate| {
        let write = Arc::clone(&write_clone);
        let fromhost = Arc::clone(&fromhost_for_ice);
        Box::pin(async move {
            if let Some(c) = candidate {
                let c_json = match c.to_json() {
                    Ok(json) => json,
                    Err(err) => {
                        eprintln!("ICE to_json エラー: {:?}", err);
                        return;
                    }
                };

                let msg = IceCandidateMsg {
                    candidate: c_json.candidate,
                    sdpMid: c_json.sdp_mid,
                    sdpMLineIndex: c_json.sdp_mline_index.map(|v| v as u16),
                };

                let body = match serde_json::to_string(&msg) {
                    Ok(text) => text,
                    Err(err) => {
                        eprintln!("ICE JSON 生成エラー: {:?}", err);
                        return;
                    }
                };

                let tohost = {
                    let guard = fromhost.lock().await;
                    guard.clone().unwrap_or_else(|| "unknown".to_string())
                };

                let reply = AnswerSigMessage {
                    mtype: "ice".to_string(),
                    tohost,
                    body,
                };

                if let Err(err) = write
                    .lock()
                    .await
                    .send(Message::Text(serde_json::to_string(&reply).unwrap()))
                    .await
                {
                    eprintln!("ICE 送信エラー: {:?}", err);
                }
            }
        })
    }));

    let write_for_dc = Arc::clone(&write);
    peer_connection.on_data_channel(Box::new(move |dc| {
        let dc = Arc::new(dc);
        register_data_channel_handlers(
            Arc::clone(&dc),
            notify_for_dc.clone(),
            Arc::clone(&write_for_dc),
            Arc::clone(&enigo_mutex),
            Arc::clone(&left_mouse),
            Arc::clone(&right_mouse),
        );
        Box::pin(async move {})
    }));

    while let Some(msg) = read.next().await {
        let text = msg?.into_text()?;
        println!("{}", text);
        if text.trim().is_empty() {
            continue;
        }

        let signal: SigMessage = match serde_json::from_str(&text) {
            Ok(signal) => signal,
            Err(err) => {
                eprintln!("JSON parse error: {:?}", err);
                continue;
            }
        };

        {
            let mut guard = fromhost_shared.lock().await;
            *guard = Some(signal.fromhost.clone());
        }

        if signal.mtype == "sdp" {
            let sdpsignal: SigMessageSdp = serde_json::from_str(&text)?;
            peer_connection
                .set_remote_description(sdpsignal.body)
                .await?;
            let answer = peer_connection.create_answer(None).await?;
            peer_connection
                .set_local_description(answer.clone())
                .await?;
            let reply = AnswerSigMessage {
                mtype: "sdpoffer".to_string(),
                tohost: sdpsignal.fromhost,
                body: serde_json::to_string(&answer)?,
            };
            write
                .lock()
                .await
                .send(Message::Text(serde_json::to_string(&reply)?))
                .await?;
        } else if signal.mtype == "ice" {
            let candidate_init: RTCIceCandidateInit = serde_json::from_value(signal.body)?;
            peer_connection.add_ice_candidate(candidate_init).await?;
        } else if signal.mtype == "myname" {
            let json = serde_json::to_string(&signal.body)?;
            Notification::new()
                .summary("接続")
                .body(&json)
                .timeout(8000)
                .appname("PortapadSystem")
                .show()
                .unwrap();
        }
    }

    tokio::select! {
        _ = notify.notified() => {
            println!("データチャネルが閉じられました");
        }
        _ = signal::ctrl_c() => {
            println!("Ctrl+C で終了します");
        }
    }

    Ok(())
}

fn ping_task(write: Arc<Mutex<WsSink>>) -> impl std::future::Future<Output = ()> {
    async move {
        let mut ticker = interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let ping_msg = Message::Ping(b"heartbeat".to_vec());
            if let Err(err) = write.lock().await.send(ping_msg).await {
                eprintln!("Ping 送信エラー: {:?}", err);
                break;
            }
        }
    }
}

fn register_data_channel_handlers(
    dc: Arc<RTCDataChannel>,
    notify: Arc<Notify>,
    write_for_close: Arc<Mutex<WsSink>>,
    enigo_mutex: Arc<Mutex<Enigo>>,
    left_mouse: Arc<Mutex<bool>>,
    right_mouse: Arc<Mutex<bool>>,
) {
    let handler = InputHandler::new(
        Arc::clone(&enigo_mutex),
        Arc::clone(&left_mouse),
        Arc::clone(&right_mouse),
        Arc::clone(&dc),
    );

    let handler_for_msg = handler.clone();
    let dc_for_open = Arc::clone(&dc);

    dc.on_open(Box::new(move || {
        let dc = Arc::clone(&dc_for_open);
        let write = Arc::clone(&write_for_close);
        Box::pin(async move {
            Notification::new()
                .summary("接続")
                .body("スマホから接続されました")
                .timeout(8000)
                .appname("PortapadSystem")
                .show()
                .unwrap();

            tokio::spawn(async move {
                if let Err(err) = write.lock().await.close().await {
                    eprintln!("WebSocket close error: {:?}", err);
                }
            });

            if let Err(err) = dc.send_text(format!("ca{}", CONFIG.pc_code)).await {
                eprintln!("PCコード送信エラー: {:?}", err);
            }

            let width = unsafe { winuser::GetSystemMetrics(winuser::SM_CXSCREEN) };
            let height = unsafe { winuser::GetSystemMetrics(winuser::SM_CYSCREEN) };
            if let Err(err) = dc.send_text(format!("ms{},{}", width, height)).await {
                eprintln!("画面サイズ送信エラー: {:?}", err);
            }
        })
    }));

    let notify_close = notify.clone();
    dc.on_close(Box::new(move || {
        let notify = notify_close.clone();
        Box::pin(async move {
            println!("データチャネルが閉じられました");
            notify.notify_one();
        })
    }));

    dc.on_message(Box::new(move |msg| {
        let handler = handler_for_msg.clone();
        Box::pin(async move {
            println!("Received: {:?}", String::from_utf8_lossy(&msg.data));
            handler.handle_message(&msg.data).await;
        })
    }));
}
