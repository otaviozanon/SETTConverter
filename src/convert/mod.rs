// Picks the converter for each input.

pub mod archive;
pub mod document;
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
        return folder(options, input, config);
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
        Family::Image | Family::Vector => {
            image_family(options, input, &extension, quality, out_dir, config)
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
            "png" | "jpg" => {
                pdf::render_pages(input, &options.to, out_dir, options.width, options.dpi)
            }
            "pdf" => match options.split_every {
                Some(every) => pdf::split(input, every, out_dir),
                // No flags: rotate/metadata/watermark or just recompress.
                None => pdf::edit(input, out_dir, options),
            },
            other => bail!("PDF para .{other} não suportado"),
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
                    Ok(vec![out])
                }
                None => bail!("arquivo para .{other} não suportado"),
            },
        },
    }
}

fn image_family(
    options: &Options,
    input: &Path,
    extension: &str,
    quality: u8,
    out_dir: Option<&Path>,
    config: &Config,
) -> Result<Vec<PathBuf>> {
    let to = options.to.as_str();

    // JPEG XL has no FFmpeg decoder in the essentials build.
    if extension == "jxl" {
        let mut decoded = jxl::decode(input)?;
        image::apply_size(&mut decoded, options.size);
        if to == "pdf" {
            let out = crate::naming::claim(
                &out_dir
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| crate::naming::parent_of(input)),
                &crate::naming::stem_of(input),
                "pdf",
            )?;
            image::to_pdf(&decoded, &out)?;
            return Ok(vec![out]);
        }
        if !formats::is_image_target(to) {
            bail!("JXL para .{to} não suportado");
        }
        let out = crate::naming::claim(
            &out_dir
                .map(Path::to_path_buf)
                .unwrap_or_else(|| crate::naming::parent_of(input)),
            &crate::naming::stem_of(input),
            to,
        )?;
        image::save_image(&decoded, &out, to, quality)?;
        return Ok(vec![out]);
    }

    // AVIF goes through FFmpeg; converting it to PDF needs a PNG step first.
    if extension == "avif" || to == "avif" {
        if to == "pdf" {
            bail!("AVIF para PDF não é suportado; converta para PNG ou JPG antes");
        }
        return media::still_image(input, to, config, out_dir, quality);
    }

    // Animated sources keep every frame only through FFmpeg; APNG/MP4 need it too.
    let animated = matches!(extension, "gif" | "webp");
    let animated_target = matches!(to, "gif" | "webp" | "apng" | "mp4" | "webm");
    if animated_target && (animated || matches!(to, "apng" | "mp4" | "webm")) {
        return media::animated_image(input, to, config, out_dir, options);
    }

    if formats::family(extension) == Some(Family::Vector) {
        if to == "pdf" || formats::is_image_target(to) {
            return image::convert_svg(input, to, quality, out_dir, options.size);
        }
        bail!("vetor para .{to} não suportado");
    }

    if to == "pdf" || to == "svg" || formats::is_image_target(to) {
        image::convert(input, to, quality, out_dir, options.size)
    } else {
        bail!("imagem para .{to} não suportado")
    }
}

/// A folder: archive it, make one PDF, or convert every supported file inside.
fn folder(options: &Options, dir: &Path, config: &Config) -> Result<Vec<PathBuf>> {
    let out_dir = options
        .out
        .clone()
        .unwrap_or_else(|| crate::naming::parent_of(dir));
    std::fs::create_dir_all(&out_dir)?;
    let stem = crate::naming::stem_of(dir);

    match options.to.as_str() {
        "zip" => {
            let out = crate::naming::claim(&out_dir, &stem, "zip")?;
            archive::pack_zip(std::slice::from_ref(&dir.to_path_buf()), &out)?;
            return Ok(vec![out]);
        }
        "7z" => {
            let out = crate::naming::claim(&out_dir, &stem, "7z")?;
            archive::pack_7z(std::slice::from_ref(&dir.to_path_buf()), &out, config)?;
            return Ok(vec![out]);
        }
        "pdf" => {
            let images = files_in(dir, |family| {
                matches!(family, Family::Image | Family::Vector)
            })?;
            if images.is_empty() {
                bail!("nenhuma imagem em {}", dir.display());
            }
            let out = crate::naming::claim(&out_dir, &stem, "pdf")?;
            pdf::from_images(&images, &out)?;
            return Ok(vec![out]);
        }
        other => {
            if let Some(kind) = archive::TarKind::of(other) {
                let out = crate::naming::claim(&out_dir, &stem, kind.extension())?;
                archive::pack_tar(std::slice::from_ref(&dir.to_path_buf()), &out, kind)?;
                return Ok(vec![out]);
            }
        }
    }

    let files = files_in(dir, |family| accepts(&family, &options.to, options))?;
    if files.is_empty() {
        bail!(
            "nenhum arquivo compatível com .{} em {}",
            options.to,
            dir.display()
        );
    }
    let mut written = Vec::new();
    for file in files {
        written.extend(one(options, &file, config)?);
    }
    Ok(written)
}

