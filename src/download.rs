// Download helpers: curl with a PowerShell fallback, plus SHA-256 checking.

use anyhow::{Context, Result, bail};
use std::fs::File;
use std::os::windows::process::CommandExt;
use std::path::Path;

use crate::lang::t;
use crate::win::CREATE_NO_WINDOW;

pub const TOOLS_FILE: &str = "tools-win64.zip";

fn tool_urls() -> [String; 2] {
    let repo = env!("CARGO_PKG_REPOSITORY");
    [
        // Versioned first: an old exe never installs a newer, incompatible bundle.
        format!(
            "{repo}/releases/download/v{}/{TOOLS_FILE}",
            env!("CARGO_PKG_VERSION")
        ),
        format!("{repo}/releases/latest/download/{TOOLS_FILE}"),
    ]
}

/// Downloads the tools bundle, checking it against the published SHA-256.
pub fn bundle(out: &Path) -> Result<()> {
    let mut last: Option<anyhow::Error> = None;
    for url in tool_urls() {
        match download(&url, out).and_then(|()| verify_hash(out, &url)) {
            Ok(()) => return Ok(()),
            Err(error) => last = Some(error),
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("download failed")))
}

fn verify_hash(zip: &Path, url: &str) -> Result<()> {
    let checksum_url = format!("{}.sha256", url.trim_end_matches(".zip"));
    let expected = fetch_text(&checksum_url)?;
    let expected = expected
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_lowercase();
    let actual = sha256_file(zip)?;
    if expected.is_empty() || expected != actual {
        bail!(
            "{} {}",
            t("SHA-256 não confere para", "SHA-256 mismatch for"),
            zip.display()
        );
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn fetch_text(url: &str) -> Result<String> {
    let output = std::process::Command::new("curl.exe")
        .args(["-L", "--fail", "--silent", "--show-error"])
        .arg(url)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match output {
        Ok(result) if result.status.success() => {
            Ok(String::from_utf8_lossy(&result.stdout).to_string())
        }
        Ok(result) => bail!(
            "{} {}",
            t("download falhou:", "download failed:"),
            String::from_utf8_lossy(&result.stderr).trim()
        ),
        Err(_) => {
            let script = format!(
                "(Invoke-WebRequest -UseBasicParsing -Uri '{}').Content",
                url.replace('\'', "''")
            );
            let result = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .context(t(
                    "executar curl nem PowerShell",
                    "running curl or PowerShell",
                ))?;
            if !result.status.success() {
                bail!(
                    "{} {}",
                    t("download falhou:", "download failed:"),
                    String::from_utf8_lossy(&result.stderr).trim()
                );
            }
            Ok(String::from_utf8_lossy(&result.stdout).to_string())
        }
    }
}

fn download(url: &str, out: &Path) -> Result<()> {
    let curl = std::process::Command::new("curl.exe")
        .args(["-L", "--fail", "--silent", "--show-error", "--output"])
        .arg(out)
        .arg(url)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match curl {
        Ok(result) if result.status.success() && out.is_file() => return Ok(()),
        Ok(result) => bail!(
            "{} {}",
            t("download falhou:", "download failed:"),
            String::from_utf8_lossy(&result.stderr).trim()
        ),
        // Windows 10 before 1803 has no curl.exe: fall back to PowerShell.
        Err(_) => {}
    }
    let script = format!(
        "Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'",
        url.replace('\'', "''"),
        out.display().to_string().replace('\'', "''")
    );
    let result = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context(t(
            "executar curl nem PowerShell",
            "running curl or PowerShell",
        ))?;
    if !result.status.success() || !out.is_file() {
        bail!(
            "{} {}",
            t("download falhou:", "download failed:"),
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    Ok(())
}
