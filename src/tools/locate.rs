// Find FFmpeg/7-Zip on PATH, in standard install folders, or in the cache.

use anyhow::{Result, bail};
use std::path::PathBuf;

use crate::config::Config;
use crate::lang::t;

pub fn ffmpeg(cfg: &Config) -> Result<PathBuf> {
    find(cfg.ffmpeg.as_deref(), "ffmpeg.exe", cfg.download_tools())
}

pub fn seven_zip(cfg: &Config) -> Result<PathBuf> {
    find(cfg.seven_zip.as_deref(), "7z.exe", cfg.download_tools())
}

fn find(configured: Option<&str>, name: &str, download: bool) -> Result<PathBuf> {
    if let Some(configured) = configured {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Ok(path);
        }
    }
    if let Some(path) = locate(name) {
        return Ok(path);
    }
    let cached = dir().join(name);
    if cached.is_file() {
        return Ok(cached);
    }
    if !download {
        bail!(
            "{} {name}. {}",
            t("não encontrado:", "not found:"),
            t(
                "instale-o ou defina o caminho no SETTConverter.toml",
                "install it or set its path in SETTConverter.toml"
            )
        );
    }
    super::ensure()?;
    if cached.is_file() {
        Ok(cached)
    } else {
        bail!(
            "{} {name}",
            t(
                "não encontrado após o download:",
                "not found after download:"
            )
        )
    }
}

/// Already usable on this PC: on PATH or in a standard install location.
fn locate(name: &str) -> Option<PathBuf> {
    let on_path = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    });
    on_path.or_else(|| installed(name))
}

fn installed(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if name == "7z.exe" {
        for base in [user_env("ProgramFiles"), user_env("ProgramFiles(x86)")]
            .into_iter()
            .flatten()
        {
            candidates.push(base.join("7-Zip").join("7z.exe"));
        }
    } else {
        if let Some(local) = user_env("LOCALAPPDATA") {
            candidates.push(
                local
                    .join("Microsoft")
                    .join("WinGet")
                    .join("Links")
                    .join("ffmpeg.exe"),
            );
        }
        if let Some(home) = user_env("USERPROFILE") {
            candidates.push(home.join("scoop").join("shims").join("ffmpeg.exe"));
        }
        candidates.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin\ffmpeg.exe"));
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn user_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

pub fn dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("SETTConverter")
        .join("tools")
}
