// Explorer context menu: HKCU only, no installer and no admin.
mod commands;
pub mod dialog;
mod registry;

use anyhow::Result;
use std::path::Path;

use crate::formats::{self, Family};

use commands::{command, group_command};
use registry::{
    BACKGROUND_KEY, FOLDER_KEY, ROOT, delete_tree, read_string, write_base, write_command,
};

pub fn install(exe: &Path) -> Result<()> {
    // absolute() instead of canonicalize(): no "\\?\" prefix in the registry command.
    let exe = std::path::absolute(exe)?;
    let exe = exe.to_string_lossy().to_string();

    for (family, extensions) in families() {
        for extension in extensions {
            if extension.contains('.') {
                continue;
            }
            let key = verb_key(extension);
            write_base(&key, &exe)?;
            for (pt, en, target) in formats::menu_targets(family) {
                let label = if crate::lang::is_pt() { pt } else { en };
                write_command(&key, label, &command(&exe, target))?;
            }
        }
    }

    write_base(FOLDER_KEY, &exe)?;
    for (pt, en, target) in folder_targets() {
        let label = if crate::lang::is_pt() { pt } else { en };
        write_command(FOLDER_KEY, label, &group_command(&exe, target, "%1"))?;
    }

    // Right click on the empty area of a folder.
    write_base(BACKGROUND_KEY, &exe)?;
    for (pt, en, target) in folder_targets() {
        let label = if crate::lang::is_pt() { pt } else { en };
        write_command(BACKGROUND_KEY, label, &group_command(&exe, target, "%V"))?;
    }
    Ok(())
}

fn folder_targets() -> [(&'static str, &'static str, &'static str); 6] {
    [
        ("Compactar em ZIP", "Compress to ZIP", "zip"),
        ("Compactar em 7Z", "Compress to 7Z", "7z"),
        ("Compactar em TAR.GZ", "Compress to TAR.GZ", "tar.gz"),
        ("Compactar em TAR.BZ2", "Compress to TAR.BZ2", "tar.bz2"),
        ("Compactar em TAR.XZ", "Compress to TAR.XZ", "tar.xz"),
        ("Juntar PDFs da pasta", "Merge PDFs in folder", "pdf-merge"),
    ]
}

pub fn uninstall() -> Result<()> {
    for (_, extensions) in families() {
        for extension in extensions {
            if extension.contains('.') {
                continue;
            }
            delete_tree(&verb_key(extension));
        }
    }
    delete_tree(FOLDER_KEY);
    delete_tree(BACKGROUND_KEY);
    Ok(())
}

/// Installed *and* pointing at this exe; a moved exe must reinstall.
pub fn is_installed() -> bool {
    let key = verb_key("png");
    let Some(installed) = read_string(&key, "Icon") else {
        return false;
    };
    match std::env::current_exe() {
        Ok(exe) => std::path::absolute(exe)
            .map(|exe| installed.eq_ignore_ascii_case(&exe.to_string_lossy()))
            .unwrap_or(true),
        Err(_) => true,
    }
}

fn families() -> [(Family, &'static [&'static str]); 9] {
    [
        (Family::Image, formats::IMAGE_IN),
        (Family::Vector, formats::VECTOR_IN),
        (Family::Pdf, formats::PDF_IN),
        (Family::Video, formats::VIDEO_IN),
        (Family::Audio, formats::AUDIO_IN),
        (Family::Subtitle, formats::SUBTITLE_IN),
        (Family::Document, formats::DOCUMENT_IN),
        (Family::Font, formats::FONT_IN),
        (Family::Archive, formats::ARCHIVE_IN),
    ]
}

fn verb_key(extension: &str) -> String {
    format!("Software\\Classes\\SystemFileAssociations\\.{extension}\\shell\\{ROOT}")
}
