// PDF page rendering through the embedded PDFium.

use anyhow::{Context, Result, bail};
use image::{DynamicImage, ImageEncoder, RgbaImage};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::lang::t;

/// Settings shared by page and long-image rendering.
#[derive(Clone, Copy)]
pub struct RenderOptions {
    pub width: Option<u32>,
    pub dpi: Option<u32>,
    pub quality: u8,
    pub png: u8,
}

pub fn render_pages(
    input: &Path,
    to: &str,
    out_dir: Option<&Path>,
    options: RenderOptions,
) -> Result<Vec<PathBuf>> {
    let input_owned = input.to_path_buf();
    let out_owned = out_dir.map(Path::to_path_buf);
    let to_owned = to.to_string();
    let label = format!(
        "{} {}...",
        t("Renderizando", "Rendering"),
        input
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    crate::progress::run_cancellable(&label, move |cancel| {
        render_pages_inner(
            &input_owned,
            &to_owned,
            out_owned.as_deref(),
            options,
            cancel,
        )
    })
}

fn render_pages_inner(
    input: &Path,
    to: &str,
    out_dir: Option<&Path>,
    options: RenderOptions,
    cancel: &AtomicBool,
) -> Result<Vec<PathBuf>> {
    let RenderOptions {
        width,
        dpi,
        quality,
        png,
    } = options;
    with_pdfium(|pdfium| {
        let document = pdfium
            .load_pdf_from_file(input, None)
            .with_context(|| format!("abrir PDF {}", input.display()))?;
        let dir = out_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| crate::naming::parent_of(input));
        let folder =
            crate::naming::claim_dir(&dir, &format!("{}_paginas", crate::naming::stem_of(input)))?;

        let render = render_config(width, dpi);

        let mut written = Vec::new();
        for (index, page) in document.pages().iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                bail!("{}", t("conversão cancelada", "conversion cancelled"));
            }
            let bitmap = page
                .render_with_config(&render)
                .with_context(|| format!("renderizar página {}", index + 1))?;
            let (w, h) = (bitmap.width() as u32, bitmap.height() as u32);
            let rgba = RgbaImage::from_raw(w, h, bitmap.as_rgba_bytes())
                .context("bitmap da página em RGBA")?;
            let out = folder.join(format!(
                "{}_{}.{}",
                crate::naming::stem_of(input),
                index + 1,
                to
            ));
            if to == "jpg" {
                let mut file = std::io::BufWriter::new(std::fs::File::create(&out)?);
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality)
                    .encode_image(&rgba)?;
            } else {
                let mut file = std::io::BufWriter::new(std::fs::File::create(&out)?);
                image::codecs::png::PngEncoder::new_with_quality(
                    &mut file,
                    image::codecs::png::CompressionType::Level(png.min(9)),
                    image::codecs::png::FilterType::Adaptive,
                )
                .write_image(
                    rgba.as_raw(),
                    w,
                    h,
                    image::ExtendedColorType::Rgba8,
                )?;
            }
            written.push(out);
        }
        let _ = folder.into_path()?;
        Ok(written)
    })
}

pub fn render_long(
    input: &Path,
    to: &str,
    out_dir: Option<&Path>,
    options: RenderOptions,
) -> Result<Vec<PathBuf>> {
    let input_owned = input.to_path_buf();
    let out_owned = out_dir.map(Path::to_path_buf);
    let to_owned = to.to_string();
    let label = format!(
        "{} {}...",
        t("Renderizando", "Rendering"),
        input
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    crate::progress::run_cancellable(&label, move |cancel| {
        render_long_inner(
            &input_owned,
            &to_owned,
            out_owned.as_deref(),
            options,
            cancel,
        )
    })
}

fn render_long_inner(
    input: &Path,
    to: &str,
    out_dir: Option<&Path>,
    options: RenderOptions,
    cancel: &AtomicBool,
) -> Result<Vec<PathBuf>> {
    let RenderOptions {
        width,
        dpi,
        quality,
        png,
    } = options;
    with_pdfium(|pdfium| {
        let document = pdfium
            .load_pdf_from_file(input, None)
            .with_context(|| format!("abrir PDF {}", input.display()))?;
        let render = render_config(width, dpi);

        // shortcut: all pages stay in memory; a huge PDF can use a lot of RAM.
        let mut pages = Vec::new();
        for (index, page) in document.pages().iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                bail!("{}", t("conversão cancelada", "conversion cancelled"));
            }
            let bitmap = page
                .render_with_config(&render)
                .with_context(|| format!("renderizar página {}", index + 1))?;
            let (width, height) = (bitmap.width() as u32, bitmap.height() as u32);
            pages.push(
                RgbaImage::from_raw(width, height, bitmap.as_rgba_bytes())
                    .context("bitmap da página em RGBA")?,
            );
        }
        if pages.is_empty() {
            bail!("{}", t("PDF sem páginas", "PDF has no pages"));
        }
        let width = pages.iter().map(|page| page.width()).max().unwrap_or(1);
        let height: u64 = pages.iter().map(|page| page.height() as u64).sum();
        if height > 30_000 {
            bail!(
                "{}",
                t(
                    "imagem longa passaria de 30000 px de altura; use --width menor",
                    "long image would exceed 30000 px; use a smaller --width"
                )
            );
        }
        let mut canvas = RgbaImage::new(width, height as u32);
        let mut y = 0i64;
        for page in &pages {
            image::imageops::overlay(&mut canvas, page, 0, y);
            y += page.height() as i64;
        }

        let dir = out_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| crate::naming::parent_of(input));
        let out = crate::naming::claim(
            &dir,
            &format!("{}_longa", crate::naming::stem_of(input)),
            to,
        )?;
        let save = crate::convert::image::SaveOptions {
            quality,
            png,
            gif: 100,
            ico: 256,
            pdf_jpeg: None,
            pdf_dpi: 96.0,
        };
        crate::convert::image::save_image(&DynamicImage::ImageRgba8(canvas), &out, to, save)?;
        Ok(vec![out.into_path()?])
    })
}

