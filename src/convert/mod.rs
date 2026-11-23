// Picks the converter for each input.

pub mod archive;
pub mod image;
pub mod media;
pub mod pdf;

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::formats::{self, Family};

pub struct Options {
    pub to: String,
    pub out: Option<PathBuf>,
    pub quality: Option<u8>,
    pub split_every: Option<usize>,
    pub merge: bool,
    pub frames: bool,
}

pub fn run(options: &Options, inputs: &[PathBuf], config: &Config) -> Result<Vec<PathBuf>> {
    if let Some(out) = &options.out {
        std::fs::create_dir_all(out)?;
    }
    if options.merge {
        if options.to != "pdf" {
            bail!("--merge só vale com --to pdf");
        }
        return merge_pdfs(options, inputs);
    }
    let mut written = Vec::new();
    for input in inputs {
        written.extend(one(options, input, config)?);
    }
    Ok(written)
}

fn one(options: &Options, input: &Path, config: &Config) -> Result<Vec<PathBuf>> {
    if input.is_dir() {
        if options.to == "zip" {
            let dir = options
                .out
                .clone()
                .unwrap_or_else(|| crate::naming::parent_of(input));
            std::fs::create_dir_all(&dir)?;
            let out = crate::naming::unique(&dir, &crate::naming::stem_of(input), "zip");
            archive::pack_zip(std::slice::from_ref(&input.to_path_buf()), &out)?;
            return Ok(vec![out]);
        }
        bail!(
            "pasta só converte para ZIP (ou use --to pdf --merge): {}",
            input.display()
        );
    }

    let extension = formats::extension_of(input);
    let family = formats::family(&extension)
        .with_context(|| format!("formato não suportado: .{extension}"))?;
    let quality = options.quality.unwrap_or_else(|| {
        if options.to == "webp" {
            config.webp_quality()
        } else {
            config.jpg_quality()
        }
    });
    let out_dir = options.out.as_deref();

    match family {
        Family::Image => {
            if options.to == "pdf" || options.to == "svg" || formats::is_image_target(&options.to) {
                image::convert(input, &options.to, quality, out_dir)
            } else {
                bail!("imagem para .{} não suportado", options.to)
            }
        }
        Family::Vector => {
            if options.to == "pdf" || formats::is_image_target(&options.to) {
                image::convert_svg(input, &options.to, quality, out_dir)
            } else {
                bail!("vetor para .{} não suportado", options.to)
            }
        }
        Family::Pdf => match options.to.as_str() {
            "txt" => pdf::text(input, out_dir),
            "png" | "jpg" => pdf::render_pages(input, &options.to, out_dir),
            "pdf" => match options.split_every {
                Some(every) => pdf::split(input, every, out_dir),
                None => bail!("PDF para PDF precisa de --split-every <n>"),
            },
            other => bail!("PDF para .{other} não suportado"),
        },
        Family::Video | Family::Audio => {
            media::convert(input, &options.to, config, out_dir, options.frames)
        }
        Family::Archive => match options.to.as_str() {
            "extract" => archive::extract(input, config, out_dir),
            "tar.gz" => {
                let dir = options
                    .out
                    .clone()
                    .unwrap_or_else(|| crate::naming::parent_of(input));
                std::fs::create_dir_all(&dir)?;
                let out =
                    crate::naming::unique(&dir, &crate::naming::stem_of(input), "tar.gz");
                archive::repack_targz(input, &out)?;
                Ok(vec![out])
            }
            other => bail!("arquivo para .{other} não suportado"),
        },
    }
}

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
        bail!("nenhum PDF encontrado");
    }
    let first = &pdfs[0];
    let dir = options
        .out
        .clone()
        .unwrap_or_else(|| crate::naming::parent_of(first));
    std::fs::create_dir_all(&dir)?;
    let stem = if inputs.len() == 1 && inputs[0].is_dir() {
        crate::naming::stem_of(&inputs[0])
    } else {
        "juntado".to_string()
    };
    let out = crate::naming::unique(&dir, &stem, "pdf");
    pdf::merge(&pdfs, &out)?;
    Ok(vec![out])
}
