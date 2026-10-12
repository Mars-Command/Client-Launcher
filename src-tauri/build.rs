fn main() {
    use base64::Engine;

    println!("cargo:rerun-if-env-changed=MARS_UPDATER_PUBLIC_KEY");
    let public_key = std::env::var("MARS_UPDATER_PUBLIC_KEY").unwrap_or_default();
    if public_key.trim().is_empty() {
        assert!(
            std::env::var("PROFILE").as_deref() != Ok("release"),
            "Release builds require MARS_UPDATER_PUBLIC_KEY. Never ship an updater without its pinned verification key."
        );
    } else {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(public_key.trim())
            .expect("MARS_UPDATER_PUBLIC_KEY must be the base64 contents of the Tauri .pub file");
        let decoded =
            std::str::from_utf8(&decoded).expect("Updater public key must decode to text");
        minisign_verify::PublicKey::decode(decoded).expect("Invalid updater public key");
        println!(
            "cargo:rustc-env=MARS_UPDATER_PUBLIC_KEY={}",
            public_key.trim()
        );
    }

    // Public deployment configuration only; never embed desktop/provider tokens.
    for key in [
        "MARS_COMMUNITY_API_BASE",
        "MARS_WEBSITE_BASE",
        "MARS_SPONSORS_URL",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
        if let Ok(value) = std::env::var(key) {
            println!("cargo:rustc-env={key}={value}");
        }
    }
    tauri_build::build()
}
