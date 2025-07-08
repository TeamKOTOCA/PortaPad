#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use eframe::{egui::*, NativeOptions};
use eframe::egui;
use futures_util::future::ok;
use std::collections::BTreeMap;
//mod keyboard;

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
}

const RUST_LOGO: egui::ImageSource = egui::include_image!("portapad.webp");

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("シグナリングサーバー");
            ui.horizontal(|ui| {
                ui.label("シグナリングサーバーURL：");
                ui.text_edit_singleline(&mut self.sig_url);
            });

            ui.heading("シグナリングサーバー");
            ui.horizontal(|ui| {
                ui.label("ボタンを押した後、登録したいキーボードのキーを押してください：");
                if ui.button("記録").clicked() {
                    
                }
            });


            ui.separator();
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Portapad v1.2.1");

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center), // ← 右寄せ
                    |ui| {
                        if ui.button("適用").clicked() {
                            println!("シグナリングサーバー：{}", self.sig_url);
                        }
                        if ui.button("キャンセル").clicked() {
                            std::process::exit(0);
                        }
                        if ui.button("保存").clicked() {
                            println!("シグナリングサーバー：{}", self.sig_url);
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
