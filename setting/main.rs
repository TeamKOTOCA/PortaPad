// すでにあるコードを活かしつつ、UIを切り替える形に構造化する
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use eframe::{egui::*, NativeOptions};
use eframe::egui;
use std::process::Command;
use std::fs;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default)]
struct Config {
    sigserver: String,
    sec_sigserver: String,
    keyboard: String,
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
enum AppScreen {
    #[default]
    Main,
    SubWindow,
}

#[derive(Default, Clone)]
pub struct MyApp {
    pub sig_url: String,
    pub sig_url_sec: String,
    pub key_devicename: String,
    is_recording: bool,
    current_screen: AppScreen,
}

impl MyApp {
    fn show_main_ui(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
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
                ui.label("登録したいキーボードのキーを押してください：");
                let recording_text = if self.is_recording { "記録中..." } else { "記録" };
                if ui.button(recording_text).clicked() {
                    self.is_recording = true;
                    ctx.request_repaint();
                    let mut child = Command::new("target/debug/setting_forkey.exe")
                        .spawn()
                        .expect("rawinputプロセス起動失敗");
                    child.wait().expect("プロセス待機中にエラー");
                    let path = APPDATA.join("input_key_num.txt");
                    if let Ok(contents) = fs::read_to_string(path) {
                        self.key_devicename = contents.trim().to_string();
                    }
                    self.is_recording = false;
                    ctx.request_repaint();
                }
            });

            let keypath = fs::read_to_string(APPDATA.join("input_key_num.txt")).unwrap_or("未記録".to_string());
            ui.label(RichText::new(keypath).small());

            if ui.button("キーボードのマッピングをする").clicked() {
                self.current_screen = AppScreen::SubWindow;
            }

            TopBottomPanel::bottom("bottom_panel").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Portapad v2.1.1");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("キャンセル").clicked() {
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
                            if !self.key_devicename.is_empty() {
                                config.keyboard = self.key_devicename.clone();
                            }
                            fs::write(APPDATA.join("config.toml"), toml::to_string_pretty(&config).unwrap()).unwrap();
                            std::process::exit(0);
                        }
                    });
                });
            });
        });
    }

    fn show_subwindow_ui(&mut self, ctx: &Context) {

        CentralPanel::default().show(ctx, |ui| {
            if ui.button("戻る").clicked() {
                self.current_screen = AppScreen::Main;
            }
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        match self.current_screen {
            AppScreen::Main => self.show_main_ui(ctx),
            AppScreen::SubWindow => self.show_subwindow_ui(ctx),
        }
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