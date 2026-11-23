// settconv.toml next to the executable. Every field is optional.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Default)]
pub struct Config {
    pub jpg_quality: Option<u8>,
    pub webp_quality: Option<u8>,
    pub ffmpeg: Option<String>,
    pub seven_zip: Option<String>,
}

impl Config {
    pub fn jpg_quality(&self) -> u8 {
        self.jpg_quality.unwrap_or(90).clamp(1, 100)
    }

    pub fn webp_quality(&self) -> u8 {
        self.webp_quality.unwrap_or(85).clamp(1, 100)
    }
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn load() -> Config {
    std::fs::read_to_string(exe_dir().join("settconv.toml"))
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}
