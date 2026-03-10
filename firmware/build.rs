use std::{env, fs};

fn main() {
    let secrets_path = env::var("SECRETS_FILE").expect("SECRETS_FILE not set");
    let secrets_bytes = fs::read(&secrets_path).expect("Failed to read secrets file");
    let secrets: serde_json::Value = serde_json::from_slice(&secrets_bytes)
        .expect("Failed to parse secrets file");
    let pin_hex = secrets["pin"].as_str().expect("pin not found in secrets");
    println!("cargo:rustc-env=DEVICE_PIN={}", pin_hex);
    println!("cargo:rerun-if-changed={}", secrets_path);
}
