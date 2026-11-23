// JPEG XL decode through jxl-oxide (the FFmpeg essentials build has no JXL).

use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};
use std::path::Path;

use crate::lang::t;

pub fn decode(input: &Path) -> Result<DynamicImage> {
    use jxl_oxide::{EnumColourEncoding, JxlImage, RenderingIntent};

    let file = std::fs::File::open(input)
        .with_context(|| format!("{} {}", t("abrir", "open"), input.display()))?;
    let mut image = JxlImage::builder()
        .read(std::io::BufReader::new(file))
        .map_err(|error| {
            anyhow::anyhow!("{} {}: {error}", t("ler JXL", "read JXL"), input.display())
        })?;
    image.request_color_encoding(EnumColourEncoding::srgb(RenderingIntent::Perceptual));
    let render = image.render_frame(0).map_err(|error| {
        anyhow::anyhow!(
            "{} {}: {error}",
            t("decodificar JXL", "decode JXL"),
            input.display()
        )
    })?;

    let frame = render.image_all_channels();
    let (width, height, channels) = (frame.width(), frame.height(), frame.channels());
    let samples = frame.buf();
    let mut rgba = Vec::with_capacity(width * height * 4);
    for pixel in samples.chunks_exact(channels) {
        let channel = |index: usize| -> u8 {
            (pixel.get(index).copied().unwrap_or(1.0).clamp(0.0, 1.0) * 255.0).round() as u8
        };
        let alpha = if channels >= 4 { channel(3) } else { 255 };
        rgba.extend_from_slice(&[channel(0), channel(1), channel(2), alpha]);
    }
    let image =
        RgbaImage::from_raw(width as u32, height as u32, rgba).context("buffer do JXL em RGBA")?;
    Ok(DynamicImage::ImageRgba8(image))
}