/// Files directly inside `dir` whose family the target accepts, sorted.
fn files_in(dir: &Path, accept: impl Fn(Family) -> bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(dir).max_depth(1) {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let family = formats::family(&formats::extension_of(path));
        if family.is_some_and(&accept) {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn accepts(family: &Family, to: &str, options: &Options) -> bool {
    match family {
        Family::Image | Family::Vector => {
            to == "pdf" || to == "svg" || formats::is_image_target(to)
        }
        Family::Video | Family::Audio => is_media_target(to) || (options.cover && to == "jpg"),
        Family::Subtitle => matches!(to, "srt" | "vtt" | "ass" | "ssa"),
        Family::Document => matches!(to, "txt" | "md"),
        Family::Font => matches!(to, "ttf" | "woff2"),
        Family::Pdf => matches!(to, "png" | "jpg" | "txt" | "pdf"),
        Family::Archive => archive::TarKind::of(to).is_some(),
    }
}

fn is_media_target(to: &str) -> bool {
    matches!(
        to,
        "mp4"
            | "webm"
            | "mkv"
            | "mov"
            | "avi"
            | "gif"
            | "apng"
            | "mp3"
            | "m4a"
            | "wav"
            | "flac"
            | "ogg"
            | "opus"
            | "png"
            | "jpg"
    )
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
    let out = crate::naming::claim(&dir, &stem, "pdf")?;
    pdf::merge(&pdfs, &out)?;
    Ok(vec![out])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::image::RgbaImage;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("settconv-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn options(to: &str) -> Options {
        Options {
            to: to.into(),
            out: None,
            quality: None,
            split_every: None,
            merge: false,
            frames: false,
            width: None,
            dpi: None,
            size: None,
            strip: false,
            at: None,
            start: None,
            end: None,
            rotate: None,
            title: None,
            author: None,
            watermark: None,
            long: false,
            sheet: false,
            cover: false,
            gpu: false,
        }
    }

    #[test]
    fn image_pdf_merge_split_and_render() {
        let dir = temp("pipeline");
        let cfg = Config::default();
        let mut pdfs = Vec::new();
        for name in ["a.png", "b.png"] {
            let png = dir.join(name);
            RgbaImage::from_pixel(8, 8, ::image::Rgba([200, 30, 30, 255]))
                .save(&png)
                .unwrap();
            let pdf = run(&options("pdf"), std::slice::from_ref(&png), &cfg).unwrap();
            assert!(pdf[0].metadata().unwrap().len() > 0);
            pdfs.push(pdf[0].clone());
        }

        let merged = run(
            &Options {
                merge: true,
                ..options("pdf")
            },
            &pdfs,
            &cfg,
        )
        .unwrap();
        let parts = run(
            &Options {
                split_every: Some(1),
                ..options("pdf")
            },
            &merged,
            &cfg,
        )
        .unwrap();
        assert_eq!(parts.len(), 2);

        let pages = run(
            &Options {
                dpi: Some(72),
                ..options("png")
            },
            &parts[0..1],
            &cfg,
        )
        .unwrap();
        assert_eq!(pages.len(), 1);
        assert!(pages[0].metadata().unwrap().len() > 0);
    }

    #[test]
    fn svg_to_png() {
        let dir = temp("svg");
        let svg = dir.join("logo.svg");
        std::fs::write(
            &svg,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#ff6600"/></svg>"##,
        )
        .unwrap();
        let png = run(
            &options("png"),
            std::slice::from_ref(&svg),
            &Config::default(),
        )
        .unwrap();
        assert_eq!(png.len(), 1);
        assert!(png[0].metadata().unwrap().len() > 0);
    }

    #[test]
    fn batch_keeps_every_output() {
        let dir = temp("batch");
        let cfg = Config::default();
        let mut inputs = Vec::new();
        for index in 0..4 {
            let png = dir.join(format!("p{index}.png"));
            RgbaImage::from_pixel(4, 4, ::image::Rgba([index, 0, 0, 255]))
                .save(&png)
                .unwrap();
            inputs.push(png);
        }
        let out = run(&options("jpg"), &inputs, &cfg).unwrap();
        assert_eq!(out.len(), 4);
        assert!(out.iter().all(|path| path.metadata().unwrap().len() > 0));
    }

    #[test]
    fn folder_to_pdf_and_long_image() {
        let dir = temp("folder");
        for name in ["a.png", "b.png"] {
            RgbaImage::from_pixel(16, 8, ::image::Rgba([0, 100, 200, 255]))
                .save(dir.join(name))
                .unwrap();
        }
        let out = run(
            &options("pdf"),
            std::slice::from_ref(&dir),
            &Config::default(),
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        let long = run(
            &Options {
                long: true,
                dpi: Some(36),
                ..options("png")
            },
            &out,
            &Config::default(),
        )
        .unwrap();
        assert!(long[0].metadata().unwrap().len() > 0);
    }

    #[test]
    fn folder_batch_converts_images() {
        let dir = temp("folder-batch");
        for name in ["a.png", "b.png"] {
            RgbaImage::from_pixel(4, 4, ::image::Rgba([10, 20, 30, 255]))
                .save(dir.join(name))
                .unwrap();
        }
        let out = run(
            &options("jpg"),
            std::slice::from_ref(&dir),
            &Config::default(),
        )
        .unwrap();
        assert_eq!(out.len(), 2);
    }
}
