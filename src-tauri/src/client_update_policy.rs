use base64::Engine;
use url::Url;

pub fn validate_download_url(url: &Url, version: &str) -> Result<(), String> {
    let prefix = format!("/Mars-Command/Client-Launcher/releases/download/client-v{version}/");
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(
            url.path().strip_prefix(&prefix),
            Some(name) if name == format!("setup-{version}.exe")
                || name == format!("mars-command-{version}.AppImage")
        )
    {
        return Err("Update download is not a versioned Mars Command GitHub release asset".into());
    }
    Ok(())
}

pub fn verify_artifact(
    bytes: &[u8],
    public_key: &str,
    signature: &str,
    version: &str,
) -> Result<(), String> {
    let decode = |value: &str| -> Result<String, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(value)
            .map_err(|err| format!("Invalid updater signature encoding: {err}"))?;
        String::from_utf8(bytes).map_err(|err| format!("Invalid updater signature text: {err}"))
    };
    let key = minisign_verify::PublicKey::decode(&decode(public_key)?)
        .map_err(|err| format!("Invalid updater public key: {err}"))?;
    let signature = minisign_verify::Signature::decode(&decode(signature)?)
        .map_err(|err| format!("Invalid updater signature: {err}"))?;
    key.verify(bytes, &signature, true)
        .map_err(|err| format!("Signature verification failed: {err}"))?;
    let signed_version = signature
        .trusted_comment()
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"));
    if signed_version != Some(version) {
        return Err(
            "The signed artifact version does not match the announced update version".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The disposable test signing key was deleted; these public fixtures cannot sign releases.
    const BYTES: &[u8] = b"Mars Command updater test artifact. Not an executable.\n";
    const KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEVCQ0Q0QTlDQ0ZERUY0MEEKUldRSzlON1BuRXJONjdxTjZsbFFPVkhGOE1KNHRaUndzVmwrYUJNbHFuMHBYQWpVdzFlaUV0TTEK";
    const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRSzlON1BuRXJONjRvdVpuZWRGYzlnMitNUlJ4SG8xcjBJNlUzbERIYW5xUXVnemluVUE0TkVQTE1PbkVpRk9LNnB3R1NibVpCckNsZ093ZW11aVY1bm80ajJwaGROcWdRPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkxNzY2NzQ2CWZpbGU6Zml4dHVyZS50eHQJdmVyc2lvbjoxLjUuMApXWWVPRThiL1QxSGhxaDN6Ny9KVVd6a29XUHpETStmcXRlQVpiNTVYZkxmWFFNY2JxQWd4cUxLazRISzl5NDdHaE84MzRVbmFhR0t3UHVvTkEyR0xEZz09Cg==";

    #[test]
    fn valid_signature_and_signed_version_are_required() {
        assert!(verify_artifact(BYTES, KEY, SIGNATURE, "1.5.0").is_ok());
        assert!(verify_artifact(b"Fake update", KEY, SIGNATURE, "1.5.0").is_err());
        assert!(verify_artifact(BYTES, KEY, SIGNATURE, "9.9.9").is_err());
        assert!(verify_artifact(BYTES, KEY, "not a signature", "1.5.0").is_err());
        assert!(verify_artifact(BYTES, "not a public key", SIGNATURE, "1.5.0").is_err());
    }

    #[test]
    fn changing_the_signed_comment_does_not_allow_a_fake_version() {
        let decoded = String::from_utf8(
            base64::engine::general_purpose::STANDARD
                .decode(SIGNATURE)
                .unwrap(),
        )
        .unwrap();
        let tampered = base64::engine::general_purpose::STANDARD
            .encode(decoded.replace("version:1.5.0", "version:9.9.9"));
        assert!(verify_artifact(BYTES, KEY, &tampered, "9.9.9").is_err());
    }

    #[test]
    fn only_versioned_official_https_assets_are_allowed() {
        for asset in ["setup-1.5.0.exe", "mars-command-1.5.0.AppImage"] {
            let url = Url::parse(&format!(
                "https://github.com/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/{asset}"
            )).unwrap();
            assert!(validate_download_url(&url, "1.5.0").is_ok());
            assert!(validate_download_url(&url, "1.4.0").is_err());
        }
        for url in [
            "http://github.com/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/setup-1.5.0.exe",
            "https://github.com.evil.test/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/setup-1.5.0.exe",
            "https://github.com/attacker/Client-Launcher/releases/download/client-v1.5.0/setup-1.5.0.exe",
            "https://github.com/Mars-Command/Client-Launcher/releases/latest/download/setup-1.5.0.exe",
            "https://user@github.com/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/setup-1.5.0.exe",
            "https://github.com/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/setup-1.5.0.exe?override=1",
            "https://github.com/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/setup-1.5.0.exe#override",
            "https://github.com:8443/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/setup-1.5.0.exe",
            "https://github.com/Mars-Command/Client-Launcher/releases/download/client-v1.5.0/setup-1.5.0.exe/extra",
        ] {
            assert!(
                validate_download_url(&Url::parse(url).unwrap(), "1.5.0").is_err(),
                "{url}"
            );
        }
    }
}
