// Creating archives: ZIP, TAR.GZ/BZ2/XZ and 7z.

use anyhow::{Result, bail};
use flate2::Compression;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::config::Config;
use crate::lang::t;

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
