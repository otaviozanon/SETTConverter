// Image and vector dispatch: JXL, AVIF, animated sources and the in-process encoders.

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::config::Config;
use crate::formats::{self, Family};
use crate::lang::t;

use super::{Options, image, jxl, media};

pub(super) fn convert(
    options: &Options,
    input: &Path,
    extension: &str,
    quality: u8,
    out_dir: Option<&Path>,
    config: &Config,
) -> Result<Vec<PathBuf>> {
    let to = options.to.as_str();
    let save = image::SaveOptions {
        quality,
        png: config.png_compression(),
        gif: config.gif_quality(),
        ico: config.ico_size(),
        pdf_jpeg: config.pdf_jpeg().then_some(quality),
        pdf_dpi: config.image_pdf_dpi(),
    };

    // JPEG XL has no FFmpeg decoder in the essentials build.
    if extension == "jxl" {
        let mut decoded = jxl::decode(input)?;
        image::apply_size(&mut decoded, options.size);
        if to == "pdf" {
            let out = crate::naming::claim(
                &out_dir
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| crate::naming::parent_of(input)),
                &crate::naming::stem_of(input),
                "pdf",
            )?;
            image::to_pdf(&decoded, &out, save.pdf_jpeg, save.pdf_dpi)?;
            return Ok(vec![out.into_path()?]);
        }
        if !formats::is_image_target(to) {
            bail!(
                "{} .{to}",
                t("JXL para não suportado:", "JXL to unsupported:")
            );
        }
        let out = crate::naming::claim(
            &out_dir
                .map(Path::to_path_buf)
                .unwrap_or_else(|| crate::naming::parent_of(input)),
            &crate::naming::stem_of(input),
            to,
        )?;
        image::save_image(&decoded, &out, to, save)?;
        return Ok(vec![out.into_path()?]);
    }

    // AVIF/HEIC/HEIF are decoded by FFmpeg; PDF needs a PNG step first.
    let ffmpeg_only = matches!(extension, "avif" | "heic" | "heif");
    if ffmpeg_only || to == "avif" {
        if !(formats::is_image_target(to) || matches!(to, "apng" | "mp4" | "webm" | "pdf")) {
            bail!(
                "{} .{extension} -> .{to}",
                t("imagem não suportada:", "unsupported image conversion:")
            );
        }
        if to == "pdf" {
            return ffmpeg_to_pdf(input, config, out_dir, quality);
        }
        return media::still_image(input, to, config, out_dir, quality);
    }

    // Animated sources keep every frame only through FFmpeg; APNG/MP4 need it too.
    let animated = matches!(extension, "gif" | "webp");
    let animated_target = matches!(to, "gif" | "webp" | "apng" | "mp4" | "webm");
    if animated_target && (animated || matches!(to, "apng" | "mp4" | "webm")) {
        return media::animated_image(input, to, config, out_dir, options);
    }

    if formats::family(extension) == Some(Family::Vector) {
        if to == "pdf" || formats::is_image_target(to) {
            return image::convert_svg(input, to, save, out_dir, options.size);
        }
        bail!(
            "{} .{to}",
            t("vetor para não suportado:", "vector to unsupported:")
        );
    }

    if to == "pdf" || to == "svg" || formats::is_image_target(to) {
        image::convert(input, to, save, out_dir, options.size)
    } else {
        bail!(
            "{} .{to}",
            t("imagem para não suportado:", "image to unsupported:")
        )
    }
}

static DECODE_TEMP: AtomicU32 = AtomicU32::new(0);

/// HEIC/AVIF to PDF: decode to a temporary PNG, then use the in-process writer.
fn ffmpeg_to_pdf(
    input: &Path,
    config: &Config,
    out_dir: Option<&Path>,
    quality: u8,
) -> Result<Vec<PathBuf>> {
    let temp = std::env::temp_dir().join(format!(
        "settconv-decode-{}-{}",
        std::process::id(),
        DECODE_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&temp)?;
    let result = (|| -> Result<PathBuf> {
        let decoded = media::still_image(input, "png", config, Some(&temp), quality)?;
        let image = image::decode(&decoded[0])?;
        let dir = out_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| crate::naming::parent_of(input));
        let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), "pdf")?;
        image::to_pdf(
            &image,
            &out,
            config.pdf_jpeg().then_some(quality),
            config.image_pdf_dpi(),
        )?;
        out.into_path()
    })();
    let _ = std::fs::remove_dir_all(&temp);
    Ok(vec![result?])
}
