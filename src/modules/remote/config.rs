use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub sigserver: String,
    pub sec_sigserver: String,
    pub pc_code: String,
    pub privatekey: String,
    pub publickey: String,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config_path = get_config_path();
    let config_str = fs::read_to_string(&config_path)
        .expect(&format!("設定ファイルが読めません: {:?}", config_path));
    toml::from_str(&config_str).expect("config.toml の解析に失敗しました")
});

fn get_config_path() -> PathBuf {
    let mut path = env::var_os("APPDATA")
        .map(PathBuf::from)
        .expect("APPDATA が見つかりません");
    path.push("portapad");
    fs::create_dir_all(&path).expect("設定フォルダの作成に失敗しました");
    path.push("config.toml");
    path
}
