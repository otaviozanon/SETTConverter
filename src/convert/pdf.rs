// PDF: text, page images, merge and split. Page images need the embedded PDFium.

use anyhow::{Context, Result, bail};
use image::{DynamicImage, RgbaImage};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::convert::Options;

pub fn text(input: &Path, out_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let text = pdf_extract::extract_text(input)
        .with_context(|| format!("ler texto de {}", input.display()))?;
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), "txt")?;
    std::fs::write(&out, text)?;
    Ok(vec![out])
}

pub fn render_pages(
    input: &Path,
    to: &str,
    out_dir: Option<&Path>,
    width: Option<u32>,
    dpi: Option<u32>,
) -> Result<Vec<PathBuf>> {
    let pdfium = pdfium()?;
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
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 90)
                .encode_image(&rgba)?;
        } else {
            rgba.save_with_format(&out, image::ImageFormat::Png)?;
        }
        written.push(out);
    }
    Ok(written)
}

pub fn merge(inputs: &[PathBuf], out: &Path) -> Result<()> {
    if inputs.is_empty() {
        bail!("nenhum PDF para juntar");
    }
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut pages: Vec<Object> = Vec::new();
    let mut next_id = 1u32;

    for path in inputs {
        let mut doc =
            Document::load(path).with_context(|| format!("abrir PDF {}", path.display()))?;
        doc.renumber_objects_with(next_id);
        next_id = doc.max_id + 1;
        for (_, page_id) in doc.get_pages() {
            pages.push(page_with_inherited(&doc, page_id)?);
        }
        objects.extend(std::mem::take(&mut doc.objects));
    }

    let pages_id = (next_id, 0u16);
    let mut kids = Vec::new();
    for (index, mut page) in pages.into_iter().enumerate() {
        let id = (pages_id.0 + 1 + index as u32, 0u16);
        if let Object::Dictionary(dict) = &mut page {
            dict.set("Parent", pages_id);
        }
        objects.insert(id, page);
        kids.push(Object::Reference(id));
    }

    let catalog_id = (pages_id.0 + 1 + kids.len() as u32, 0u16);
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", kids.len() as i64);
    pages_dict.set("Kids", kids);
    objects.insert(pages_id, Object::Dictionary(pages_dict));

    let mut catalog = Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", pages_id);
    objects.insert(catalog_id, Object::Dictionary(catalog));

    let mut doc = Document::new();
    doc.objects = objects;
    doc.max_id = catalog_id.0;
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    doc.save(out)?;
    Ok(())
}

pub fn split(input: &Path, every: usize, out_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let every = every.max(1) as u32;
    let total = Document::load(input)
        .with_context(|| format!("abrir PDF {}", input.display()))?
        .get_pages()
        .len() as u32;
    if total == 0 {
        bail!("PDF sem páginas");
    }
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let stem = crate::naming::stem_of(input);
    let mut written = Vec::new();

    let mut start = 1;
    while start <= total {
        let end = (start + every - 1).min(total);
        let mut doc = Document::load(input)?;
        let remove: Vec<u32> = doc
            .get_pages()
            .keys()
            .copied()
            .filter(|n| *n < start || *n > end)
            .collect();
        doc.delete_pages(&remove);
        doc.prune_objects();
        doc.renumber_objects();
        let out = crate::naming::claim(&dir, &format!("{stem}_p{start}-{end}"), "pdf")?;
        doc.save(&out)?;
        written.push(out);
        start = end + 1;
    }
    Ok(written)
}

