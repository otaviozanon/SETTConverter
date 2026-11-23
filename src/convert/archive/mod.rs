// Archives: extract ZIP/TAR/GZ/BZ2/XZ natively, 7z and RAR through 7-Zip.
mod pack;

pub use pack::{TarKind, pack_7z, pack_tar, pack_zip, repack};

use anyhow::{Result, bail};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use crate::config::Config;
use crate::lang::t;

pub fn extract(input: &Path, cfg: &Config, out_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let ext = crate::formats::extension_of(input);
    let base = out_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::naming::parent_of(input));
    let folder = crate::naming::claim_dir(&base, &crate::naming::stem_of(input))?;

    match ext.as_str() {
        "zip" => extract_zip(input, &folder)?,
        "tar" => tar_unpack(File::open(input)?, &folder)?,
        "tar.gz" | "tgz" => tar_unpack(flate2::read::GzDecoder::new(File::open(input)?), &folder)?,
        "tar.bz2" | "tbz2" => tar_unpack(bzip2::read::BzDecoder::new(File::open(input)?), &folder)?,
        "tar.xz" | "txz" => tar_unpack(xz2::read::XzDecoder::new(File::open(input)?), &folder)?,
        "gz" => single_stream(
            flate2::read::GzDecoder::new(File::open(input)?),
            &base,
            &crate::naming::stem_of(input),
        )?,
        "bz2" => single_stream(
            bzip2::read::BzDecoder::new(File::open(input)?),
            &base,
            &crate::naming::stem_of(input),
        )?,
        "xz" => single_stream(
            xz2::read::XzDecoder::new(File::open(input)?),
            &base,
            &crate::naming::stem_of(input),
        )?,
        "7z" | "rar" => seven_zip(input, cfg, &folder)?,
        other => bail!(
            "{} .{other}",
            t(
                "extração não suportada para",
                "extraction not supported for"
            )
        ),
    }
    Ok(vec![folder.into_path()?])
}

fn extract_zip(input: &Path, folder: &Path) -> Result<()> {
    let mut zip = ZipArchive::new(File::open(input)?)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let destination = folder.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = File::create(&destination)?;
            std::io::copy(&mut entry, &mut file)?;
        }
    }
    Ok(())
}

fn tar_unpack<R: std::io::Read>(reader: R, folder: &Path) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    archive.unpack(folder)?;
    Ok(())
}

fn single_stream<R: std::io::Read>(reader: R, base: &Path, name: &str) -> Result<()> {
    let out = crate::naming::claim(base, name, "")?;
    let mut file = File::create(&out)?;
    std::io::copy(&mut BufReader::new(reader), &mut file)?;
    out.into_path()?;
    Ok(())
}

fn seven_zip(input: &Path, cfg: &Config, folder: &Path) -> Result<()> {
    let seven = crate::tools::seven_zip(cfg)?;
    let mut command = std::process::Command::new(seven);
    command
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", folder.display()))
        .arg(input);
    let label = format!(
        "{} {}...",
        t("Extraindo", "Extracting"),
        input
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    let output = crate::progress::run(&label, &mut command)?;
    if !output.status.success() {
        bail!(
            "{} {}",
            t("7-Zip falhou:", "7-Zip failed:"),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use walkdir::WalkDir;

    #[test]
    fn zip_round_trip() {
        let dir = std::env::temp_dir().join(format!("settconv-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.txt"), b"hello").unwrap();
        let archive = dir.join("a.zip");
        pack_zip(&[dir.join("src")], &archive).unwrap();
        let target = dir.join("out");
        extract_zip(&archive, &target).unwrap();
        assert_eq!(std::fs::read(target.join("a.txt")).unwrap(), b"hello");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tar_round_trip() {
        let dir = std::env::temp_dir().join(format!("settconv-tar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.txt"), b"hello").unwrap();
        for kind in [TarKind::Gz, TarKind::Bz2, TarKind::Xz] {
            let archive = dir.join(format!("a.{}", kind.extension()));
            pack_tar(&[dir.join("src")], &archive, kind).unwrap();
            assert!(archive.metadata().unwrap().len() > 0);
            let folder = extract(&archive, &Config::default(), None).unwrap();
            let found = WalkDir::new(&folder[0])
                .into_iter()
                .filter_map(|entry| entry.ok())
                .any(|entry| entry.file_name() == "a.txt");
            assert!(found, "{} lost the file", kind.extension());
            let _ = std::fs::remove_dir_all(&folder[0]);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
