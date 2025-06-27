#![windows_subsystem = "windows"]
mod remote;
use tokio;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};
use std::process::Command;
use notify_rust::Notification;

#[derive(Deserialize, Debug)]
struct Config {
    server: ServerConfig,
}

#[derive(Deserialize, Debug)]
struct ServerConfig {
    host: String,
    port: u16,
}
/*
fn get_config_path() -> PathBuf {
    let mut path = env::var_os("APPDATA")
        .map(PathBuf::from)
        .expect("APPDATAが取得できませんでした");
    path.push("portapad");
    fs::create_dir_all(&path).expect("フォルダ作成失敗");
    path.push("config.toml");
    path
}


fn open_setting(){
        // GUIサブプロセスを起動
    let mut child = Command::new("target/debug/gui_app") 
        .spawn()
        .expect("設定画面起動失敗");

    println!("設定画面を別プロセスで起動しました");

    // 子プロセスの終了を待つ（必要なら）
    let status = child.wait().expect("プロセス待機中にエラー");
    println!("GUIプロセス終了: {:?}", status);
}*/

#[tokio::main]
async fn main(){/*
    let config_path = get_config_path();
    if !config_path.exists() {
        //open_setting();
    }
    let config_str = fs::read_to_string(&config_path)
        .expect("読み込み失敗");
    let config: Config = toml::from_str(&config_str)
        .expect("TOMLエラー");
    println!("{:?}", config);*/
    loop {
        if let Err(e) = remote::remote_main().await {
            eprintln!("remote_mainが終了しました: {:?}", e);
            // 必要なら少し待機してから再起動
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }
}