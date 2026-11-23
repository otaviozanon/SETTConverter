// PDF: text, split and folder to PDF. Render/merge/edit live in submodules.
pub mod edit;
mod inherit;
pub mod merge;
pub mod render;

pub use edit::edit;
pub use merge::merge;
pub use render::{render_long, render_pages};

use anyhow::{Context, Result, bail};
use lopdf::Document;
use std::path::{Path, PathBuf};

use crate::lang::t;

pub fn text(input: &Path, out_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let text = pdf_extract::extract_text(input)
        .with_context(|| format!("ler texto de {}", input.display()))?;
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), "txt")?;
    std::fs::write(&out, text)?;
    Ok(vec![out.into_path()?])
}

pub fn split(input: &Path, every: usize, out_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let every = every.max(1) as u32;
    let total = Document::load(input)
        .with_context(|| format!("abrir PDF {}", input.display()))?
        .get_pages()
        .len() as u32;
    if total == 0 {
        bail!("{}", t("PDF sem páginas", "PDF has no pages"));
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
        written.push(out.into_path()?);
        start = end + 1;
    }
    Ok(written)
}

pub fn from_images(paths: &[PathBuf], out: &Path) -> Result<()> {
    if paths.is_empty() {
        bail!("{}", t("nenhuma imagem encontrada", "no image found"));
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
