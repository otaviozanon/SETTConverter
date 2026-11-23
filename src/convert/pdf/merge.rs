// PDF merge: page objects with inherited attributes copied down.

use anyhow::{Context, Result, bail};
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::lang::t;

pub fn merge(inputs: &[PathBuf], out: &Path) -> Result<()> {
    if inputs.is_empty() {
        bail!("{}", t("nenhum PDF para juntar", "no PDF to merge"));
    }
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut pages: Vec<Object> = Vec::new();
    let mut info: Option<Dictionary> = None;
    let mut next_id = 1u32;

    for (index, path) in inputs.iter().enumerate() {
        let mut doc =
            Document::load(path).with_context(|| format!("abrir PDF {}", path.display()))?;
        if index == 0 {
            info = doc
                .trailer
                .get(b"Info")
                .ok()
                .and_then(|object| object.as_reference().ok())
                .and_then(|id| doc.get_object(id).ok())
                .and_then(|object| object.as_dict().ok())
                .cloned();
        }
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
    if let Some(info) = info {
        let info_id = doc.add_object(info);
        doc.trailer.set("Info", info_id);
    }
    doc.compress();
    doc.save(out)?;
    Ok(())
}

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
