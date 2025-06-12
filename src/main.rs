mod remote;
use tokio;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Debug)]
struct Config {
    server: ServerConfig,
}

#[derive(Deserialize, Debug)]
struct ServerConfig {
    host: String,
    port: u16,
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

#[tokio::main]
async fn main(){
    let config_path = get_config_path();
    if !config_path.exists() {
        
    }
    let config_str = fs::read_to_string(&config_path)
        .expect("読み込み失敗");
    let config: Config = toml::from_str(&config_str)
        .expect("TOMLエラー");
    println!("{:?}", config);
    remote::remote_main().await.unwrap();
}