fn render_config(width: Option<u32>, dpi: Option<u32>) -> pdfium_render::prelude::PdfRenderConfig {
    match dpi {
        Some(dpi) => pdfium_render::prelude::PdfRenderConfig::new()
            .scale_page_by_factor(dpi.max(1) as f32 / 72.0),
        None => pdfium_render::prelude::PdfRenderConfig::new()
            .set_target_width(width.unwrap_or(1600).clamp(1, 100_000) as i32),
    }
}

static PDFIUM: std::sync::Mutex<Option<pdfium_render::prelude::Pdfium>> =
    std::sync::Mutex::new(None);

/// PDFium bindings are process-global; load once and serialize access.
fn with_pdfium<T>(work: impl FnOnce(&pdfium_render::prelude::Pdfium) -> Result<T>) -> Result<T> {
    let mut guard = PDFIUM.lock().unwrap_or_else(|poison| poison.into_inner());
    if guard.is_none() {
        *guard = Some(load_pdfium()?);
    }
    work(guard.as_ref().expect("pdfium loaded"))
}

fn load_pdfium() -> Result<pdfium_render::prelude::Pdfium> {
    use pdfium_render::prelude::Pdfium;
    let dll = ensure_pdfium()?;
    match Pdfium::bind_to_library(&dll) {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(_) => {
            // Half-written dll (crash, antivirus): extract it again once.
            let _ = std::fs::remove_file(&dll);
            let dll = ensure_pdfium()?;
            let bindings = Pdfium::bind_to_library(&dll)
                .with_context(|| format!("carregar {}", dll.display()))?;
            Ok(Pdfium::new(bindings))
        }
    }
}

static PDFIUM_TGZ: &[u8] = include_bytes!("../../../assets/pdfium-win-x64.tgz");
const PDFIUM_MIN_BYTES: u64 = 1_000_000;
static PDFIUM_TEMP_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn ensure_pdfium() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("settconv").join("pdfium");
    let dll = dir.join("pdfium.dll");
    if let Ok(metadata) = std::fs::metadata(&dll) {
        if metadata.len() >= PDFIUM_MIN_BYTES {
            return Ok(dll);
        }
        let _ = std::fs::remove_file(&dll);
    }
    std::fs::create_dir_all(&dir)?;
    let decoder = flate2::read::GzDecoder::new(PDFIUM_TGZ);
    let mut archive = tar::Archive::new(decoder);
    // Unique per call: parallel converters in one process share the temp folder.
    let counter = PDFIUM_TEMP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp = dir.join(format!("pdfium.{}.{}.tmp", std::process::id(), counter));
    let mut found = false;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().replace('\\', "/");
        if path.ends_with("bin/pdfium.dll") {
            entry.unpack(&temp)?;
            found = true;
            break;
        }
    }
    if !found {
        bail!(
            "{}",
            t(
                "pdfium.dll não encontrado no pacote embutido",
                "pdfium.dll missing from the embedded package"
            )
        );
    }
    if std::fs::metadata(&temp)?.len() < PDFIUM_MIN_BYTES {
        let _ = std::fs::remove_file(&temp);
        bail!(
            "{}",
            t(
                "pdfium.dll inválido no pacote embutido",
                "invalid pdfium.dll in the embedded package"
            )
        );
    }
    match std::fs::rename(&temp, &dll) {
        Ok(()) => {}
        Err(_) if dll.exists() => {
            let _ = std::fs::remove_file(&temp);
        }
        Err(e) => return Err(e.into()),
    }
    Ok(dll)
}
