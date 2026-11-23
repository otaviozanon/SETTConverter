// Codec selection for FFmpeg, CPU and hardware encoders.

use anyhow::{Result, bail};
use std::process::Command;

use crate::convert::Options;
use crate::lang::t;

pub(super) fn avif_crf(quality: u8) -> u8 {
    // quality 1-100 -> CRF 55-10
    (60 - (quality.min(100) as u16 * 50 / 100) as u8).clamp(10, 63)
}

pub(super) fn uses_video_codec(to: &str) -> bool {
    matches!(to, "mp4" | "mkv" | "mov" | "webm")
}

pub(super) fn codecs(command: &mut Command, to: &str, options: &Options) -> Result<()> {
    if options.gpu {
        codecs_gpu(command, to)?;
    } else {
        codecs_cpu(command, to)?;
    }
    Ok(())
}

pub(super) fn codecs_cpu(command: &mut Command, to: &str) -> Result<()> {
    match to {
        "mp4" => command.args([
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "23",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
        ]),
        "webm" => command.args([
            "-c:v",
            "libvpx-vp9",
            "-crf",
            "32",
            "-b:v",
            "0",
            "-c:a",
            "libopus",
        ]),
        "mkv" | "mov" => command.args(["-c:v", "libx264", "-crf", "23", "-c:a", "aac"]),
        "avi" => command.args(["-c:v", "mpeg4", "-q:v", "5", "-c:a", "libmp3lame"]),
        "gif" => command.args([
            "-vf",
            "fps=15,scale=480:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse",
            "-loop",
            "0",
        ]),
        "apng" => command.args(["-plays", "0", "-f", "apng"]),
        "mp3" => command.args(["-vn", "-c:a", "libmp3lame", "-q:a", "2"]),
        "m4a" => command.args(["-vn", "-c:a", "aac", "-b:a", "192k"]),
        "wav" => command.args(["-vn", "-c:a", "pcm_s16le"]),
        "flac" => command.args(["-vn", "-c:a", "flac"]),
        "ogg" => command.args(["-vn", "-c:a", "libvorbis", "-q:a", "5"]),
        "opus" => command.args(["-vn", "-c:a", "libopus", "-b:a", "128k"]),
        "png" | "jpg" => command.args(["-frames:v", "1", "-q:v", "2"]),
        other => bail!(
            "{} {other}",
            t("destino de mídia desconhecido:", "unknown media target:")
        ),
    };
    Ok(())
}

pub(super) fn codecs_gpu(command: &mut Command, to: &str) -> Result<()> {
    match to {
        "mp4" => command.args([
            "-c:v",
            "h264_nvenc",
            "-preset",
            "p5",
            "-cq",
            "23",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-movflags",
            "+faststart",
        ]),
        "mkv" | "mov" => command.args(["-c:v", "h264_nvenc", "-cq", "23", "-c:a", "aac"]),
        other => return codecs_cpu(command, other),
    };
    Ok(())
}
