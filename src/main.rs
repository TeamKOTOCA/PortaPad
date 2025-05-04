use enigo::*;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};  // Messageのインポート
use url::Url;
use futures_util::{SinkExt, StreamExt};
use serde::{de::IntoDeserializer, Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize, Debug)]  // JSON用の構造体
struct Person {
    mtype: String,
    
}

#[tokio::main]
async fn main() {
    let mut enigo = Enigo::new();

    // マウスを座標(500, 300)へ移動
    enigo.mouse_move_to(500, 300);

    println!("sssssss");
    enigo.mouse_click(MouseButton::Left);

    let url = Url::parse("wss://portapad-signal.onrender.com").unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("接続失敗");

    let (mut write, mut read) = ws_stream.split();

    // メッセージ送信
    write.send(Message::Text("host".into())).await.unwrap();
    write.send(Message::Text("viewclients".into())).await.unwrap();

    // 応答受信
    while let Some(message) = read.next().await {  // socketではなくreadを使用
        match message {
            Ok(Message::Text(text)) => {
                // JSONメッセージをデシリアライズ
                match serde_json::from_str::<Person>(&text) {
                    Ok(person) => {
                        println!("Received Person: {:?}", person);
                    }
                    Err(e) => {
                        eprintln!("Failed to parse JSON: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error in WebSocket communication: {}", e);
                break;
            }
            _ => {}
        }
    }
}
