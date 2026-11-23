// Image and vector dispatch: JXL, AVIF, animated sources and the in-process encoders.

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

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
            image::to_pdf(&decoded, &out)?;
            return Ok(vec![out.into_path()]);
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
        image::save_image(&decoded, &out, to, quality)?;
        return Ok(vec![out.into_path()]);
    }

    // AVIF/HEIC/HEIF are decoded by FFmpeg; PDF would need a PNG step first.
    let ffmpeg_only = matches!(extension, "avif" | "heic" | "heif");
    if ffmpeg_only || to == "avif" {
        if to == "pdf" {
            bail!(
                "{}",
                t(
                    "HEIC/AVIF para PDF não é suportado; converta para PNG ou JPG antes",
                    "HEIC/AVIF to PDF is not supported; convert to PNG or JPG first"
                )
            );
        }
        if !(formats::is_image_target(to) || matches!(to, "apng" | "mp4" | "webm")) {
            bail!(
                "{} .{extension} -> .{to}",
                t("imagem não suportada:", "unsupported image conversion:")
            );
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
            return image::convert_svg(input, to, quality, out_dir, options.size);
        }
        bail!(
            "{} .{to}",
            t("vetor para não suportado:", "vector to unsupported:")
        );
    }

    if to == "pdf" || to == "svg" || formats::is_image_target(to) {
        image::convert(input, to, quality, out_dir, options.size)
    } else {
        bail!(
            "{} .{to}",
            t("imagem para não suportado:", "image to unsupported:")
        )
    }
}
