// Video, audio and animated images through FFmpeg.
pub mod codecs;
pub mod subtitle;

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::config::Config;
use crate::convert::Options;
use crate::lang::t;

use codecs::{avif_crf, codecs, codecs_cpu, uses_video_codec};
pub use subtitle::subtitle;

pub fn convert(
    input: &Path,
    to: &str,
    cfg: &Config,
    out_dir: Option<&Path>,
    options: &Options,
) -> Result<Vec<PathBuf>> {
    let ffmpeg = crate::tools::ffmpeg(cfg)?;
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let stem = crate::naming::stem_of(input);

    if options.sheet {
        let out = crate::naming::claim(&dir, &format!("{stem}_contatos"), to)?;
        let mut command = base_command(&ffmpeg);
        command
            .args(["-i"])
            .arg(input)
            .args(["-vf", "fps=1/2,scale=320:-1,tile=4x4", "-frames:v", "1"])
            .arg(&out);
        let output = crate::progress::run(
            &label(t("Folha de contatos", "Contact sheet"), input),
            &mut command,
        )?;
        ensure_success(&output)?;
        return Ok(vec![out.into_path()]);
    }

    let (out, pattern) = if options.frames {
        let folder = crate::naming::claim_dir(&dir, &format!("{stem}_frames"))?;
        let pattern = folder.join(format!("frame_%06d.{to}"));
        (folder, Some(pattern))
    } else {
        (crate::naming::claim(&dir, &stem, to)?, None)
    };

    let mut command = base_command(&ffmpeg);
    if let Some(start) = &options.start {
        command.arg("-ss").arg(start);
    }
    // Fast seek for a single frame, before decoding the whole file.
    if let Some(at) = &options.at {
        command.arg("-ss").arg(at);
    }
    command.arg("-i").arg(input);

    if options.strip {
        command.args(["-map_metadata", "-1"]);
    }
    if let Some(duration) = duration(options) {
        command.args(["-t", &duration]);
    }

    if let Some(pattern) = &pattern {
        command.args(["-fps_mode", "passthrough"]).arg(pattern);
    } else if options.at.is_some() {
        command.args(["-frames:v", "1", "-q:v", "2"]).arg(&out);
    } else {
        codecs(&mut command, to, options)?;
        command.arg(&out);
    }

    let output = crate::progress::run(&label(t("Convertendo", "Converting"), input), &mut command)?;
    if !output.status.success() && options.gpu && uses_video_codec(to) {
        // Hardware encoder missing or busy: same job on the CPU.
        let mut fallback = base_command(&ffmpeg);
        if let Some(start) = &options.start {
            fallback.arg("-ss").arg(start);
        }
        fallback.arg("-i").arg(input);
        if options.strip {
            fallback.args(["-map_metadata", "-1"]);
        }
        if let Some(duration) = duration(options) {
            fallback.args(["-t", &duration]);
        }
        codecs_cpu(&mut fallback, to)?;
        fallback.arg(&out);
        let retry =
            crate::progress::run(&label(t("Convertendo", "Converting"), input), &mut fallback)?;
        ensure_success(&retry)?;
    } else {
        ensure_success(&output)?;
    }

    if options.frames {
        let mut written: Vec<PathBuf> = std::fs::read_dir(&out)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect();
        written.sort();
        if written.is_empty() {
            bail!("FFmpeg não escreveu nenhum frame");
        }
        let _ = out.into_path();
        Ok(written)
    } else {
        Ok(vec![out.into_path()])
    }
}

