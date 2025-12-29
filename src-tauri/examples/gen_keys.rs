//! 生成 Ed25519 密钥对示例
//!
//! 运行方式: cargo run --example gen_keys
//!
//! 输出:
//! - 私钥 (32 bytes hex): 需要安全保管，用于签名
//! - 公钥 (32 bytes Rust 数组): 复制到 signature.rs 中的 OFFICIAL_PUBLIC_KEY

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

fn main() {
    // 生成新的密钥对
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    // 输出私钥 (hex 格式)
    println!("=== Ed25519 密钥对生成 ===\n");

    println!("🔒 私钥 (32 bytes, hex 格式):");
    println!("   ⚠️  请安全保管！不要提交到 Git！");
    let private_hex: String = signing_key
        .to_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    println!("   {}\n", private_hex);

    // 输出公钥 (Rust 数组格式)
    println!("🔓 公钥 (32 bytes, Rust 数组格式):");
    println!("   复制到 signature.rs 中的 OFFICIAL_PUBLIC_KEY");
    let public_bytes = verifying_key.as_bytes();
    print!("   [");
    for (i, byte) in public_bytes.iter().enumerate() {
        if i > 0 && i % 14 == 0 {
            print!("\n    ");
        }
        print!("0x{:02x}", byte);
        if i < 31 {
            print!(", ");
        }
    }
    println!("]");

    println!("\n=== 完成 ===");
}