/// Page attributes a PDF may keep in the parent `Pages` node instead of the page itself.
const INHERITED: &[&[u8]] = &[b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Clone of a page with inherited attributes copied down; merging without this
/// loses the page size/resources of PDFs that rely on the page tree.
fn page_with_inherited(doc: &Document, page_id: ObjectId) -> Result<Object> {
    let mut page = doc.get_object(page_id)?.clone();
    if let Object::Dictionary(dict) = &mut page {
        let mut parent = dict
            .get(b"Parent")
            .ok()
            .and_then(|object| object.as_reference().ok());
        while let Some(id) = parent {
            let Ok(Object::Dictionary(parent_dict)) = doc.get_object(id) else {
                break;
            };
            for key in INHERITED {
                if !dict.has(key)
                    && let Ok(value) = parent_dict.get(key)
                {
                    dict.set(*key, value.clone());
                }
            }
            parent = parent_dict
                .get(b"Parent")
                .ok()
                .and_then(|object| object.as_reference().ok());
        }
    }
    Ok(page)
}

/// Rotates, edits metadata, stamps a text watermark and/or recompresses a PDF.
pub fn edit(input: &Path, out_dir: Option<&Path>, options: &Options) -> Result<Vec<PathBuf>> {
    let mut doc =
        Document::load(input).with_context(|| format!("abrir PDF {}", input.display()))?;
    let pages: Vec<ObjectId> = doc.get_pages().values().copied().collect();

    if let Some(degrees) = options.rotate {
        for page_id in &pages {
            let current = inherited_value(&doc, *page_id, b"Rotate")
                .and_then(|value| value.as_i64().ok())
                .unwrap_or(0);
            let rotated = (((current + degrees) % 360) + 360) % 360;
            let page = doc.get_object_mut(*page_id)?.as_dict_mut()?;
            page.set("Rotate", rotated);
        }
    }

    if options.title.is_some() || options.author.is_some() {
        let mut info = doc
            .trailer
            .get(b"Info")
            .ok()
            .and_then(|object| object.as_reference().ok())
            .and_then(|id| doc.get_object(id).ok())
            .and_then(|object| object.as_dict().ok())
            .cloned()
            .unwrap_or_default();
        if let Some(title) = &options.title {
            info.set("Title", Object::string_literal(title.clone()));
        }
        if let Some(author) = &options.author {
            info.set("Author", Object::string_literal(author.clone()));
        }
        let info_id = doc.add_object(info);
        doc.trailer.set("Info", info_id);
    }

    if let Some(text) = &options.watermark {
        watermark(&mut doc, &pages, text)?;
    }

    doc.compress();
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), "pdf")?;
    doc.save(&out)?;
    Ok(vec![out])
}

fn watermark(doc: &mut Document, pages: &[ObjectId], text: &str) -> Result<()> {
    let mut font = Dictionary::new();
    font.set("Type", "Font");
    font.set("Subtype", "Type1");
    font.set("BaseFont", "Helvetica");
    font.set("Encoding", "WinAnsiEncoding");
    let font_id = doc.add_object(font);
    // '(' and ')' and '\' are the only characters that need escaping in a PDF string.
    let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");

    for page_id in pages {
        let (width, height) = page_size(doc, *page_id);
        let size = (width.min(height) / 12.0).clamp(18.0, 72.0);
        let x = width * 0.10;
        let y = height * 0.42;
        let content = format!(
            "q 0.85 0.85 0.85 rg BT /F1 {size:.1} Tf 0.7071 0.7071 -0.7071 0.7071 {x:.1} {y:.1} Tm ({escaped}) Tj ET Q"
        );
        let content_id = doc.add_object(Stream::new(Dictionary::new(), content.into_bytes()));
        add_font_resource(doc, *page_id, font_id)?;
        let page = doc.get_object_mut(*page_id)?.as_dict_mut()?;
        let contents = page.get(b"Contents").ok().cloned();
        let updated = match contents {
            Some(Object::Reference(reference)) => Object::Array(vec![
                Object::Reference(reference),
                Object::Reference(content_id),
            ]),
            Some(Object::Array(mut items)) => {
                items.push(Object::Reference(content_id));
                Object::Array(items)
            }
            _ => Object::Reference(content_id),
        };
        page.set("Contents", updated);
    }
    Ok(())
}

fn add_font_resource(doc: &mut Document, page_id: ObjectId, font_id: ObjectId) -> Result<()> {
    let mut resources = match inherited_value(doc, page_id, b"Resources") {
        Some(Object::Dictionary(dict)) => dict,
        Some(Object::Reference(id)) => doc.get_object(id)?.as_dict()?.clone(),
        _ => Dictionary::new(),
    };
    let mut fonts = match resources.get(b"Font") {
        Ok(Object::Dictionary(dict)) => dict.clone(),
        Ok(Object::Reference(id)) => doc.get_object(*id)?.as_dict()?.clone(),
        _ => Dictionary::new(),
    };
    fonts.set("F1", font_id);
    resources.set("Font", fonts);
    doc.get_object_mut(page_id)?
        .as_dict_mut()?
        .set("Resources", resources);
    Ok(())
}

fn inherited_value(doc: &Document, start: ObjectId, key: &[u8]) -> Option<Object> {
    let mut current = Some(start);
    while let Some(id) = current {
        let object = doc.get_object(id).ok()?;
        let dict = object.as_dict().ok()?;
        if let Ok(value) = dict.get(key) {
            return Some(value.clone());
        }
        current = dict
            .get(b"Parent")
            .ok()
            .and_then(|object| object.as_reference().ok());
    }
    None
}

fn page_size(doc: &Document, page_id: ObjectId) -> (f32, f32) {
    let numbers: Vec<f32> = inherited_value(doc, page_id, b"MediaBox")
        .and_then(|object| object.as_array().ok().cloned())
        .map(|items| items.iter().filter_map(number).collect())
        .unwrap_or_default();
    match numbers.as_slice() {
        [left, bottom, right, top, ..] => {
            ((right - left).abs().max(1.0), (top - bottom).abs().max(1.0))
        }
        _ => (595.0, 842.0),
    }
}

