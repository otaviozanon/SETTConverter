// PDF rotation, metadata, text watermark and recompression.

use anyhow::{Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::path::{Path, PathBuf};

use super::inherit::inherited_value;
use crate::convert::Options;

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
    Ok(vec![out.into_path()])
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
