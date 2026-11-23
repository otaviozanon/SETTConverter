// Explorer context menu: HKCU only, no installer and no admin.

use anyhow::{Result, bail};
use std::path::Path;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW, RegSetValueExW,
};

use crate::formats::{self, Family};

const ROOT: &str = "SETTConverter";
const FOLDER_KEY: &str = "Software\\Classes\\Directory\\shell\\SETTConverter";

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
            for (label, target) in formats::menu_targets(family) {
                write_command(&key, label, &command(&exe, target))?;
            }
        }
    }

    write_base(FOLDER_KEY, &exe)?;
    write_command(
        FOLDER_KEY,
        "Compactar em ZIP",
        &format!("\"{exe}\" convert --to zip \"%1\""),
    )?;
    write_command(
        FOLDER_KEY,
        "Juntar PDFs da pasta",
        &format!("\"{exe}\" convert --to pdf --merge \"%1\""),
    )?;
    Ok(())
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
    Ok(())
}

pub fn is_installed() -> bool {
    let key = wide(&verb_key("png"));
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        let status = RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_READ, &mut handle);
        if status == ERROR_SUCCESS {
            RegCloseKey(handle);
            true
        } else {
            false
        }
    }
}

fn families() -> [(Family, &'static [&'static str]); 6] {
    [
        (Family::Image, formats::IMAGE_IN),
        (Family::Vector, formats::VECTOR_IN),
        (Family::Pdf, formats::PDF_IN),
        (Family::Video, formats::VIDEO_IN),
        (Family::Audio, formats::AUDIO_IN),
        (Family::Archive, formats::ARCHIVE_IN),
    ]
}

fn verb_key(extension: &str) -> String {
    format!("Software\\Classes\\SystemFileAssociations\\.{extension}\\shell\\{ROOT}")
}

fn command(exe: &str, target: &str) -> String {
    if target == "extract" {
        return format!("\"{exe}\" extract \"%1\"");
    }
    if let Some(every) = target.strip_prefix("pdf-split:") {
        return format!("\"{exe}\" convert --to pdf --split-every {every} \"%1\"");
    }
    if let Some(format) = target.strip_prefix("frames:") {
        return format!("\"{exe}\" convert --to {format} --frames \"%1\"");
    }
    format!("\"{exe}\" convert --to {target} \"%1\"")
}

fn write_base(key: &str, exe: &str) -> Result<()> {
    set_string(key, "MUIVerb", ROOT)?;
    set_string(key, "Icon", exe)?;
    // An empty SubCommands turns the entry into a cascading menu with its own "shell" subkey.
    set_string(key, "SubCommands", "")
}

fn write_command(key: &str, label: &str, command: &str) -> Result<()> {
    let child = format!("{key}\\shell\\{label}");
    set_string(&child, "MUIVerb", label)?;
    set_string(&format!("{child}\\command"), "", command)
}

fn set_string(key: &str, name: &str, value: &str) -> Result<()> {
    let key_w = wide(key);
    let name_w = wide(name);
    let value_w = wide(value);
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_w.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut handle,
            std::ptr::null_mut(),
        );
        if status != ERROR_SUCCESS {
            bail!("RegCreateKeyExW({key}) falhou: {status}");
        }
        let status = RegSetValueExW(
            handle,
            name_w.as_ptr(),
            0,
            REG_SZ,
            value_w.as_ptr() as *const u8,
            (value_w.len() * 2) as u32,
        );
        RegCloseKey(handle);
        if status != ERROR_SUCCESS {
            bail!("RegSetValueExW({key}\\{name}) falhou: {status}");
        }
    }
    Ok(())
}

fn delete_tree(key: &str) {
    let key_w = wide(key);
    unsafe {
        RegDeleteTreeW(HKEY_CURRENT_USER, key_w.as_ptr());
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
