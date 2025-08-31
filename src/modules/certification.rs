use ed25519_dalek::{
    SigningKey, VerifyingKey, Signature, Signer, Verifier,
    pkcs8::{EncodePublicKey, DecodePublicKey},
};
use base64::{engine::general_purpose, Engine};
use std::convert::TryInto;

use crate::open_setting;

pub fn certification(signature: String, private_key: String, public_key: String, pc_code: String) -> Result<(), i32> {
    /*
    println!("📨 code（署名対象）: {}", pc_code);
    println!("📨 publickey: {}", public_key);
    println!("📨 privatekey: {}", private_key);
    println!("📨 signature: {}", signature);
     */

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
            eprintln!("❌ 検証失敗(認証コードが違います): {}", e);
            return Err(1);
        }
    }
}