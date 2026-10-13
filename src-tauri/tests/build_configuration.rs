#[path = "../build_config.rs"]
mod build_config;

use std::collections::BTreeMap;
use std::path::Path;

fn defaults() -> BTreeMap<String, String> {
    build_config::read_public_file(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("config/production.env"),
        true,
    )
    .unwrap()
}

#[test]
fn native_compilation_receives_all_public_settings_without_a_runtime_env_file() {
    let compiled: BTreeMap<String, String> = BTreeMap::from([
        (
            "MARS_MANIFEST_BASE_URL".into(),
            env!("MARS_MANIFEST_BASE_URL").into(),
        ),
        (
            "MARS_MANIFEST_PUBLIC_KEY".into(),
            env!("MARS_MANIFEST_PUBLIC_KEY").into(),
        ),
        (
            "MARS_COMMUNITY_API_BASE".into(),
            env!("MARS_COMMUNITY_API_BASE").into(),
        ),
        ("MARS_WEBSITE_BASE".into(), env!("MARS_WEBSITE_BASE").into()),
        ("MARS_SPONSORS_URL".into(), env!("MARS_SPONSORS_URL").into()),
        (
            "MARS_UPDATER_PUBLIC_KEY".into(),
            env!("MARS_UPDATER_PUBLIC_KEY").into(),
        ),
    ]);
    assert!(!compiled["MARS_COMMUNITY_API_BASE"].is_empty());
    assert!(!compiled["MARS_WEBSITE_BASE"].is_empty());
    build_config::resolve_public_config(&BTreeMap::new(), &BTreeMap::new(), &compiled, false)
        .unwrap();
}

#[test]
fn ci_checkout_has_complete_public_configuration_without_private_dotenv_files() {
    let config =
        build_config::resolve_public_config(&defaults(), &BTreeMap::new(), &BTreeMap::new(), true)
            .unwrap();
    assert_eq!(config.len(), build_config::PUBLIC_KEYS.len());
    assert_eq!(
        config["MARS_COMMUNITY_API_BASE"],
        "https://api.nexusgit.info"
    );
    assert_eq!(config["MARS_WEBSITE_BASE"], "https://mars.nexusgit.info");
    assert_eq!(
        config["MARS_MANIFEST_PUBLIC_KEY"],
        "a88e7dc70a1227cbc3decac967664e3bea75da9d72550e14f2c6a4ea8118391e"
    );
}

#[test]
fn local_prod_file_is_filtered_and_never_embeds_secrets_or_tauri_debug_flags() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join(".env.prod");
    std::fs::write(
        &file,
        "JWT_SECRET=never-embed-this-secret\n\
         TAURI_SIGNING_PRIVATE_KEY=not-a-real-private-key\n\
         TAURI_ENV_DEBUG=true\n\
         MARS_COMMUNITY_API_BASE=https://custom.example\n",
    )
    .unwrap();
    let local = build_config::read_public_file(&file, false).unwrap();
    let config =
        build_config::resolve_public_config(&defaults(), &local, &BTreeMap::new(), true).unwrap();
    assert_eq!(config["MARS_COMMUNITY_API_BASE"], "https://custom.example");
    for forbidden in ["JWT_SECRET", "TAURI_SIGNING_PRIVATE_KEY", "TAURI_ENV_DEBUG"] {
        assert!(!config.contains_key(forbidden));
    }
    assert!(config.values().all(|value| !value.contains("never-embed")));
}

#[test]
fn explicit_process_values_override_local_files_then_versioned_defaults() {
    let local = BTreeMap::from([(
        "MARS_COMMUNITY_API_BASE".into(),
        "https://local.example".into(),
    )]);
    let environment = BTreeMap::from([
        (
            "MARS_COMMUNITY_API_BASE".into(),
            "https://ci.example".into(),
        ),
        ("MARS_SPONSORS_URL".into(), String::new()),
    ]);
    let config =
        build_config::resolve_public_config(&defaults(), &local, &environment, true).unwrap();
    assert_eq!(config["MARS_COMMUNITY_API_BASE"], "https://ci.example");
    assert_eq!(config["MARS_SPONSORS_URL"], "");
    assert_eq!(config["MARS_WEBSITE_BASE"], defaults()["MARS_WEBSITE_BASE"]);
}

#[test]
fn legacy_api_origin_maps_to_pack_endpoint_without_overriding_explicit_base() {
    let local = BTreeMap::from([(
        "MARS_COMMAND_API_URL".into(),
        "https://configured.example".into(),
    )]);
    let config =
        build_config::resolve_public_config(&defaults(), &local, &BTreeMap::new(), true).unwrap();
    assert_eq!(
        config["MARS_MANIFEST_BASE_URL"],
        "https://configured.example/api/v1"
    );
    assert!(!config.contains_key("MARS_COMMAND_API_URL"));
    let environment = BTreeMap::from([(
        "MARS_MANIFEST_BASE_URL".into(),
        "https://other.example/releases".into(),
    )]);
    let config =
        build_config::resolve_public_config(&defaults(), &local, &environment, true).unwrap();
    assert_eq!(
        config["MARS_MANIFEST_BASE_URL"],
        "https://other.example/releases"
    );
}

#[test]
fn invalid_public_values_fail_without_printing_the_value() {
    for (key, value) in [
        ("MARS_COMMUNITY_API_BASE", "https://example/api"),
        (
            "MARS_MANIFEST_BASE_URL",
            "https://example/path?token=sensitive-fixture",
        ),
        ("MARS_MANIFEST_PUBLIC_KEY", "00"),
        ("MARS_UPDATER_PUBLIC_KEY", "not-a-public-key"),
        ("MARS_SPONSORS_URL", "https://attacker.example/donate"),
    ] {
        let environment = BTreeMap::from([(key.to_string(), value.to_string())]);
        let error =
            build_config::resolve_public_config(&defaults(), &BTreeMap::new(), &environment, true)
                .unwrap_err();
        assert!(!error.contains("sensitive-fixture"));
    }
}

#[test]
fn malformed_private_file_errors_do_not_echo_credential_text() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join(".env.prod");
    std::fs::write(&file, "JWT_SECRET='sensitive-fixture-without-closing-quote").unwrap();
    let error = build_config::read_public_file(&file, false).unwrap_err();
    assert!(!error.contains("sensitive-fixture"));
}

#[test]
fn versioned_public_defaults_reject_accidental_private_settings() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("production.env");
    std::fs::write(&file, "JWT_SECRET=secret-fixture").unwrap();
    let error = build_config::read_public_file(&file, true).unwrap_err();
    assert!(error.contains("unapproved setting"));
    assert!(!error.contains("secret-fixture"));
}

#[test]
fn missing_optional_file_is_allowed_but_missing_defaults_fail() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("missing.env");
    assert!(
        build_config::read_public_file(&file, false)
            .unwrap()
            .is_empty()
    );
    assert!(build_config::read_public_file(&file, true).is_err());
}
