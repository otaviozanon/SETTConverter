// Settings\SETTConverter.toml. Created on first run; every field is optional.

use serde::Deserialize;
use std::path::{Path, PathBuf};

const FILE: &str = "SETTConverter.toml";

const TEMPLATE: &str = r##"# SETTConverter - configuracoes opcionais.
# Remova o "#" da linha que quiser mudar. Sem este arquivo, os padroes valem.

# Qualidade JPG (1-100)
# jpg_quality = 90

# Qualidade WEBP (1-100)
# webp_quality = 85

# Caminho proprio do FFmpeg (vazio = procura no PATH e baixa se faltar)
# ffmpeg = "C:\\tools\\ffmpeg.exe"

# Caminho proprio do 7-Zip (vazio = procura no PATH e baixa se faltar)
# seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"
"##;

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
        if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(dir.join(FILE), TEMPLATE).is_ok() {
            return;
        }
    }
}

pub fn load() -> Config {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}
