// Raw HKCU registry access for the context menu.

use anyhow::{Result, bail};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW, RegSetValueExW,
};

use crate::win::wide;

pub(super) const ROOT: &str = "SETTConverter";
pub(super) const FOLDER_KEY: &str = "Software\\Classes\\Directory\\shell\\SETTConverter";
pub(super) const BACKGROUND_KEY: &str =
    "Software\\Classes\\Directory\\Background\\shell\\SETTConverter";

pub(super) fn read_string(key: &str, name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{REG_SZ, RegQueryValueExW};
    let key_w = wide(key);
    let name_w = wide(name);
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, key_w.as_ptr(), 0, KEY_READ, &mut handle)
            != ERROR_SUCCESS
        {
            return None;
        }
        let mut kind = REG_SZ;
        let mut size = 0u32;
        let status = RegQueryValueExW(
            handle,
            name_w.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut size,
        );
        if status != ERROR_SUCCESS || size < 4 {
            RegCloseKey(handle);
            return None;
        }
        let mut buffer = vec![0u16; (size / 2) as usize];
        let status = RegQueryValueExW(
            handle,
            name_w.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            buffer.as_mut_ptr() as *mut u8,
            &mut size,
        );
        RegCloseKey(handle);
        if status != ERROR_SUCCESS {
            return None;
        }
        buffer.truncate(buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len()));
        Some(String::from_utf16_lossy(&buffer))
    }
}

pub(super) fn write_base(key: &str, exe: &str) -> Result<()> {
    set_string(key, "MUIVerb", ROOT)?;
    set_string(key, "Icon", exe)?;
    // An empty SubCommands turns the entry into a cascading menu with its own "shell" subkey.
    set_string(key, "SubCommands", "")
}

pub(super) fn write_command(key: &str, label: &str, command: &str) -> Result<()> {
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

pub(super) fn delete_tree(key: &str) {
    let key_w = wide(key);
    unsafe {
        RegDeleteTreeW(HKEY_CURRENT_USER, key_w.as_ptr());
    }
}
