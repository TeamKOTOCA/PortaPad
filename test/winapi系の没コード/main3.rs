use hidapi::{HidApi, HidDevice};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 検索したいデバイスのベンダーIDとプロダクトIDを設定します
    // これらはあなたのHIDデバイスによって異なります。
    // 例: VID=0xABCD, PID=0x1234
    let target_vid: u16 = 0x0930; // 実際のデバイスのベンダーIDに置き換えてください
    let target_pid: u16 = 0x0323; // 実際のデバイスのプロダクトIDに置き換えてください

    let api = HidApi::new()?;

    println!("利用可能なHIDデバイスを検索中...");

    let mut found_device: Option<HidDevice> = None;

    for device_info in api.device_list() {
        println!(
            "  VID: 0x{:04x}, PID: 0x{:04x}, 製品名: {:?}, 製造元: {:?}",
            device_info.vendor_id(),
            device_info.product_id(),
            device_info.product_string(),
            device_info.manufacturer_string()
        );

        if device_info.vendor_id() == target_vid && device_info.product_id() == target_pid {
            println!("ターゲットデバイスを発見しました！");
            // デバイスを開く
            match api.open_path(device_info.path()) {
                Ok(device) => {
                    found_device = Some(device);
                    break; // 最初の見つかったデバイスを使用
                },
                Err(e) => {
                    eprintln!("デバイスのオープンに失敗しました: {:?}", e);
                }
            }
        }
    }

    match found_device {
        Some(device) => {
            println!("デバイスを開きました。入力レポートを読み取ります...");

            // 読み取るバッファのサイズ。HIDレポートのサイズに依存します。
            // 通常、HIDレポートディスクリプタで定義されています。
            // ここでは仮に64バイトとしています。
            let mut buf = [0u8; 64];

            loop {
                // read_timeoutはブロッキングで、指定したタイムアウト時間まで待機します。
                // 0を指定するとノンブロッキングになりますが、通常はタイムアウトを設定します。
                match device.read_timeout(&mut buf, 0) { // タイムアウト1000ms
                    Ok(bytes_read) => {
                        if bytes_read > 0 {
                            println!("読み取りました ({}バイト): {:?}", bytes_read, &buf[..bytes_read]);
                            // ここで読み取ったデータを処理します
                            // 例えば、キーボードのキーコードやマウスの座標など
                        } else {
                            // タイムアウトしてもデータがなかった場合
                            // println!("データなし (タイムアウト)");
                        }
                    },
                    Err(e) => {
                        eprintln!("読み取りエラー: {:?}", e);
                        break;
                    }
                }
                // 短い時間待機してCPU使用率を抑える
                std::thread::sleep(Duration::from_millis(10));
            }
        },
        None => {
            eprintln!(
                "指定されたVID (0x{:04x}) とPID (0x{:04x}) のデバイスは見つかりませんでした。",
                target_vid, target_pid
            );
        }
    }

    Ok(())
}