use enigo::*;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};  // Messageのインポート
use url::Url;
use futures_util::{SinkExt, StreamExt};
use serde_json;
use serde::Deserialize;
use serde::Serialize;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::ice_transport::ice_server::RTCIceServer;
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

    while let Some(msg) = read.next().await {
        let text = msg?.into_text()?;
        println!("{}", text);
        let signal: SigMessage = serde_json::from_str(&text)?;
        print!("{}", signal.mtype);
        if(signal.mtype == "sdp"){
            print!("sdpきた");
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
            write.send(Message::Text(json)).await?;
        }
    }
    Ok(())
}