#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use eframe::{egui::*, NativeOptions};
use eframe::egui;
use std::process::Command;
use futures_util::future::ok;
use std::collections::BTreeMap;
use std::thread;
use std::fs;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use std::path::PathBuf;
use serde::Deserialize;
use serde::Serialize;

//設定ファイルの形式
#[derive(Deserialize, Serialize, Debug, Default)]
struct Config {
    sigserver: String,
    sec_sigserver: String,
    keyboard: String,
}

//APPDATAフォルダーのPORTAPADフォルダーを表す変数
pub static APPDATA: Lazy<PathBuf> = Lazy::new(|| {
    //C:\Users\<ユーザー名>\AppData\Roaming
    let base_dir = dirs::config_dir().expect("APPDATAが取得できませんでした");
    // Portapadフォルダ
    let app_dir = base_dir.join("Portapad");
    //なければ作る
    fs::create_dir_all(&app_dir).expect("Portapadフォルダが作れませんでした");
    dirs::config_dir()
        .expect("APPDATAが取得できませんでした")
        .join("portapad")
});

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // フォントファイルを読み込む
    let font_data = std::fs::read("C:\\Windows\\Fonts\\meiryo.ttc")
        .expect("フォントファイルが読み込めません");

    // フォントを追加（"jp" という名前をつける）
    fonts.font_data.insert(
        "jp".to_owned(),
        egui::FontData::from_owned(font_data).into(),
    );

    // 日本語表示をサポートするように上書き
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "jp".to_owned()); // 優先的に使うフォントにする

    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, "jp".to_owned());

    ctx.set_fonts(fonts);
}

#[derive(Default, Clone)]
pub struct MyApp {
    pub sig_url: String,
    pub sig_url_sec: String,
    pub key_devicename: String,
    is_recording: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("シグナリングサーバー");
            ui.horizontal(|ui| {
                ui.label("シグナリングサーバーURL：");
                ui.text_edit_singleline(&mut self.sig_url);
            });
            ui.horizontal(|ui| {
                ui.label("バックアップ用シグナリングサーバーURL：");
                ui.text_edit_singleline(&mut self.sig_url_sec);
            });

            ui.separator();

            ui.heading("キーボード登録");
            ui.horizontal(|ui| {
                ui.label("ボタンを押した後、登録したいキーボードのキーを押してください：");
                let recording_text;
                if self.is_recording {recording_text = "記録中...";}else{recording_text = "記録"}
                if ui.button(recording_text).clicked() {
                    self.is_recording = true;
                    ctx.request_repaint();

                    let mut child = Command::new("target/debug/setting_forkey.exe") 
                        .spawn()
                        .expect("rawinputプロセス起動失敗");

                    println!("rawinputプロセス起動しました");
                    // 子プロセスの終了を待つ
                    let status = child.wait().expect("プロセス待機中にエラー");
                    println!("rawinputプロセス起動おわり");
                    let path = APPDATA.join("input_key_num.txt");
                    match fs::read_to_string(path) {
                        Ok(contents) => {
                            println!("コード:\n{}", contents);
                            self.key_devicename = contents.trim().to_string();
                        }
                        Err(e) => {
                            eprintln!("エラー: {}", e);
                        }
                    }
                    self.is_recording = false;
                    ctx.request_repaint();
                }
            });
            let path = APPDATA.join("input_key_num.txt");
            let mut keypath = "未記録".to_string();
            match fs::read_to_string(path) {
                Ok(contents) => {
                    keypath = contents;
                }
                Err(e) => {
                }
            }
            ui.label(egui::RichText::new(keypath).small());

            ui.separator();
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Portapad v2.1.1");

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if ui.button("キャンセル").clicked() {
                            std::process::exit(0);
                        }
                        if ui.button("保存").clicked() {

                            let mut config: Config = match fs::read_to_string(&APPDATA.join("config.toml")) {
                                Ok(toml_str) => {
                                    toml::from_str(&toml_str).unwrap_or_else(|e| {
                                        eprintln!("TOMLの読み込みに失敗: {}", e);
                                        Config::default()
                                    })
                                }
                                Err(_) => {
                                    eprintln!("ない。新規作成します。");
                                    let default_config = Config::default();

                                    // TOMLに変換して保存
                                    let toml_str = toml::to_string_pretty(&default_config).unwrap();
                                    fs::write(&APPDATA.join("config.toml"), toml_str);
                                    default_config
                                }
                            };
                            
                            if !self.sig_url.is_empty() { config.sigserver = self.sig_url.to_string(); };
                            if !self.sig_url_sec.is_empty() { config.sec_sigserver = self.sig_url_sec.to_string(); };
                            if !self.key_devicename.is_empty() { config.keyboard = self.key_devicename.to_string(); };

                            let new_toml = toml::to_string_pretty(&config).unwrap();
                            fs::write(&APPDATA.join("config.toml"), new_toml);
                            std::process::exit(0);
                        }
                    },
                );
            });
        });

    }

}

fn main() {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 300.0])
            .with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "設定 - Portapad",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    );
}
