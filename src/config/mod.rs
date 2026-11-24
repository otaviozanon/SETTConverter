// Settings\SETTConverter.toml. Created on first run; every field is optional.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

mod template;

use template::{TEMPLATE_EN, TEMPLATE_PT};

const FILE: &str = "SETTConverter.toml";

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub jpg_quality: Option<u8>,
    pub webp_quality: Option<u8>,
    pub avif_quality: Option<u8>,
    pub png_compression: Option<u8>,
    pub gif_quality: Option<u8>,
    pub ico_size: Option<u32>,
    pub pdf_width: Option<u32>,
    pub pdf_dpi: Option<u32>,
    pub pdf_jpeg: Option<bool>,
    pub image_pdf_dpi: Option<u32>,
    pub video_quality: Option<u8>,
    pub video_preset: Option<String>,
    pub audio_bitrate: Option<String>,
    pub gif_fps: Option<u32>,
    pub gif_width: Option<u32>,
    pub gpu: Option<bool>,
    pub strip: Option<bool>,
    pub subtitle_encoding: Option<String>,
    pub ffmpeg: Option<String>,
    pub seven_zip: Option<String>,
    pub out_dir: Option<String>,
    pub overwrite: Option<bool>,
    pub download_tools: Option<bool>,
}

impl Config {
    pub fn jpg_quality(&self) -> u8 {
        self.jpg_quality.unwrap_or(90).clamp(1, 100)
    }

    pub fn webp_quality(&self) -> u8 {
        self.webp_quality.unwrap_or(85).clamp(1, 100)
    }

    pub fn avif_quality(&self) -> u8 {
        self.avif_quality
            .or(self.jpg_quality)
            .unwrap_or(90)
            .clamp(1, 100)
    }

    /// Quality for a lossy image target, used where the caller has no explicit value.
    pub fn quality(&self, to: &str) -> u8 {
        match to {
            "webp" => self.webp_quality(),
            "avif" => self.avif_quality(),
            _ => self.jpg_quality(),
        }
    }

    pub fn png_compression(&self) -> u8 {
        self.png_compression.unwrap_or(1).min(9)
    }

    pub fn gif_quality(&self) -> u8 {
        self.gif_quality.unwrap_or(100).clamp(1, 100)
    }

    pub fn ico_size(&self) -> u32 {
        self.ico_size.unwrap_or(256).clamp(1, 256)
    }

    pub fn pdf_width(&self) -> Option<u32> {
        self.pdf_width.map(|width| width.clamp(1, 100_000))
    }

    pub fn pdf_dpi(&self) -> Option<u32> {
        self.pdf_dpi.map(|dpi| dpi.clamp(1, 2400))
    }

    pub fn pdf_jpeg(&self) -> bool {
        self.pdf_jpeg.unwrap_or(false)
    }

    pub fn image_pdf_dpi(&self) -> f32 {
        self.image_pdf_dpi.unwrap_or(96).clamp(1, 2400) as f32
    }

    /// Empty means keep each codec's current default CRF.
    pub fn video_quality(&self) -> Option<u8> {
        self.video_quality.map(|quality| quality.clamp(1, 100))
    }

    pub fn video_preset(&self) -> &str {
        let preset = self.video_preset.as_deref().unwrap_or("medium");
        match preset {
            "ultrafast" | "superfast" | "veryfast" | "faster" | "fast" | "medium" | "slow"
            | "slower" | "veryslow" => preset,
            _ => "medium",
        }
    }

    /// Empty means keep the default bitrate of the codec.
    pub fn audio_bitrate(&self) -> Option<&str> {
        self.audio_bitrate
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }

    pub fn gif_fps(&self) -> u32 {
        self.gif_fps.unwrap_or(15).clamp(1, 60)
    }

    pub fn gif_width(&self) -> u32 {
        self.gif_width.unwrap_or(480).clamp(16, 8192)
    }

    pub fn gpu(&self) -> bool {
        self.gpu.unwrap_or(false)
    }

    pub fn strip(&self) -> bool {
        self.strip.unwrap_or(false)
    }

    pub fn subtitle_encoding(&self) -> Option<&str> {
        self.subtitle_encoding
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }

    pub fn download_tools(&self) -> bool {
        self.download_tools.unwrap_or(true)
    }
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Next to the exe, then %LOCALAPPDATA% for read-only installs.
fn dirs() -> [PathBuf; 2] {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    [
        exe_dir().join("SETTConverter").join("Settings"),
        base.join("SETTConverter").join("Settings"),
    ]
}

pub fn path() -> PathBuf {
    let dirs = dirs();
    dirs.iter()
        .map(|dir| dir.join(FILE))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| dirs[0].join(FILE))
}

/// Writes the commented template if no config exists yet.
pub fn ensure() {
    if path().is_file() {
        return;
    }
    for dir in dirs() {
        let template = if crate::lang::is_pt() {
            TEMPLATE_PT
        } else {
            TEMPLATE_EN
        };
        if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(dir.join(FILE), template).is_ok()
        {
            return;
        }
    }
}

pub fn load() -> Result<Config> {
    load_at(&path())
}

fn load_at(file: &Path) -> Result<Config> {
    if !file.is_file() {
        return Ok(Config::default());
    }
    let text = std::fs::read_to_string(file)
        .with_context(|| format!("{} {}", crate::lang::t("ler", "read"), file.display()))?;
    toml::from_str(&text)
        .with_context(|| format!("{} {}", crate::lang::t("corrigir", "fix"), file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_means_defaults() {
        let file = std::env::temp_dir().join("settconv-missing-config.toml");
        let _ = std::fs::remove_file(&file);
        assert_eq!(load_at(&file).unwrap().jpg_quality(), 90);
    }

    #[test]
    fn broken_file_is_reported() {
        let file =
            std::env::temp_dir().join(format!("settconv-broken-{}.toml", std::process::id()));
        std::fs::write(&file, "jpg_quality = ").unwrap();
        let error = load_at(&file).unwrap_err().to_string();
        assert!(
            error.contains("corrigir") || error.contains("fix"),
            "{error}"
        );
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn quality_follows_the_target_and_clamps() {
        let config = Config {
            jpg_quality: Some(255),
            webp_quality: Some(0),
            avif_quality: Some(50),
            ..Default::default()
        };
        assert_eq!(config.quality("jpg"), 100);
        assert_eq!(config.quality("webp"), 1);
        assert_eq!(config.quality("avif"), 50);
        assert_eq!(config.quality("png"), 100);
    }

    #[test]
    fn avif_falls_back_to_jpg_quality() {
        let config = Config {
            jpg_quality: Some(70),
            ..Default::default()
        };
        assert_eq!(config.avif_quality(), 70);
    }

    #[test]
    fn media_settings_have_safe_defaults() {
        let config = Config::default();
        assert_eq!(config.png_compression(), 1);
        assert_eq!(config.video_quality(), None);
        assert_eq!(config.video_preset(), "medium");
        assert_eq!(config.audio_bitrate(), None);
        assert_eq!(config.gif_fps(), 15);
        assert_eq!(config.gif_width(), 480);
        assert_eq!(config.ico_size(), 256);
        assert_eq!(config.image_pdf_dpi(), 96.0);
        assert!(config.download_tools());
    }

    #[test]
    fn invalid_preset_falls_back_to_medium() {
        let config = Config {
            video_preset: Some("turbo".into()),
            ..Default::default()
        };
        assert_eq!(config.video_preset(), "medium");
    }
}
