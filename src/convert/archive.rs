// Archives: extract ZIP/TAR/GZ/BZ2/XZ natively, 7z and RAR through 7-Zip.

use anyhow::{Result, bail};
use flate2::Compression;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::config::Config;

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
        other => bail!("extração não suportada para .{other}"),
    }
    Ok(vec![folder])
}

pub fn pack_zip(inputs: &[PathBuf], out: &Path) -> Result<()> {
    let file = File::create(out)?;
    let mut zip = ZipWriter::new(BufWriter::new(file));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for input in inputs {
        if input.is_dir() {
            let base = input.parent().unwrap_or_else(|| Path::new("."));
            add_tree(&mut zip, base, input, options)?;
        } else {
            let name = input
                .file_name()
                .map(|n| n.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| "file".into());
            zip.start_file(name, options)?;
            std::io::copy(&mut File::open(input)?, &mut zip)?;
        }
    }
    zip.finish()?;
    Ok(())
}

pub fn repack_targz(input: &Path, out: &Path) -> Result<()> {
    let mut zip = ZipArchive::new(File::open(input)?)?;
    let encoder = flate2::write::GzEncoder::new(File::create(out)?, Compression::default());
    let mut tar = tar::Builder::new(encoder);
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let mut header = tar::Header::new_gnu();
        header.set_size(entry.size());
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, &mut entry)?;
    }
    tar.finish()?;
    Ok(())
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
        "Extraindo {}...",
        input
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    let output = crate::progress::run(&label, &mut command)?;
    if !output.status.success() {
        bail!(
            "7-Zip falhou: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn add_tree<W: std::io::Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    base: &Path,
    dir: &Path,
    options: SimpleFileOptions,
) -> Result<()> {
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let name = relative.to_string_lossy().replace('\\', "/");
        if entry.file_type().is_dir() {
            zip.add_directory(format!("{name}/"), options)?;
        } else {
            zip.start_file(name, options)?;
            std::io::copy(&mut File::open(path)?, zip)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(std::fs::read(target.join("src/a.txt")).unwrap(), b"hello");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
