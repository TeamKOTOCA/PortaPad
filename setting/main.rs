
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use eframe::{egui::*, NativeOptions};
use eframe::egui;
use std::fs;
use std::io; // io::Error を使うために必要
use once_cell::sync::Lazy;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Deserialize, Serialize, Debug, Default)]
struct Config {
    sigserver: String,
    sec_sigserver: String,
}

pub static APPDATA: Lazy<PathBuf> = Lazy::new(|| {
    let base_dir = dirs::config_dir().expect("APPDATAが取得できませんでした");
    let app_dir = base_dir.join("Portapad");
    fs::create_dir_all(&app_dir).expect("Portapadフォルダが作れませんでした");
    app_dir
});

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font_data = std::fs::read("C:\\Windows\\Fonts\\meiryo.ttc")
        .expect("フォントファイルが読み込めません");
    fonts.font_data.insert("jp".to_owned(), egui::FontData::from_owned(font_data).into());
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "jp".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "jp".to_owned());
    ctx.set_fonts(fonts);
}

#[derive(PartialEq, Clone, Default)]
pub struct MyApp {
    pub sig_url: String,
    pub sig_url_sec: String,
    pub chenged_clients_list: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("シグナリングサーバー");
            ui.label("ws://やhttps://などは書かないでください。");
            ui.horizontal(|ui| {
                ui.label("シグナリングサーバーURL：");
                ui.text_edit_singleline(&mut self.sig_url);
            });
            ui.horizontal(|ui| {
                ui.label("バックアップ用シグナリングサーバーURL：");
                ui.text_edit_singleline(&mut self.sig_url_sec);
            });
            ui.separator();
            ui.heading("許可済みクライアントリスト");
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .hscroll(false) 
                .show(ui, |ui| {
                    ui.label("[削除]で許可を取り消します。");
                    egui::Grid::new("my_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        let mut clients: Vec<String> = vec!["クライアントが見つかりませんでした。".to_string()];
                        match read_lines_from_file(APPDATA.join("clients_list.txt")) {
                            Ok(lines) => {
                                if lines.len() >= 1 {
                                    clients = lines;
                                }
                            }
                            Err(e) => {
                                eprintln!("ファイルの読み込み中にエラーが発生しました: {}", e);
                            }
                        }
                        for client in &clients {
                            ui.label(client);
                            if *client != "クライアントが見つかりませんでした。".to_string() && ui.button("削除").clicked() {
                                remove_client(client.as_str());
                            }
                            ui.end_row();
                        }
                    });
            });
        });
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Portapad v2.1.1");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("キャンセル").clicked() {
                            // Portapadを再起動
                            let _ = Command::new("C:\\Program Files\\PortaPad\\PortaPad.exe")
                                .spawn();

                            std::process::exit(0);
                        }
                        if ui.button("保存").clicked() {
                            let mut config: Config = match fs::read_to_string(APPDATA.join("config.toml")) {
                                Ok(toml_str) => toml::from_str(&toml_str).unwrap_or_default(),
                                Err(_) => Config::default(),
                            };
                            if !self.sig_url.is_empty() {
                                config.sigserver = self.sig_url.clone();
                            }
                            if !self.sig_url_sec.is_empty() {
                                config.sec_sigserver = self.sig_url_sec.clone();
                            }
                            fs::write(APPDATA.join("config.toml"), toml::to_string_pretty(&config).unwrap()).unwrap();
                            
                            // Portapadを再起動
                            let _ = Command::new("C:\\Program Files\\PortaPad\\PortaPad.exe")
                                .spawn();

                            std::process::exit(0);
                        }
                    });
                });
            });
    }
}

fn remove_client(client_name: &str){
    println!("{}", client_name);
        // 1. ファイルから全ての行を読み込む
    let content = fs::read_to_string(APPDATA.join("clients_list.txt")).unwrap();

    // 読み込んだ文字列を改行で分割し、可変なVec<String>に収集する
    let mut lines: Vec<String> = content
        .lines()
        .map(|s| s.to_string())
        .collect();

    // 2. 指定した要素を探して削除する
    // `retain` メソッドは、クロージャが `true` を返す要素だけを残します。
    // そのため、削除したい要素と一致しないものだけを残します。
    let initial_len = lines.len();
    lines.retain(|line| line != client_name); // `line` は `&String`、`target_element` は `&str` なので比較可能

    // 3. 変更されたリストをファイルに保存する
    // 各行を改行文字で結合して一つの文字列に戻す
    let updated_content = lines.join("\n");
    fs::write(APPDATA.join("clients_list.txt"), updated_content).unwrap(); // ファイルに書き戻す
}

fn read_lines_from_file(file_path: PathBuf) -> Result<Vec<String>, io::Error> {
    let content = fs::read_to_string(file_path)?;
    let lines: Vec<String> = content
        .lines() // 改行で文字列をイテレータに分割
        .map(|s| s.to_string()) // 各 &str を所有する String に変換
        .collect();
    Ok(lines)
}

fn main() {
    // portapadプロセスを強制終了
    let _ = Command::new("taskkill")
        .args(["/IM", "PortaPad.exe", "/F"])
        .output();

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
    
    // Portapadを再起動
    let _ = Command::new("C:\\Program Files\\PortaPad\\PortaPad.exe")
        .spawn();
}