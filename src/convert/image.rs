// Raster images, SVG and image to PDF.

use anyhow::{Context, Result, bail};
use image::{DynamicImage, ImageDecoder, ImageFormat, RgbaImage};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

pub fn convert(
    input: &Path,
    to: &str,
    quality: u8,
    out_dir: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let stem = crate::naming::stem_of(input);
    let out = crate::naming::claim(&dir, &stem, to)?;

    if to == "svg" {
        trace(input, &out)?;
        return Ok(vec![out]);
    }
    let img = decode(input)?;
    if to == "pdf" {
        to_pdf(&img, &out)?;
        return Ok(vec![out]);
    }
    save(&img, &out, to, quality)?;
    Ok(vec![out])
}

/// Reads the pixels and applies the EXIF orientation, so phone photos are not stored sideways.
fn decode(input: &Path) -> Result<DynamicImage> {
    let file = File::open(input).with_context(|| format!("abrir {}", input.display()))?;
    let mut decoder = image::ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .with_context(|| format!("reconhecer {}", input.display()))?
        .into_decoder()
        .with_context(|| format!("decodificar {}", input.display()))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder)
        .with_context(|| format!("decodificar {}", input.display()))?;
    image.apply_orientation(orientation);
    Ok(image)
}

pub fn convert_svg(
    input: &Path,
    to: &str,
    quality: u8,
    out_dir: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    let data = std::fs::read(input).with_context(|| format!("abrir {}", input.display()))?;
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
        .map_err(|e| anyhow::anyhow!("converter SVG para PDF: {e:?}"))?;
        std::fs::write(&out, pdf)?;
        return Ok(vec![out]);
    }

    let size = tree.size().to_int_size();
    let (w, h) = (size.width().max(1), size.height().max(1));
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).context("alocar bitmap do SVG")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let rgba = RgbaImage::from_raw(w, h, pixmap.data().to_vec()).context("bitmap do SVG")?;
    save(&DynamicImage::ImageRgba8(rgba), &out, to, quality)?;
    Ok(vec![out])
}

fn save(img: &DynamicImage, out: &Path, to: &str, quality: u8) -> Result<()> {
    match to {
        "jpg" | "jpeg" => {
            let rgb = img.to_rgb8();
            let mut file = BufWriter::new(File::create(out)?);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality)
                .encode_image(&rgb)?;
        }
        "png" => img.save_with_format(out, ImageFormat::Png)?,
        "webp" => img.save_with_format(out, ImageFormat::WebP)?,
        // shortcut: GIF recebe so o primeiro frame, animacao fica para depois.
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
        other => bail!("destino de imagem desconhecido: {other}"),
    }
    Ok(())
}

fn to_pdf(img: &DynamicImage, out: &Path) -> Result<()> {
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
        .map_err(|e| anyhow::anyhow!("vetorizar: {e}"))?;
    Ok(())
}
