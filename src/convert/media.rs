// Audio and video through FFmpeg (from PATH, settconv.toml or the downloaded tools).

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;

pub fn convert(
    input: &Path,
    to: &str,
    cfg: &Config,
    out_dir: Option<&Path>,
    frames: bool,
) -> Result<Vec<PathBuf>> {
    let ffmpeg = crate::tools::ffmpeg(cfg)?;
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let stem = crate::naming::stem_of(input);

    let (out, pattern) = if frames {
        let folder = crate::naming::unique_dir(&dir, &format!("{stem}_frames"));
        std::fs::create_dir_all(&folder)?;
        let pattern = folder.join(format!("frame_%06d.{to}"));
        (folder.clone(), Some(pattern))
    } else {
        (crate::naming::unique(&dir, &stem, to), None)
    };

    let mut cmd = Command::new(&ffmpeg);
    cmd.arg("-nostdin")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(input);

    if let Some(pattern) = &pattern {
        cmd.args(["-fps_mode", "passthrough"]).arg(pattern);
    } else {
        match to {
            "mp4" => cmd.args([
                "-c:v", "libx264", "-preset", "medium", "-crf", "23", "-c:a", "aac", "-b:a",
                "192k", "-movflags", "+faststart",
            ]),
            "webm" => cmd.args([
                "-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0", "-c:a", "libopus",
            ]),
            "mkv" | "mov" => cmd.args(["-c:v", "libx264", "-crf", "23", "-c:a", "aac"]),
            "avi" => cmd.args(["-c:v", "mpeg4", "-q:v", "5", "-c:a", "libmp3lame"]),
            "gif" => cmd.args([
                "-vf",
                "fps=15,scale=480:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse",
                "-loop",
                "0",
            ]),
            "mp3" => cmd.args(["-vn", "-c:a", "libmp3lame", "-q:a", "2"]),
            "m4a" => cmd.args(["-vn", "-c:a", "aac", "-b:a", "192k"]),
            "wav" => cmd.args(["-vn", "-c:a", "pcm_s16le"]),
            "flac" => cmd.args(["-vn", "-c:a", "flac"]),
            "ogg" => cmd.args(["-vn", "-c:a", "libvorbis", "-q:a", "5"]),
            "opus" => cmd.args(["-vn", "-c:a", "libopus", "-b:a", "128k"]),
            "png" | "jpg" => cmd.args(["-frames:v", "1", "-q:v", "2"]),
            other => bail!("destino de mídia desconhecido: {other}"),
        };
        cmd.arg(&out);
    }

    let result = cmd
        .output()
        .with_context(|| format!("executar {}", ffmpeg.display()))?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!("FFmpeg falhou: {}", stderr.trim());
    }

    if frames {
        let mut written: Vec<PathBuf> = std::fs::read_dir(&out)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        written.sort();
        if written.is_empty() {
            bail!("FFmpeg não escreveu nenhum frame");
        }
        Ok(written)
    } else {
        Ok(vec![out])
    }
}

