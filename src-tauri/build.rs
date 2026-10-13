mod build_config;

fn main() {
    let manifest_dir = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest root"),
    );
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    let defaults_path = manifest_dir.join("config/production.env");
    let local_path = manifest_dir
        .parent()
        .expect("Client repository root")
        .join(if release { ".env.prod" } else { ".env.dev" });

    println!("cargo:rerun-if-changed={}", defaults_path.display());
    println!("cargo:rerun-if-changed={}", local_path.display());
    let defaults =
        build_config::read_public_file(&defaults_path, true).expect("Public production defaults");
    let local = build_config::read_public_file(&local_path, false).expect("Local public overrides");
    let mut environment = std::collections::BTreeMap::new();
    for key in build_config::PUBLIC_KEYS
        .into_iter()
        .chain([build_config::LEGACY_API_KEY])
    {
        println!("cargo:rerun-if-env-changed={key}");
        if let Ok(value) = std::env::var(key) {
            environment.insert(key.to_string(), value);
        }
    }
    let values = build_config::resolve_public_config(&defaults, &local, &environment, release)
        .expect("Invalid public client build configuration");
    for (key, value) in values {
        println!("cargo:rustc-env={key}={value}");
    }
    tauri_build::build()
}
