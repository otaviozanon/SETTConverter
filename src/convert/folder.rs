// Folder input: archive it, make one PDF, or convert every supported file inside.

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::formats::{self, Family};
use crate::lang::t;

use super::{Options, one};

/// A folder: archive it, make one PDF, or convert every supported file inside.
pub(super) fn folder(options: &Options, dir: &Path, config: &Config) -> Result<Vec<PathBuf>> {
    let out_dir = options
        .out
        .clone()
        .unwrap_or_else(|| crate::naming::parent_of(dir));
    std::fs::create_dir_all(trimmed(&out_dir))?;
    let stem = crate::naming::stem_of(dir);

    match options.to.as_str() {
        "zip" => {
            let out = crate::naming::claim(&out_dir, &stem, "zip")?;
            super::archive::pack_zip(std::slice::from_ref(&dir.to_path_buf()), &out)?;
            return Ok(vec![out.into_path()]);
        }
        "7z" => {
            let out = crate::naming::claim(&out_dir, &stem, "7z")?;
            super::archive::pack_7z(std::slice::from_ref(&dir.to_path_buf()), &out, config)?;
            return Ok(vec![out.into_path()]);
        }
        "pdf" => {
            let images = files_in(dir, |family| {
                matches!(family, Family::Image | Family::Vector)
            })?;
            if images.is_empty() {
                bail!(
                    "{} {}",
                    t("nenhuma imagem em", "no images in"),
                    dir.display()
                );
            }
            let out = crate::naming::claim(&out_dir, &stem, "pdf")?;
            super::pdf::from_images(&images, &out)?;
            return Ok(vec![out.into_path()]);
        }
        other => {
            if let Some(kind) = super::archive::TarKind::of(other) {
                let out = crate::naming::claim(&out_dir, &stem, kind.extension())?;
                super::archive::pack_tar(std::slice::from_ref(&dir.to_path_buf()), &out, kind)?;
                return Ok(vec![out.into_path()]);
            }
        }
    }

    let files = files_in(dir, |family| accepts(&family, &options.to, options))?;
    if files.is_empty() {
        bail!(
            "{} .{} {} {}",
            t("nenhum arquivo compatível com", "no files compatible with"),
            options.to,
            t("em", "in"),
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
        Family::Archive => super::archive::TarKind::of(to).is_some(),
    }
}

/// `CreateDirectoryW` rejects a trailing separator.
pub(super) fn trimmed(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    let end = text.trim_end_matches(['\\', '/']);
    if end.is_empty() {
        path.to_path_buf()
    } else {
        PathBuf::from(end)
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
