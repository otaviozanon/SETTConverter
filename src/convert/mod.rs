// Picks the converter for each input.

pub mod archive;
pub mod document;
mod folder;
pub mod font;
pub mod image;
pub mod jxl;
pub mod media;
pub mod pdf;

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::formats::{self, Family};
use crate::lang::t;

mod image_family;
use folder::trimmed;

#[derive(Default)]
pub struct Options {
    pub to: String,
    pub out: Option<PathBuf>,
    pub quality: Option<u8>,
    pub split_every: Option<usize>,
    pub merge: bool,
    pub frames: bool,
    pub width: Option<u32>,
    pub dpi: Option<u32>,
    pub size: Option<(u32, u32)>,
    pub strip: bool,
    pub at: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub rotate: Option<i64>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub watermark: Option<String>,
    pub long: bool,
    pub sheet: bool,
    pub cover: bool,
    pub gpu: bool,
    pub force: bool,
}

pub fn run(options: &Options, inputs: &[PathBuf], config: &Config) -> Result<Vec<PathBuf>> {
    if let Some(out) = &options.out {
        // CreateDirectoryW rejects a trailing separator.
        std::fs::create_dir_all(trimmed(out))?;
    }
    if options.merge {
        if options.to != "pdf" {
            bail!(
                "{}",
                t(
                    "--merge só vale com --to pdf",
                    "--merge only works with --to pdf"
                )
            );
        }
        return merge_pdfs(options, inputs);
    }
    // Photos are independent: a batch converts in parallel.
    let images = inputs.len() > 1
        && !options.sheet
        && !options.cover
        && inputs.iter().all(|input| {
            input.is_file()
                && matches!(
                    formats::family(&formats::extension_of(input)),
                    Some(Family::Image | Family::Vector)
                )
        });
    if images {
        use rayon::prelude::*;
        let batches: Vec<Vec<PathBuf>> = inputs
            .par_iter()
            .map(|input| one(options, input, config))
            .collect::<Result<Vec<_>>>()?;
        return Ok(batches.into_iter().flatten().collect());
    }
    let mut written = Vec::new();
    for input in inputs {
        written.extend(one(options, input, config)?);
    }
    Ok(written)
}

fn one(options: &Options, input: &Path, config: &Config) -> Result<Vec<PathBuf>> {
    if input.is_dir() {
        return folder::folder(options, input, config);
    }

    let extension = formats::extension_of(input);
    let family = formats::family(&extension).with_context(|| {
        format!(
            "{} .{extension}",
            t("formato não suportado:", "unsupported format:")
        )
    })?;
    let quality = options.quality.unwrap_or_else(|| {
        if options.to == "webp" {
            config.webp_quality()
        } else {
            config.jpg_quality()
        }
    });
    let out_dir = options.out.as_deref();

    match family {
        Family::Image | Family::Vector => {
            image_family::convert(options, input, &extension, quality, out_dir, config)
        }
        Family::Pdf => match options.to.as_str() {
            "txt" => pdf::text(input, out_dir),
            "png" | "jpg" if options.long => pdf::render_long(
                input,
                &options.to,
                out_dir,
                options.width,
                options.dpi,
                quality,
            ),
            "png" | "jpg" => pdf::render_pages(
                input,
                &options.to,
                out_dir,
                options.width,
                options.dpi,
                quality,
            ),
            "pdf" => match options.split_every {
                Some(every) => pdf::split(input, every, out_dir),
                // No flags: rotate/metadata/watermark or just recompress.
                None => pdf::edit(input, out_dir, options),
            },
            other => bail!(
                "{} .{other}",
                t("PDF para não suportado:", "PDF to unsupported:")
            ),
        },
        Family::Video | Family::Audio => {
            if options.cover {
                media::audio_cover(input, config, out_dir)
            } else {
                media::convert(input, &options.to, config, out_dir, options)
            }
        }
        Family::Subtitle => media::subtitle(input, &options.to, config, out_dir),
        Family::Document => document::convert(input, &options.to, out_dir),
        Family::Font => font::convert(input, &options.to, out_dir),
        Family::Archive => match options.to.as_str() {
            "extract" => archive::extract(input, config, out_dir),
            other => match archive::TarKind::of(other) {
                Some(kind) => {
                    let dir = out_dir
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| crate::naming::parent_of(input));
                    let out = crate::naming::claim(
                        &dir,
                        &crate::naming::stem_of(input),
                        kind.extension(),
                    )?;
                    archive::repack(input, &out, kind)?;
                    Ok(vec![out.into_path()])
                }
                None => bail!(
                    "{} .{other}",
                    t("arquivo para não suportado:", "archive to unsupported:")
                ),
            },
        },
    }
}

#[cfg(test)]
mod tests;

fn merge_pdfs(options: &Options, inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut pdfs = Vec::new();
    for input in inputs {
        if input.is_dir() {
            for entry in WalkDir::new(input).max_depth(1) {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && formats::extension_of(path) == "pdf" {
                    pdfs.push(path.to_path_buf());
                }
            }
        } else {
            pdfs.push(input.clone());
        }
    }
    pdfs.sort();
    if pdfs.is_empty() {
        bail!("{}", t("nenhum PDF encontrado", "no PDF found"));
    }
    let first = &pdfs[0];
    let dir = options
        .out
        .clone()
        .unwrap_or_else(|| crate::naming::parent_of(first));
    std::fs::create_dir_all(trimmed(&dir))?;
    let stem = if inputs.len() == 1 && inputs[0].is_dir() {
        crate::naming::stem_of(&inputs[0])
    } else {
        "juntado".to_string()
    };
    let out = crate::naming::claim(&dir, &stem, "pdf")?;
    pdf::merge(&pdfs, &out)?;
    Ok(vec![out.into_path()])
}
