//! The module shared between ALCOM and ALCOM online updater

use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::{Deserialize, Deserializer};
use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;

pub static PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDkyMjAzMkU2Q0ZGQjQ0MjYKUldRbVJQdlA1aklna2d2NnRoM3ZsT3lzWEQ3MC9zTGpaWVR4NGdQOXR0UGJaOHBlY2xCcFY5bHcK";

pub fn get_updater_url(stable: bool) -> Cow<'static, str> {
    if let Ok(from_env) =
        std::env::var("___ALCOM_UPDATER_URL_OVERRIDE_DEBUG_ONLY_FEATURE_YOU_SHOULD_NOT_USE_THIS___")
    {
        from_env.into()
    } else if stable {
        "https://vrc-get.anatawa12.com/api/gui/tauri-updater.json".into()
    } else {
        "https://vrc-get.anatawa12.com/api/gui/tauri-updater-beta.json".into()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseManifestPlatform {
    pub url: String,
    pub signature: String,
    // alcom specific information
    /// Command line parameters for windows installer
    ///
    /// If one of arg is prefixed with '!', such parameters have special handling.
    /// If the updater cannot process ! args, such parameters will be ignored.
    ///
    /// Current ! operations are shown below:
    /// - `!peruser:` appended only if t installation is user installuser install is active
    /// - `!current installation is machine install
    // /// - `!install-path:` substitute `${installed}` with currently installed dir. // initially planned but not implemented.
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Deserialize)]
pub struct RemoteRelease {
    #[serde(alias = "name", deserialize_with = "parse_version")]
    pub version: Version,
    pub notes: Option<String>,
    pub platforms: HashMap<String, ReleaseManifestPlatform>,
}

fn parse_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Version;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a semver version")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Version::from_str(v.trim_start_matches('v'))
                .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(v), &self))
        }
    }

    deserializer.deserialize_str(Visitor)
}

// ---------------------------------------------------------------------------
// Signature verification
// ---------------------------------------------------------------------------

pub enum VerifySignatureError {
    InvalidBase64(base64::DecodeError),
    MiniSignError(minisign_verify::Error),
    SignatureIsNotUtf8,
}

pub fn verify_signature(
    data: &[u8],
    release_signature: &str,
    pub_key: &str,
) -> Result<bool, VerifySignatureError> {
    if std::env::var(
        "___ALCOM_UPDATER_DISABLE_SIGNATURE_VERIFICATION_DEBUG_ONLY_FEATURE_DO_NOT_USE_THIS_OR_YOU_WILL_BE_HACKED___",
    )
        .as_deref()
        == Ok("YES_I_WANT_TO_BE_HACKED")
    {
        return Ok(true);
    }
    let pub_key_decoded = base64_to_string(pub_key)?;
    let public_key =
        PublicKey::decode(&pub_key_decoded).map_err(VerifySignatureError::MiniSignError)?;
    let sig_decoded = base64_to_string(release_signature)?;
    let signature = Signature::decode(&sig_decoded).map_err(VerifySignatureError::MiniSignError)?;
    public_key
        .verify(data, &signature, true)
        .map_err(VerifySignatureError::MiniSignError)?;
    Ok(true)
}

fn base64_to_string(base64_string: &str) -> Result<String, VerifySignatureError> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(base64_string)
        .map_err(VerifySignatureError::InvalidBase64)?;

    std::str::from_utf8(&decoded)
        .map(|s| s.to_string())
        .map_err(|_| VerifySignatureError::SignatureIsNotUtf8)
}
