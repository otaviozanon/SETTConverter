// PDF: text, page images, merge and split. Page images need the embedded PDFium.

use anyhow::{Context, Result, bail};
use image::RgbaImage;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

    let render = match dpi {
        Some(dpi) => pdfium_render::prelude::PdfRenderConfig::new()
            .scale_page_by_factor(dpi.max(1) as f32 / 72.0),
        None => pdfium_render::prelude::PdfRenderConfig::new()
            .set_target_width(width.unwrap_or(1600).clamp(1, 100_000) as i32),
    };

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
    let temp = dir.join(format!("pdfium.{}.tmp", std::process::id()));
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
