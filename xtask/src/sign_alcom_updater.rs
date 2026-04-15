// TAURI_SIGNING_PRIVATE_KEY
// TAURI_SIGNING_PRIVATE_KEY_PASSWORD

use anyhow::{Context, Result};
use base64::Engine;
use minisign::{SecretKey, SignatureBox};
use std::fs;
use std::path::{Path, PathBuf};

/// Signs updater artifact
#[derive(clap::Parser)]
pub struct Command {
    #[arg()]
    file: PathBuf,
}

impl crate::Command for Command {
    fn run(self) -> anyhow::Result<i32> {
        let private_key = std::env::var("TAURI_SIGNING_PRIVATE_KEY")
            .context("Required environment variable TAURI_SIGNING_PRIVATE_KEY")?;
        let password = std::env::var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD")
            .context("Required environment variable TAURI_SIGNING_PRIVATE_KEY_PASSWORD")?;

        let signature = sign_file(&secret_key(&private_key, &password)?, &self.file)
            .with_context(|| "failed to sign file")?;

        let signature_path = self.file.with_added_extension("sig");

        let encoded_signature =
            base64::engine::general_purpose::STANDARD.encode(signature.to_string());

        fs::write(&signature_path, &encoded_signature).with_context(|| {
            format!(
                "failed to write signature file: {}",
                signature_path.display()
            )
        })?;

        println!(
            "Your file was signed successfully, You can find the signature here:\n\
            {signature_path}\n
            \n\
            Public signature:\n\
            {encoded_signature}\
            \n
            \n\
            Make sure to include this into the signature field of your update server.",
            signature_path = signature_path.display(),
        );

        Ok(0)
    }
}

fn secret_key(private_key: &str, password: &str) -> Result<SecretKey> {
    let decoded_secret = base64::engine::general_purpose::STANDARD
        .decode(private_key)
        .map_err(anyhow::Error::from)
        .and_then(|x| String::from_utf8(x).map_err(anyhow::Error::from))
        .context("failed to decode base64 secret key")?;

    let sk_box = minisign::SecretKeyBox::from_string(&decoded_secret)
        .context("failed to load updater private key")?;
    let sk = sk_box
        .into_secret_key(Some(password.into()))
        .context("incorrect updater private key password")?;
    Ok(sk)
}

pub fn sign_file(secret_key: &SecretKey, bin_path: &Path) -> Result<SignatureBox> {
    let trusted_comment = format!(
        "timestamp:{}\tfile:{}",
        unix_timestamp(),
        bin_path.file_name().unwrap().to_string_lossy()
    );

    let data_reader = fs::File::open(bin_path).context("failed to open data file")?;

    let signature_box = minisign::sign(
        None,
        secret_key,
        data_reader,
        Some(trusted_comment.as_str()),
        Some("signature from tauri secret key"),
    )
    .context("failed to sign file")?;

    Ok(signature_box)
}

fn unix_timestamp() -> u64 {
    let start = std::time::SystemTime::now();
    let since_the_epoch = start
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is incorrect");
    since_the_epoch.as_secs()
}