fn number(object: &Object) -> Option<f32> {
    match object {
        Object::Real(value) => Some(*value),
        Object::Integer(value) => Some(*value as f32),
        _ => None,
    }
}

/// Every page rendered at the same width and stacked into one image.
pub fn render_long(
    input: &Path,
    to: &str,
    out_dir: Option<&Path>,
    width: Option<u32>,
    dpi: Option<u32>,
    quality: u8,
) -> Result<Vec<PathBuf>> {
    let pdfium = pdfium()?;
    let document = pdfium
        .load_pdf_from_file(input, None)
        .with_context(|| format!("abrir PDF {}", input.display()))?;
    let render = render_config(width, dpi);

    // shortcut: all pages stay in memory; a huge PDF can use a lot of RAM.
    let mut pages = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
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
        bail!("PDF sem páginas");
    }
    let width = pages.iter().map(|page| page.width()).max().unwrap_or(1);
    let height: u64 = pages.iter().map(|page| page.height() as u64).sum();
    if height > 30_000 {
        bail!("imagem longa passaria de 30000 px de altura; use --width menor");
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
    crate::convert::image::save_image(&DynamicImage::ImageRgba8(canvas), &out, to, quality)?;
    Ok(vec![out])
}

/// One image per page, in file name order (folder to PDF).
pub fn from_images(paths: &[PathBuf], out: &Path) -> Result<()> {
    if paths.is_empty() {
        bail!("nenhuma imagem encontrada");
    }
    let temp = std::env::temp_dir().join(format!("settconv-pdf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp)?;

    let mut parts = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let part = temp.join(format!("{index}.pdf"));
        let built = crate::convert::image::decode(path)
            .and_then(|image| crate::convert::image::to_pdf(&image, &part));
        if let Err(error) = built {
            let _ = std::fs::remove_dir_all(&temp);
            return Err(error).with_context(|| format!("converter {}", path.display()));
        }
        parts.push(part);
    }
    let result = merge(&parts, out);
    let _ = std::fs::remove_dir_all(&temp);
    result
}

fn render_config(width: Option<u32>, dpi: Option<u32>) -> pdfium_render::prelude::PdfRenderConfig {
    match dpi {
        Some(dpi) => pdfium_render::prelude::PdfRenderConfig::new()
            .scale_page_by_factor(dpi.max(1) as f32 / 72.0),
        None => pdfium_render::prelude::PdfRenderConfig::new()
            .set_target_width(width.unwrap_or(1600).clamp(1, 100_000) as i32),
    }
}

fn pdfium() -> Result<pdfium_render::prelude::Pdfium> {
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

static PDFIUM_TGZ: &[u8] = include_bytes!("../../assets/pdfium-win-x64.tgz");
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
        bail!("pdfium.dll não encontrado no pacote embutido");
    }
    if std::fs::metadata(&temp)?.len() < PDFIUM_MIN_BYTES {
        let _ = std::fs::remove_file(&temp);
        bail!("pdfium.dll inválido no pacote embutido");
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

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Document, Object};

    #[test]
    fn merge_needs_input() {
        assert!(merge(&[], Path::new("x.pdf")).is_err());
    }

    /// A one-page PDF that keeps MediaBox on the Pages node, as many generators do.
    fn inherited_pdf(path: &Path) {
        let mut doc = Document::with_version("1.5");
        let content = doc.add_object(lopdf::Stream::new(Dictionary::new(), Vec::new()));
        let pages_id = doc.new_object_id();
        let mut page = Dictionary::new();
        page.set("Type", "Page");
        page.set("Parent", pages_id);
        page.set("Contents", content);
        let page_id = doc.add_object(page);
        let mut pages = Dictionary::new();
        pages.set("Type", "Pages");
        pages.set("Count", 1i64);
        pages.set("Kids", vec![Object::Reference(page_id)]);
        pages.set(
            "MediaBox",
            vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(200.0),
                Object::Real(200.0),
            ],
        );
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let mut catalog = Dictionary::new();
        catalog.set("Type", "Catalog");
        catalog.set("Pages", pages_id);
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", catalog_id);
        doc.save(path).unwrap();
    }

    #[test]
    fn merge_copies_inherited_page_attributes() {
        let dir = std::env::temp_dir().join(format!("settconv-merge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let first = dir.join("a.pdf");
        let second = dir.join("b.pdf");
        inherited_pdf(&first);
        inherited_pdf(&second);
        let out = dir.join("merged.pdf");

        merge(&[first, second], &out).unwrap();
        let merged = Document::load(&out).unwrap();
        let pages = merged.get_pages();
        assert_eq!(pages.len(), 2);
        for page_id in pages.values() {
            let page = merged.get_object(*page_id).unwrap().as_dict().unwrap();
            assert!(page.has(b"MediaBox"), "page lost the inherited MediaBox");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
