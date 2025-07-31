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

pub fn certification() -> Result<(), Box<dyn Error>> {
    const CERT_BG_IMG: &[u8] = include_bytes!("cert_bg_sec.png");
    let mut background_img = image::load_from_memory(CERT_BG_IMG)?.to_rgba8();

    let private_key = create_code();
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

    background_img.copy_from(&img, 60, 130)?;

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

fn create_code() -> String {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    let der = signing_key
        .to_pkcs8_der()
        .expect("DER（公開鍵付き）失敗");

    base64::encode(der.as_bytes())
}
