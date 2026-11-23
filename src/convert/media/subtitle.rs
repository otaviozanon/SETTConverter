// Subtitle conversion through FFmpeg.

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::lang::t;

use super::{base_command, ensure_success, label};

pub fn subtitle(
    input: &Path,
    to: &str,
    cfg: &Config,
    out_dir: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    let ffmpeg = crate::tools::ffmpeg(cfg)?;
    let codec = match to {
        "srt" => "srt",
        "vtt" => "webvtt",
        "ass" | "ssa" => "ass",
        other => bail!(
            "{} {other}",
            t(
                "destino de legenda desconhecido:",
                "unknown subtitle target:"
            )
        ),
    };
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), to)?;
    let mut command = base_command(&ffmpeg);
    command
        .args(["-i"])
        .arg(input)
        .args(["-c:s", codec])
        .arg(&out);
    let output = crate::progress::run(
        &label(t("Convertendo legenda", "Converting subtitle"), input),
        &mut command,
    )?;
    ensure_success(&output)?;
    Ok(vec![out.into_path()?])
}
