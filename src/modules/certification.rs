use minifb::{Key as MinifbKey, Window, WindowOptions};
use image::{RgbaImage, ImageBuffer, Rgba, GenericImage};
use qrcode_generator::QrCodeEcc;
use std::time::{Duration, Instant};
use std::error::Error;
use rand::rngs::OsRng;
use pkcs8::EncodePrivateKey;
use ed25519_dalek::{
    SigningKey, VerifyingKey, Signature, Signer, Verifier,
    pkcs8::{EncodePublicKey, DecodePublicKey},
};
use base64::{engine::general_purpose, Engine};
use std::convert::TryInto;

pub fn certification(signature: String, private_key: String, public_key: String, pc_code: String) -> Result<(), i32> {
    println!("📨 code（署名対象）: {}", pc_code);
    println!("📨 publickey: {}", public_key);
    println!("📨 privatekey: {}", private_key);
    println!("📨 signature: {}", signature);


    // Base64デコード（署名）
    let signature_bytes = match general_purpose::STANDARD.decode(&signature) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("❌ 署名のBase64デコード失敗: {}", e);
            return Err(1);
        }
    };

    // Base64デコード（公開鍵）
    let public_key_bytes = match general_purpose::STANDARD.decode(&public_key) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("❌ 公開鍵のBase64デコード失敗: {}", e);
            return Err(1);
        }
    };

    // ✅ Vec<u8> → &[u8; 32] に変換
    let public_key_array: &[u8; 32] = match public_key_bytes.as_slice().try_into() {
        Ok(arr) => arr,
        Err(_) => {
            eprintln!("❌ 公開鍵のバイト数が32ではありません");
            return Err(1);
        }
    };

    // 署名を構造体に変換
    let signature = match Signature::from_slice(&signature_bytes) {
        Ok(sig) => sig,
        Err(e) => {
            eprintln!("❌ 署名形式エラー: {}", e);
            return Err(1);
        }
    };

    // 公開鍵を構造体に変換
    let verifying_key = match VerifyingKey::from_bytes(public_key_array) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("❌ 公開鍵形式エラー: {}", e);
            return Err(1);
        }
    };

    // 検証実行
    match verifying_key.verify(pc_code.as_bytes(), &signature) {
        Ok(_) => {
            println!("✅ 検証成功！署名は正当です: {}", pc_code);
            return Ok(());
        }
        Err(e) => {
            eprintln!("❌ 検証失敗: {}", e);
            makeQR(private_key.to_string());
            return Err(1);
        }
    }
}


fn makeQR(private_key_from_config: String) -> Result<(), Box<dyn Error>> {
        const CERT_BG_IMG: &[u8] = include_bytes!("cert_bg_sec.png");
    let mut background_img = image::load_from_memory(CERT_BG_IMG)?.to_rgba8();

    let private_key = private_key_from_config;
    let content = private_key.as_str();
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

            if x >= (margin * module_size) && x < ((margin + qr_width) * module_size) &&
               y >= (margin * module_size) && y < ((margin + qr_height) * module_size) {
                
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

    let mut buffer: Vec<u32> = Vec::with_capacity(background_img.width() as usize * background_img.height() as usize);
    for pixel in background_img.pixels() {
        let r = pixel[0] as u32;
        let g = pixel[1] as u32;
        let b = pixel[2] as u32;
        let a = pixel[3] as u32;
        buffer.push((a << 24) | (r << 16) | (g << 8) | b);
    }

    let background_width = background_img.width() as usize;
    let background_height = background_img.height() as usize;

    let mut window = Window::new(
        "Portapad認証システム",
        background_width,
        background_height,
        WindowOptions::default(),
    )?;
    println!("viewed");

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