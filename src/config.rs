// Settings\SETTConverter.toml. Created on first run; every field is optional.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

const FILE: &str = "SETTConverter.toml";

const TEMPLATE_PT: &str = r##"# SETTConverter - configuracoes opcionais.
# Remova o "#" da linha que quiser mudar. Sem este arquivo, os padroes valem.

# Qualidade JPG (1-100)
# jpg_quality = 90

# Qualidade WEBP (1-100)
# webp_quality = 85

# Pasta de saida padrao (vazio = ao lado do arquivo)
# out_dir = "C:\\Convertidos"

# Sobrescrever arquivos existentes em vez de criar "nome (2)"
# overwrite = false

# Caminho proprio do FFmpeg (vazio = procura no PATH e baixa se faltar)
# ffmpeg = "C:\\tools\\ffmpeg.exe"

# Caminho proprio do 7-Zip (vazio = procura no PATH e baixa se faltar)
# seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"
"##;

const TEMPLATE_EN: &str = r##"# SETTConverter - optional settings.
# Remove the "#" from the line you want to change. Defaults apply without this file.

# JPG quality (1-100)
# jpg_quality = 90

# WEBP quality (1-100)
# webp_quality = 85

# Default output folder (empty = next to the source file)
# out_dir = "C:\\Converted"

# Overwrite existing files instead of creating "name (2)"
# overwrite = false

# Custom FFmpeg path (empty = look in PATH and download when missing)
# ffmpeg = "C:\\tools\\ffmpeg.exe"

# Custom 7-Zip path (empty = look in PATH and download when missing)
# seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"
"##;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub jpg_quality: Option<u8>,
    pub webp_quality: Option<u8>,
    pub ffmpeg: Option<String>,
    pub seven_zip: Option<String>,
    pub out_dir: Option<String>,
    pub overwrite: Option<bool>,
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
}
