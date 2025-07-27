use minifb::{Key as MinifbKey, Window, WindowOptions};
use image::{RgbaImage, ImageBuffer, Rgba, GenericImage};
use qrcode_generator::QrCodeEcc;
use std::time::{Duration, Instant};
use std::error::Error;
use rand::rngs::OsRng;
use ed25519_dalek::{
    SigningKey, VerifyingKey, Signature, Signer, Verifier,
    pkcs8::{EncodePrivateKey, EncodePublicKey, DecodePublicKey},
};
pub fn certification() -> Result<(), Box<dyn Error>> {


    //認証画面の背景画像
    const CERT_BG_IMG: &[u8] = include_bytes!("cert_bg_sec.png");
    let mut background_img = image::load_from_memory(CERT_BG_IMG).unwrap().to_rgba8();

    //QRコード生成
    let private_key = create_code();
    let content = private_key.as_str(); // QRコードにエンコードしたい内容
    let error_correction = QrCodeEcc::High; // エラー訂正レベル
    let module_size = 4; // 各モジュールのピクセルサイズ (大きくすると画像も大きくなる)
    let margin = 4; // 余白モジュールの数 (QRコードの周りの白い部分)
    
    let qrcode_matrix = qrcode_generator::to_matrix(content, error_correction)
        .map_err(|e| format!("Failed to generate QR code matrix: {:?}", e))?;

    let qr_width = qrcode_matrix[0].len(); // QRコードのモジュール数（幅）
    let qr_height = qrcode_matrix.len();  // QRコードのモジュール数（高さ）

    //画像全体のサイズ
    let image_width = (qr_width + 2 * margin) * module_size;
    let image_height = (qr_height + 2 * margin) * module_size;

    //ピクセルデータを生成
    let mut img: RgbaImage = ImageBuffer::new(image_width as u32, image_height as u32);

    for y in 0..image_height {
        for x in 0..image_width {
            let mut is_dark = false;

            // 余白の範囲外の場合のみ、QRコードのモジュールデータを参照
            if x >= (margin * module_size) && x < ((margin + qr_width) * module_size) &&
               y >= (margin * module_size) && y < ((margin + qr_height) * module_size) {
                
                let qr_x = (x - margin * module_size) / module_size;
                let qr_y = (y - margin * module_size) / module_size;

                //データは (y, x)
                if qr_y < qr_height && qr_x < qr_width {
                    is_dark = qrcode_matrix[qr_y][qr_x]; 
                }
            }

            // ピクセルの色を設定 (QRコードがあお、背景が白)
            let pixel_color = if is_dark {
                Rgba([7, 113, 212, 1]) // 青(RGBA)
            } else {
                Rgba([255, 255, 255, 255]) // 白 (RGBA)
            };
            img.put_pixel(x as u32, y as u32, pixel_color);
        }
    }
    
    background_img.copy_from(&img, 60, 130)?;

    // --- minifbで表示するためにピクセルデータを変換 ---
    // minifbはARGB形式を期待するので、RGBAから変換します
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


    // --- ウィンドウの作成と表示 ---
    let mut window = Window::new(
        "Portapad認証システム", // ウィンドウタイトル
        background_width,
        background_height,
        WindowOptions::default(),
    )?;
    println!("viewed");

    // 10秒のタイマーを開始
    let start_time = Instant::now();
    let display_duration = Duration::from_secs(10);

    while window.is_open() && !window.is_key_down(MinifbKey::Escape) { // Escキーで終了
        // 30秒経過したかチェック
        if start_time.elapsed() >= display_duration {
            break; // 30秒経過したらループを抜ける
        }

        // ウィンドウを更新し、画像を表示
        window.update_with_buffer(&buffer, background_width, background_height)?;

        // CPU使用率を抑えるために少し待機 (任意)
        std::thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}

fn create_code() -> String{
    // 鍵作成
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    // 秘密鍵をPEM形式に変換
    let pem = signing_key
        .to_pkcs8_pem(Default::default())
        .expect("PEMエンコード失敗")
        .to_string();

    // ヘッダーとフッターを除いたBase64文字列だけを返す
    let base64_secret = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<Vec<_>>()
        .join("");

    return base64_secret;
}