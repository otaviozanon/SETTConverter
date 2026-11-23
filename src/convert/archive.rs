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
    Ok(vec![folder.into_path()])
}

/// Archive format for `pack_tar` / `repack`.
#[derive(Clone, Copy)]
pub enum TarKind {
    Gz,
    Bz2,
    Xz,
}

impl TarKind {
    /// From the output extension (`tar.gz`, `tar.bz2`, `tar.xz`).
    pub fn of(ext: &str) -> Option<Self> {
        match ext {
            "tar.gz" => Some(TarKind::Gz),
            "tar.bz2" => Some(TarKind::Bz2),
            "tar.xz" => Some(TarKind::Xz),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            TarKind::Gz => "tar.gz",
            TarKind::Bz2 => "tar.bz2",
            TarKind::Xz => "tar.xz",
        }
    }
}

pub fn pack_zip(inputs: &[PathBuf], out: &Path) -> Result<()> {
    let file = File::create(out)?;
    let mut zip = ZipWriter::new(BufWriter::new(file));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for input in inputs {
        if input.is_dir() {
            // Contents at the archive root: extracting makes one folder, not folder/folder.
            add_tree(&mut zip, input, input, options)?;
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

pub fn pack_tar(inputs: &[PathBuf], out: &Path, kind: TarKind) -> Result<()> {
    let file = File::create(out)?;
    match kind {
        TarKind::Gz => tar_into(
            inputs,
            flate2::write::GzEncoder::new(file, Compression::default()),
        ),
        TarKind::Bz2 => tar_into(
            inputs,
            bzip2::write::BzEncoder::new(file, bzip2::Compression::default()),
        ),
        TarKind::Xz => tar_into(inputs, xz2::write::XzEncoder::new(file, 6)),
    }
}

fn tar_into<W: std::io::Write>(inputs: &[PathBuf], writer: W) -> Result<()> {
    let mut tar = tar::Builder::new(writer);
    for input in inputs {
        if input.is_dir() {
            // Contents at the root, matching the ZIP behaviour.
            for entry in WalkDir::new(input) {
                let entry = entry?;
                let relative = entry.path().strip_prefix(input)?;
                if relative.as_os_str().is_empty() {
                    continue;
                }
                if entry.file_type().is_dir() {
                    tar.append_dir(relative, entry.path())?;
                } else {
                    tar.append_path_with_name(entry.path(), relative)?;
                }
            }
        } else {
            tar.append_path(input)?;
        }
    }
    // shortcut: o compressor fecha o trailer no Drop; erro de disco no fim não é reportado.
    tar.finish()?;
    Ok(())
}

pub fn pack_7z(inputs: &[PathBuf], out: &Path, cfg: &Config) -> Result<()> {
    let seven = crate::tools::seven_zip(cfg)?;
    // claim() already created an empty placeholder; 7-Zip will not reuse it.
    let _ = std::fs::remove_file(out);
    let out = std::path::absolute(out)?;
    let mut command = std::process::Command::new(seven);
    command.arg("a").arg("-y").arg(&out);
    if inputs.len() == 1 && inputs[0].is_dir() {
        // Run inside the folder so the archive holds its contents, not the folder itself.
        command.current_dir(&inputs[0]).arg(".");
    } else {
        for input in inputs {
            command.arg(input);
        }
    }
    let output = crate::progress::run(
        t("Compactando em 7Z...", "Compressing to 7Z..."),
        &mut command,
    )?;
    if !output.status.success() {
        bail!(
            "{} {}",
            t("7-Zip falhou:", "7-Zip failed:"),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if !out.is_file() {
        bail!(
            "{} {}",
            t("7-Zip não criou", "7-Zip did not create"),
            out.display()
        );
    }
    Ok(())
}

pub fn repack(input: &Path, out: &Path, kind: TarKind) -> Result<()> {
    let file = File::create(out)?;
    match kind {
        TarKind::Gz => repack_into(
            input,
            flate2::write::GzEncoder::new(file, Compression::default()),
        ),
        TarKind::Bz2 => repack_into(
            input,
            bzip2::write::BzEncoder::new(file, bzip2::Compression::default()),
        ),
        TarKind::Xz => repack_into(input, xz2::write::XzEncoder::new(file, 6)),
    }
}

fn repack_into<W: std::io::Write>(input: &Path, writer: W) -> Result<()> {
    let mut zip = ZipArchive::new(File::open(input)?)?;
    let mut tar = tar::Builder::new(writer);
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
    out.into_path();
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
