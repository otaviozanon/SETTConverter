// Raster images, SVG and image to PDF.

use anyhow::{Context, Result, bail};
use image::{DynamicImage, ImageDecoder, ImageFormat, RgbaImage};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::lang::t;

pub fn convert(
    input: &Path,
    to: &str,
    quality: u8,
    out_dir: Option<&Path>,
    size: Option<(u32, u32)>,
) -> Result<Vec<PathBuf>> {
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let stem = crate::naming::stem_of(input);
    let out = crate::naming::claim(&dir, &stem, to)?;

    if to == "svg" {
        trace(input, &out)?;
        return Ok(vec![out.into_path()]);
    }
    let mut img = decode(input)?;
    apply_size(&mut img, size);
    if to == "pdf" {
        to_pdf(&img, &out)?;
        return Ok(vec![out.into_path()]);
    }
    save_image(&img, &out, to, quality)?;
    Ok(vec![out.into_path()])
}

/// Scales to the requested box; a missing side keeps the aspect ratio.
pub fn apply_size(image: &mut DynamicImage, size: Option<(u32, u32)>) {
    let Some((width, height)) = size else {
        return;
    };
    let (source_width, source_height) = (image.width().max(1), image.height().max(1));
    let (target_width, target_height) = match (width, height) {
        (0, 0) => return,
        (width, 0) => (
            width.max(1),
            ((source_height as u64 * width as u64) / source_width as u64).max(1) as u32,
        ),
        (0, height) => (
            ((source_width as u64 * height as u64) / source_height as u64).max(1) as u32,
            height.max(1),
        ),
        (width, height) => (width.max(1), height.max(1)),
    };
    *image = image.resize_exact(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );
}

/// Reads the pixels and applies the EXIF orientation, so phone photos are not stored sideways.
pub fn decode(input: &Path) -> Result<DynamicImage> {
    let file =
        File::open(input).with_context(|| format!("{} {}", t("abrir", "open"), input.display()))?;
    let mut reader = image::ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .with_context(|| format!("{} {}", t("reconhecer", "recognize"), input.display()))?;
    // A clear limit beats a generic allocator failure on absurd images.
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(50_000);
    limits.max_image_height = Some(50_000);
    limits.max_alloc = Some(1_500_000_000);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().with_context(|| {
        format!(
            "{} {} ({})",
            t("decodificar", "decode"),
            input.display(),
            t("imagem grande ou inválida", "image too large or invalid")
        )
    })?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).with_context(|| {
        format!(
            "{} {} ({})",
            t("decodificar", "decode"),
            input.display(),
            t("imagem grande ou inválida", "image too large or invalid")
        )
    })?;
    image.apply_orientation(orientation);
    Ok(image)
}

pub fn convert_svg(
    input: &Path,
    to: &str,
    quality: u8,
    out_dir: Option<&Path>,
    size: Option<(u32, u32)>,
) -> Result<Vec<PathBuf>> {
    let data = std::fs::read(input)
        .with_context(|| format!("{} {}", t("abrir", "open"), input.display()))?;
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_data(&data, &options)
        .with_context(|| format!("ler SVG {}", input.display()))?;

    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let stem = crate::naming::stem_of(input);
    let out = crate::naming::claim(&dir, &stem, to)?;

    if to == "pdf" {
        let pdf = svg2pdf::to_pdf(
            &tree,
            svg2pdf::ConversionOptions::default(),
            svg2pdf::PageOptions::default(),
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "{} {e:?}",
                t("converter SVG para PDF:", "convert SVG to PDF:")
            )
        })?;
        std::fs::write(&out, pdf)?;
        return Ok(vec![out.into_path()]);
    }

    let svg_size = tree.size().to_int_size();
    let (w, h) = (svg_size.width().max(1), svg_size.height().max(1));
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).context("alocar bitmap do SVG")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let rgba = RgbaImage::from_raw(w, h, pixmap.data().to_vec()).context("bitmap do SVG")?;
    let mut image = DynamicImage::ImageRgba8(rgba);
    apply_size(&mut image, size);
    save_image(&image, &out, to, quality)?;
    Ok(vec![out.into_path()])
}

