// DOCX/ODT/EPUB to plain text. Text only: no layout, tables or images.

use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use crate::lang::t;

pub fn convert(input: &Path, to: &str, out_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let text = extract(input)?;
    if text.trim().is_empty() {
        bail!(
            "{} {}",
            t("nenhum texto encontrado em", "no text found in"),
            input.display()
        );
    }
    let dir = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let out = crate::naming::claim(&dir, &crate::naming::stem_of(input), to)?;
    std::fs::write(&out, text)?;
    Ok(vec![out.into_path()])
}

fn extract(input: &Path) -> Result<String> {
    match crate::formats::extension_of(input).as_str() {
        "docx" => xml_from_zip(input, &["word/document.xml"], &["p"]),
        "odt" => xml_from_zip(input, &["content.xml"], &["p", "h"]),
        "epub" => epub(input),
        other => bail!(
            "{} .{other}",
            t("documento não suportado:", "document not supported:")
        ),
    }
}

fn epub(input: &Path) -> Result<String> {
    let mut archive = ZipArchive::new(File::open(input)?)?;
    let mut text = String::new();
    let mut seen = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name()?.to_lowercase();
        if !(name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm")) {
            continue;
        }
        let mut xml = Vec::new();
        entry.read_to_end(&mut xml)?;
        text.push_str(&xml_text(
            &xml,
            &["p", "h1", "h2", "h3", "h4", "blockquote", "li"],
        ));
        seen += 1;
    }
    if seen == 0 {
        bail!(
            "{}",
            t("EPUB sem páginas de texto", "EPUB without text pages")
        );
    }
    Ok(text.trim().to_string())
}

/// Reads one XML part of an Office container; `blocks` are the paragraph elements.
fn xml_from_zip(input: &Path, parts: &[&str], blocks: &[&str]) -> Result<String> {
    let mut archive = ZipArchive::new(File::open(input)?)?;
    for part in parts {
        if let Ok(mut entry) = archive.by_name(part) {
            let mut xml = Vec::new();
            entry
                .read_to_end(&mut xml)
                .with_context(|| format!("ler {part}"))?;
            return Ok(xml_text(&xml, blocks).trim().to_string());
        }
    }
    bail!(
        "{} {}",
        t("pacote sem", "package without"),
        parts.join(t(" ou ", " or "))
    )
}

fn xml_text(xml: &[u8], blocks: &[&str]) -> String {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut text = String::new();
    let mut in_block = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = element.local_name().as_ref().to_string();
                if blocks.contains(&name.as_str()) {
                    in_block = true;
                    if !text.ends_with("\n\n") {
                        text.push('\n');
                    }
                }
            }
            Ok(Event::End(element)) => {
                let name = element.local_name().as_ref().to_string();
                if blocks.contains(&name.as_str()) {
                    in_block = false;
                }
            }
            Ok(Event::Text(value)) => {
                if in_block {
                    let raw = value.into_inner();
                    if let Ok(value) = quick_xml::escape::unescape(&raw) {
                        text.push_str(value.trim());
                        text.push(' ');
                    }
                }
            }
            Ok(Event::GeneralRef(reference)) => {
                if in_block && let Some(value) = entity(&reference.into_inner()) {
                    text.push(' ');
                    text.push(value);
                    text.push(' ');
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    let mut cleaned = String::new();
    for line in text.lines() {
        // Entities and tags split words: keep single spaces.
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            if !cleaned.ends_with("\n\n") {
                cleaned.push('\n');
            }
        } else {
            cleaned.push_str(&line);
            cleaned.push_str("\n\n");
        }
    }
    cleaned
}

fn entity(name: &str) -> Option<char> {
    if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return char::from_u32(code);
    }
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn docx_text_is_extracted() {
        let dir = std::env::temp_dir().join(format!("settconv-docx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.docx");
        let mut writer = zip::ZipWriter::new(File::create(&path).unwrap());
        writer
            .start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        writer
            .write_all(
                br#"<?xml version="1.0"?><w:document xmlns:w="x"><w:body><w:p><w:r><w:t>Ola</w:t></w:r><w:r><w:t>mundo</w:t></w:r></w:p><w:p><w:t>Segunda linha</w:t></w:p></w:body></w:document>"#,
            )
            .unwrap();
        writer.finish().unwrap();

        let text = extract(&path).unwrap();
        assert!(text.contains("Ola mundo"), "{text}");
        assert!(text.contains("Segunda linha"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn entities_are_kept() {
    let xml = br#"<root><p>a &amp; b &#65;</p></root>"#;
    let text = xml_text(xml, &["p"]);
    assert!(text.contains("a & b A"), "{text}");
}
