#![cfg_attr(
    all(not(debug_asertions), target_os="windows"),
    windows_subsystem="windows"
)]

use eframe::{egui::*, NativeOptions};

#[derive(Default, Clone)]
pub struct MyApp {
}

const RUST_LOGO: egui::ImageSource = egui::include_image!("portapad.webp");

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.0);

        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.label("設定画面へようこそ");
            egui::Image::new(RUST_LOGO).paint_at(ui, Rect::from_min_max( pos2(100.0, 100.0), pos2(300.0, 300.0)));
        });
    }
}

fn main() {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 240.0])
            .with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "設定 - Portapad",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    );
}