pub fn save_image(img: &DynamicImage, out: &Path, to: &str, quality: u8) -> Result<()> {
    match to {
        "jpg" | "jpeg" => {
            let rgb = img.to_rgb8();
            let mut file = BufWriter::new(File::create(out)?);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality)
                .encode_image(&rgb)?;
        }
        "png" => img.save_with_format(out, ImageFormat::Png)?,
        "webp" => img.save_with_format(out, ImageFormat::WebP)?,
        // Animated sources and targets go through FFmpeg; here one frame is enough.
        "gif" => img.save_with_format(out, ImageFormat::Gif)?,
        "bmp" => img.save_with_format(out, ImageFormat::Bmp)?,
        "tiff" | "tif" => img.save_with_format(out, ImageFormat::Tiff)?,
        "ico" => {
            let icon = img.resize(256, 256, image::imageops::FilterType::Lanczos3);
            icon.to_rgba8().save_with_format(out, ImageFormat::Ico)?;
        }
        "tga" => img.save_with_format(out, ImageFormat::Tga)?,
        "pnm" => img.save_with_format(out, ImageFormat::Pnm)?,
        "dds" => img.save_with_format(out, ImageFormat::Dds)?,
        "hdr" => img.save_with_format(out, ImageFormat::Hdr)?,
        "exr" => img.save_with_format(out, ImageFormat::OpenExr)?,
        "qoi" => img.save_with_format(out, ImageFormat::Qoi)?,
        other => bail!(
            "{} {other}",
            t("destino de imagem desconhecido:", "unknown image target:")
        ),
    }
    Ok(())
}

pub fn to_pdf(img: &DynamicImage, out: &Path) -> Result<()> {
    use lopdf::{Dictionary, Document, Object, Stream};

    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    // 96 DPI keeps the page the physical size the picture has on screen.
    let w_pt = w as f32 * 72.0 / 96.0;
    let h_pt = h as f32 * 72.0 / 96.0;
    let mut doc = Document::with_version("1.5");

    let mut image_dict = Dictionary::new();
    image_dict.set("Type", "XObject");
    image_dict.set("Subtype", "Image");
    image_dict.set("Width", w as i64);
    image_dict.set("Height", h as i64);
    image_dict.set("ColorSpace", "DeviceRGB");
    image_dict.set("BitsPerComponent", 8i64);
    let image_id = doc.add_object(Stream::new(image_dict, rgb.into_raw()));

    let content = format!("q {w_pt} 0 0 {h_pt} 0 0 cm /Im0 Do Q");
    let content_id = doc.add_object(Stream::new(Dictionary::new(), content.into_bytes()));

    let pages_id = doc.new_object_id();
    let mut xobjects = Dictionary::new();
    xobjects.set("Im0", image_id);
    let mut resources = Dictionary::new();
    resources.set("XObject", xobjects);

    let mut page = Dictionary::new();
    page.set("Type", "Page");
    page.set(
        "MediaBox",
        vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(w_pt),
            Object::Real(h_pt),
        ],
    );
    page.set("Resources", resources);
    page.set("Contents", content_id);
    page.set("Parent", pages_id);
    let page_id = doc.add_object(page);

    let mut pages = Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Count", 1i64);
    pages.set("Kids", vec![Object::Reference(page_id)]);
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let mut catalog = Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", pages_id);
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    doc.save(out)?;
    Ok(())
}

fn trace(input: &Path, out: &Path) -> Result<()> {
    vtracer::convert_image_to_svg(input, out, vtracer::Config::default())
        .map_err(|e| anyhow::anyhow!("{} {e}", t("vetorizar:", "trace:")))?;
    Ok(())
}
