// Fonts: WOFF2 and TTF/OTF both ways (woofwoof wraps Google's woff2 library).

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

pub fn convert(input: &Path, to: &str, out_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let ext = crate::formats::extension_of(input);
    let data = std::fs::read(input).with_context(|| format!("abrir {}", input.display()))?;
    let output = match (ext.as_str(), to) {
        ("woff2", "ttf") => woofwoof::decompress(&data)
            .ok_or_else(|| anyhow::anyhow!("não foi possível decodificar este WOFF2"))?,
        ("ttf" | "otf", "woff2") => woofwoof::compress(&data, "", 8, true)
            .ok_or_else(|| anyhow::anyhow!("não foi possível codificar esta fonte para WOFF2"))?,
        _ => bail!("fonte .{ext} para .{to} não suportado"),
    };
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), to)?;
    std::fs::write(&out, output)?;
    Ok(vec![out])
}
