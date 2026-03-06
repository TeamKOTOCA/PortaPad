use base64::{Engine, engine::general_purpose};
use ed25519_dalek::{
    Signature, Signer, SigningKey, Verifier, VerifyingKey,
    pkcs8::{DecodePublicKey, EncodePublicKey},
};
use image::{GenericImage, ImageBuffer, Rgba, RgbaImage};
use minifb::{Key as MinifbKey, Window, WindowOptions};
use pkcs8::EncodePrivateKey;
use qrcode_generator::QrCodeEcc;
use rand::rngs::OsRng;
use serde::Deserialize;
use std::convert::TryInto;
use std::error::Error;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use std::{env, fs, path::PathBuf};
use windows_dpapi::{Scope, decrypt_data, encrypt_data};

//config.toml(設定ファイル)の型
#[derive(Deserialize, Debug)]
struct Config {
    sigserver: String,
    sec_sigserver: String,
    pc_code: String,
    privatekey: String,
    publickey: String,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let config_path = get_config_path();
    let config_str = fs::read_to_string(&config_path)
        // エラーメッセージを改善し、どのファイルが読み込めなかったかを示すようにしています
        .expect(&format!(
            "設定ファイルの読み込みに失敗しました: {:?}",
            config_path
        ));
    let setting_config: Config =
        toml::from_str(&config_str).expect("TOML形式の設定ファイルのパースに失敗しました");
    setting_config
});

fn get_config_path() -> PathBuf {
    let mut path = env::var_os("APPDATA")
        .map(PathBuf::from)
        .expect("APPDATAが取得できませんでした");
    path.push("portapad");
    fs::create_dir_all(&path).expect("フォルダ作成失敗");
    path.push("config.toml");
    path
}

fn main() -> Result<(), Box<dyn Error>> {
    const CERT_BG_IMG: &[u8] = include_bytes!("cert_bg_sec.png");
    let mut background_img = image::load_from_memory(CERT_BG_IMG)?.to_rgba8();
    println!("📨 privatekey: {}", CONFIG.privatekey);

    let encrypted_bytes = general_purpose::STANDARD
        .decode(&CONFIG.privatekey)
        .expect("base64 decode failed");

    let decrypted_bytes = decrypt_data(&encrypted_bytes, Scope::User).expect("decrypt failed");

    let key_bytes: [u8; 32] = decrypted_bytes
        .as_slice()
        .try_into()
        .expect("invalid key length");

    let signing_key = SigningKey::from_bytes(&key_bytes);

    // QRコード用に base64 化する場合
    let content = base64::encode(signing_key.to_bytes());

    let error_correction = QrCodeEcc::High;
    let module_size = 4;
    let margin = 4;

    let qrcode_matrix = qrcode_generator::to_matrix(content, error_correction)
        .map_err(|e| format!("Failed to generate QR code matrix: {:?}", e))?;

    let qr_width = qrcode_matrix[0].len();
    let qr_height = qrcode_matrix.len();

    let image_width = (qr_width + 2 * margin) * module_size;
    let image_height = (qr_height + 2 * margin) * module_size;

    let mut img: RgbaImage = ImageBuffer::new(image_width as u32, image_height as u32);

    for y in 0..image_height {
        for x in 0..image_width {
            let mut is_dark = false;

            if x >= (margin * module_size)
                && x < ((margin + qr_width) * module_size)
                && y >= (margin * module_size)
                && y < ((margin + qr_height) * module_size)
            {
                let qr_x = (x - margin * module_size) / module_size;
                let qr_y = (y - margin * module_size) / module_size;

                if qr_y < qr_height && qr_x < qr_width {
                    is_dark = qrcode_matrix[qr_y][qr_x];
                }
            }

            let pixel_color = if is_dark {
                Rgba([7, 113, 212, 255]) // alpha修正
            } else {
                Rgba([255, 255, 255, 255])
            };
            img.put_pixel(x as u32, y as u32, pixel_color);
        }
    }

    background_img.copy_from(&img, 100, 170)?;

    let mut buffer: Vec<u32> =
        Vec::with_capacity(background_img.width() as usize * background_img.height() as usize);
    for pixel in background_img.pixels() {
        let r = pixel[0] as u32;
        let g = pixel[1] as u32;
        let b = pixel[2] as u32;
        let a = pixel[3] as u32;
        buffer.push((a << 24) | (r << 16) | (g << 8) | b);
    }

    let background_width = background_img.width() as usize;
    let background_height = background_img.height() as usize;

    let mut options = WindowOptions::default();
    options.topmost = true;

    let mut window = Window::new(
        "Portapad認証システム",
        background_width,
        background_height,
        options,
    )?;

    let start_time = Instant::now();
    let display_duration = Duration::from_secs(10);

    while window.is_open() && !window.is_key_down(MinifbKey::Escape) {
        if start_time.elapsed() >= display_duration {
            break;
        }

        window.update_with_buffer(&buffer, background_width, background_height)?;
        std::thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}
