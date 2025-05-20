use enigo::*;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};  // Messageのインポート
use std::sync::Arc;
use tokio::sync::Mutex;
use futures_util::{SinkExt, StreamExt};
use serde_json;
use serde::Deserialize;
use serde::Serialize;
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;


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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut enigo = Enigo::new();
    
    //MediaEngine: 音声/映像のコーデック設定
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;

    //APIBuilderでWebRTCインスタンスを作成
    let api = APIBuilder::new()
        .with_media_engine(m)
        .build();

    //ICEサーバー（STUNサーバー）設定
    let config = RTCConfiguration {
        ice_servers: vec![],
        ..Default::default()
    };

    //fromhost共有用
    let fromhost_shared = Arc::new(Mutex::new(None::<String>));

    // マウスを座標(500, 300)へ移動.
    enigo.mouse_move_to(500, 300);
    println!("sssssss");
    enigo.mouse_click(MouseButton::Left);
    //enigo終.

    // WebSocket接続開始
    let (ws_stream, _) = connect_async("wss://portapad-signal.onrender.com").await?;
    let (mut write, mut read) = ws_stream.split();

    write.send(Message::Text("host".to_string())).await?;

    // PeerConnection作成
    let peer_connection = api.new_peer_connection(config).await?;
    
    let write = Arc::new(Mutex::new(write));
    let write_clone = Arc::clone(&write);
    let fromhost_clone = Arc::clone(&fromhost_shared);


    peer_connection.on_ice_candidate(Box::new(move |candidate| {
    let write = write_clone.clone();
    let fromhost = Arc::clone(&fromhost_clone);
    Box::pin(async move {
        if let Some(c) = candidate {
            // JSON化でエラーが出る可能性があるので、match で手動処理
            let json_candidate = match serde_json::to_string(&c) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("Error serializing candidate: {:?}", e);
                    return;
                }
            };

            let tohost = {
                let fromhost_guard = fromhost.lock().await;
                fromhost_guard.clone().unwrap_or_else(|| "unknown".to_string())
            };

            let reply = AnswerSigMessage {
                mtype: "ice".to_string(),
                tohost: tohost,
                body: json_candidate,
            };

            let json = match serde_json::to_string(&reply) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("シリアライズのエラー: {:?}", e);
                    return;
                }
            };

            if let Err(e) = write.lock().await.send(Message::Text(json)).await {
                eprintln!("送信エラー: {:?}", e);
            }
        }
    })
}));

    peer_connection.on_data_channel(Box::new(|dc| {
        println!("DataChannel received: {}", dc.label());

        Box::pin(async move {
            // クローンして move で使う
            let dc_clone = Arc::clone(&dc);
            dc.on_open(Box::new(move || {
                println!("DataChannel opened!");
                Box::pin(async move {
                    dc_clone.send_text("こんにちは from offer").await.unwrap();
                })
            }));

            // クローンして message 用に使う
            let dc_clone2 = Arc::clone(&dc);
            dc.on_message(Box::new(move |msg| {
                println!("Received: {:?}", String::from_utf8_lossy(&msg.data));
                // 必要なら dc_clone2 を使って返信などもできる
                Box::pin(async {})
            }));
        })
    }));


    while let Some(msg) = read.next().await {
        let text = msg?.into_text()?;
        println!("{}", text);
        let signal: SigMessage = serde_json::from_str(&text)?;
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
        }
    }

    Ok(())
}