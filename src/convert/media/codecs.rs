// Codec selection for FFmpeg, CPU and hardware encoders.

use anyhow::{Result, bail};
use std::process::Command;

use crate::config::Config;
use crate::convert::Options;
use crate::lang::t;

pub(super) fn avif_crf(quality: u8) -> u8 {
    // quality 1-100 -> CRF 55-10
    (60 - (quality.min(100) as u16 * 50 / 100) as u8).clamp(10, 63)
}

/// 1-100 -> CRF; 75 keeps the previous H.264 default of 23. `max` is codec-specific.
pub(super) fn video_crf(quality: Option<u8>, default: u8, max: u8) -> String {
    let value = match quality {
        Some(quality) => (60.0 - quality.min(100) as f32 * 0.5).round() as u8,
        None => default,
    };
    value.clamp(10, max).to_string()
}

/// Same 1-100 scale as `jpg_quality`, inverted for FFmpeg's 2-31 qscale.
pub(super) fn jpeg_scale(quality: u8) -> String {
    (31 - (quality.min(100) as u16 * 29 / 100))
        .clamp(2, 31)
        .to_string()
}

/// Quality flags for a single output frame or frame sequence.
pub(super) fn frame_args(command: &mut Command, to: &str, options: &Options, cfg: &Config) {
    match to {
        "jpg" => {
            let quality = options.quality.unwrap_or_else(|| cfg.jpg_quality());
            command.args(["-q:v", &jpeg_scale(quality)]);
        }
        "png" => {
            command.args(["-compression_level", &cfg.png_compression().to_string()]);
        }
        _ => {}
    }
}

pub(super) fn uses_video_codec(to: &str) -> bool {
    matches!(to, "mp4" | "mkv" | "mov" | "webm")
}

pub(super) fn codecs(
    command: &mut Command,
    to: &str,
    options: &Options,
    cfg: &Config,
) -> Result<()> {
    if options.gpu {
        codecs_gpu(command, to, options, cfg)?;
    } else {
        codecs_cpu(command, to, options, cfg)?;
    }
    Ok(())
}

pub(super) fn codecs_cpu(
    command: &mut Command,
    to: &str,
    options: &Options,
    cfg: &Config,
) -> Result<()> {
    let quality = options.quality.or(cfg.video_quality());
    let preset = cfg.video_preset();
    match to {
        "mp4" => {
            let crf = video_crf(quality, 23, 51);
            let audio = cfg.audio_bitrate().unwrap_or("192k");
            command.args([
                "-c:v",
                "libx264",
                "-preset",
                preset,
                "-crf",
                &crf,
                "-c:a",
                "aac",
                "-b:a",
                audio,
                "-movflags",
                "+faststart",
            ]);
        }
        "webm" => {
            let crf = video_crf(quality, 32, 63);
            command.args([
                "-c:v",
                "libvpx-vp9",
                "-crf",
                &crf,
                "-b:v",
                "0",
                "-c:a",
                "libopus",
            ]);
            if let Some(audio) = cfg.audio_bitrate() {
                command.args(["-b:a", audio]);
            }
        }
        "mkv" | "mov" => {
            let crf = video_crf(quality, 23, 51);
            command.args([
                "-c:v", "libx264", "-preset", preset, "-crf", &crf, "-c:a", "aac",
            ]);
            if let Some(audio) = cfg.audio_bitrate() {
                command.args(["-b:a", audio]);
            }
        }
        "avi" => {
            let scale = quality.map(jpeg_scale).unwrap_or_else(|| "5".to_string());
            command.args(["-c:v", "mpeg4", "-q:v", &scale, "-c:a", "libmp3lame"]);
        }
        "gif" => {
            let filter = format!(
                "fps={},scale={}:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse",
                cfg.gif_fps(),
                cfg.gif_width()
            );
            command.args(["-vf", &filter, "-loop", "0"]);
        }
        "apng" => {
            command.args(["-plays", "0", "-f", "apng"]);
        }
        "mp3" => {
            command.args(["-vn", "-c:a", "libmp3lame"]);
            match cfg.audio_bitrate() {
                Some(audio) => command.args(["-b:a", audio]),
                None => command.args(["-q:a", "2"]),
            };
        }
        "m4a" => {
            let audio = cfg.audio_bitrate().unwrap_or("192k");
            command.args(["-vn", "-c:a", "aac", "-b:a", audio]);
        }
        "wav" => {
            command.args(["-vn", "-c:a", "pcm_s16le"]);
        }
        "flac" => {
            command.args(["-vn", "-c:a", "flac"]);
        }
        "ogg" => {
            command.args(["-vn", "-c:a", "libvorbis"]);
            match cfg.audio_bitrate() {
                Some(audio) => command.args(["-b:a", audio]),
                None => command.args(["-q:a", "5"]),
            };
        }
        "opus" => {
            let audio = cfg.audio_bitrate().unwrap_or("128k");
            command.args(["-vn", "-c:a", "libopus", "-b:a", audio]);
        }
        "png" | "jpg" => {
            command.args(["-frames:v", "1"]);
            frame_args(command, to, options, cfg);
        }
        other => bail!(
            "{} {other}",
            t("destino de mídia desconhecido:", "unknown media target:")
        ),
    };
    Ok(())
}

pub(super) fn codecs_gpu(
    command: &mut Command,
    to: &str,
    options: &Options,
    cfg: &Config,
) -> Result<()> {
    let quality = options.quality.or(cfg.video_quality());
    match to {
        "mp4" => {
            let crf = video_crf(quality, 23, 51);
            let audio = cfg.audio_bitrate().unwrap_or("192k");
            command.args([
                "-c:v",
                "h264_nvenc",
                "-preset",
                "p5",
                "-cq",
                &crf,
                "-c:a",
                "aac",
                "-b:a",
                audio,
                "-movflags",
                "+faststart",
            ]);
        }
        "mkv" | "mov" => {
            let crf = video_crf(quality, 23, 51);
            command.args([
                "-c:v",
                "h264_nvenc",
                "-preset",
                "p5",
                "-cq",
                &crf,
                "-c:a",
                "aac",
            ]);
            if let Some(audio) = cfg.audio_bitrate() {
                command.args(["-b:a", audio]);
            }
        }
        other => return codecs_cpu(command, other, options, cfg),
    };
    Ok(())
}