pub fn audio_cover(input: &Path, cfg: &Config, out_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let ffmpeg = crate::tools::ffmpeg(cfg)?;
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(
        &dir,
        &format!("{}_capa", crate::naming::stem_of(input)),
        "jpg",
    )?;
    let mut command = base_command(&ffmpeg);
    command
        .args(["-i"])
        .arg(input)
        .args(["-map", "0:v:0", "-frames:v", "1", "-q:v", "2"])
        .arg(&out);
    let output = crate::progress::run(
        &label(t("Extraindo capa", "Extracting cover"), input),
        &mut command,
    )?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&out);
        bail!(
            "{}",
            t(
                "o arquivo não tem capa embutida",
                "the file has no embedded cover"
            )
        );
    }
    Ok(vec![out.into_path()])
}

pub fn animated_image(
    input: &Path,
    to: &str,
    cfg: &Config,
    out_dir: Option<&Path>,
    options: &Options,
) -> Result<Vec<PathBuf>> {
    let ffmpeg = crate::tools::ffmpeg(cfg)?;
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), to)?;
    let mut command = base_command(&ffmpeg);
    command.args(["-i"]).arg(input);
    match to {
        "gif" => command.args([
            "-vf",
            "split[a][b];[a]palettegen[p];[b][p]paletteuse",
            "-loop",
            "0",
        ]),
        "webp" => command.args(["-c:v", "libwebp_anim", "-loop", "0", "-q:v", "80"]),
        "apng" => command.args(["-plays", "0", "-f", "apng"]),
        "mp4" => command.args([
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            "23",
            "-movflags",
            "+faststart",
        ]),
        "webm" => command.args(["-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0"]),
        other => bail!(
            "{} .{other}",
            t("animação para não suportada:", "animation to unsupported:")
        ),
    };
    let _ = options;
    command.arg(&out);
    let output = crate::progress::run(
        &label(t("Convertendo animação", "Converting animation"), input),
        &mut command,
    )?;
    ensure_success(&output)?;
    Ok(vec![out.into_path()])
}

pub fn still_image(
    input: &Path,
    to: &str,
    cfg: &Config,
    out_dir: Option<&Path>,
    quality: u8,
) -> Result<Vec<PathBuf>> {
    let ffmpeg = crate::tools::ffmpeg(cfg)?;
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), to)?;
    let mut command = base_command(&ffmpeg);
    command.args(["-i"]).arg(input);
    if to == "avif" {
        command.args([
            "-frames:v",
            "1",
            "-c:v",
            "libaom-av1",
            "-still-picture",
            "1",
            "-crf",
            &avif_crf(quality).to_string(),
        ]);
    } else {
        command.args(["-frames:v", "1"]);
    }
    command.arg(&out);
    let output = crate::progress::run(
        &label(t("Convertendo imagem", "Converting image"), input),
        &mut command,
    )?;
    ensure_success(&output)?;
    Ok(vec![out.into_path()])
}

fn base_command(ffmpeg: &Path) -> Command {
    let mut command = Command::new(ffmpeg);
    command.args(["-nostdin", "-hide_banner", "-loglevel", "error", "-y"]);
    command
}

fn ensure_success(output: &Output) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        bail!(
            "{} {}",
            t("FFmpeg falhou:", "FFmpeg failed:"),
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
}

fn label(action: &str, input: &Path) -> String {
    format!(
        "{action} {}...",
        input
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    )
}

fn duration(options: &Options) -> Option<String> {
    match (&options.start, &options.end) {
        (Some(start), Some(end)) => match (seconds(start), seconds(end)) {
            (Some(start), Some(end)) if end > start => Some(format!("{:.3}", end - start)),
            _ => None,
        },
        (None, Some(end)) => Some(end.clone()),
        _ => None,
    }
}

fn seconds(text: &str) -> Option<f64> {
    let parts: Vec<&str> = text.split(':').collect();
    match parts.len() {
        1 => parts[0].parse().ok(),
        2 => Some(parts[0].parse::<f64>().ok()? * 60.0 + parts[1].parse::<f64>().ok()?),
        3 => Some(
            parts[0].parse::<f64>().ok()? * 3600.0
                + parts[1].parse::<f64>().ok()? * 60.0
                + parts[2].parse::<f64>().ok()?,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
