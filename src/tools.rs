// FFmpeg and 7-Zip: downloaded once into %LOCALAPPDATA%, so a bare exe works.

use anyhow::{Context, Result, bail};
use std::fs::File;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

use crate::config::Config;

// Assets of the latest release; built by .github/workflows/release.yml.
const TOOLS_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/releases/latest/download/tools-win64.zip"
);
const FILES: &[&str] = &["ffmpeg.exe", "7z.exe", "7z.dll"];
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn ffmpeg(cfg: &Config) -> Result<PathBuf> {
    find(cfg.ffmpeg.as_deref(), "ffmpeg.exe")
}

pub fn seven_zip(cfg: &Config) -> Result<PathBuf> {
    find(cfg.seven_zip.as_deref(), "7z.exe")
}

fn find(configured: Option<&str>, name: &str) -> Result<PathBuf> {
    if let Some(configured) = configured {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Ok(path);
        }
    }
    if let Some(path) = on_path(name) {
        return Ok(path);
    }
    let cached = dir().join(name);
    if cached.is_file() {
        return Ok(cached);
    }
    ensure()?;
    if cached.is_file() {
        Ok(cached)
    } else {
        bail!("{name} não encontrado após o download")
    }
}

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

pub fn dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("SETTConverter")
        .join("tools")
}

/// Downloads the tool bundle once. Safe to call from every conversion.
pub fn ensure() -> Result<PathBuf> {
    let dir = dir();
    if complete(&dir) {
        return Ok(dir);
    }
    notice();
    let zip_path = std::env::temp_dir().join(format!("settconv-tools-{}.zip", std::process::id()));
    // shortcut: duas conversões simultâneas podem baixar em paralelo; o rename por arquivo mantém o resultado íntegro.
    let result = download(TOOLS_URL, &zip_path).and_then(|()| install(&zip_path, &dir));
    let _ = std::fs::remove_file(&zip_path);
    result?;
    if !complete(&dir) {
        bail!("download das ferramentas incompleto");
    }
    verify(&dir)?;
    Ok(dir)
}

fn complete(dir: &Path) -> bool {
    FILES.iter().all(|name| dir.join(name).is_file())
}

fn install(zip_path: &Path, dir: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(zip_path)?)
        .context("abrir o pacote de ferramentas baixado")?;
    std::fs::create_dir_all(dir)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        // Only the three known names, and only from the archive root.
        let Ok(entry_name) = entry.name() else {
            continue;
        };
        let Some(name) = FILES.iter().find(|name| entry_name == **name) else {
            continue;
        };
        let temp = dir.join(format!("{name}.part"));
        let mut file = File::create(&temp)?;
        std::io::copy(&mut entry, &mut file)?;
        drop(file);
        std::fs::rename(&temp, dir.join(name))?;
    }
    Ok(())
}

fn download(url: &str, out: &Path) -> Result<()> {
    let curl = std::process::Command::new("curl.exe")
        .args(["-L", "--fail", "--silent", "--show-error", "--output"])
        .arg(out)
        .arg(url)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match curl {
        Ok(result) if result.status.success() && out.is_file() => return Ok(()),
        Ok(result) => bail!(
            "download falhou: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ),
        // Windows 10 anterior a 1803 não tem curl.exe: cai no PowerShell.
        Err(_) => {}
    }
    let script = format!(
        "Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'",
        url.replace('\'', "''"),
        out.display().to_string().replace('\'', "''")
    );
    let result = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("executar curl nem PowerShell")?;
    if !result.status.success() || !out.is_file() {
        bail!(
            "download falhou: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    Ok(())
}

fn verify(dir: &Path) -> Result<()> {
    let output = std::process::Command::new(dir.join("ffmpeg.exe"))
        .arg("-version")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("executar o ffmpeg baixado")?;
    if !output.status.success() {
        bail!("o ffmpeg baixado não executou");
    }
    Ok(())
}

/// Explorer runs the exe without a console: warn before a silent 40 MB download.
fn notice() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONINFORMATION, MB_OK, MessageBoxW,
    };
    if unsafe { !GetConsoleWindow().is_null() } {
        return;
    }
    let title = wide("SETTConverter");
    let text = wide(
        "Baixando FFmpeg e 7-Zip (cerca de 40 MB) uma única vez.\n\n\
         Pode levar alguns minutos.",
    );
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        )
    };
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn install_keeps_only_known_root_files() {
        let root = std::env::temp_dir().join(format!("settconv-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let archive = root.join("tools.zip");
        let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
        for name in ["ffmpeg.exe", "7z.exe", "7z.dll", "../evil.txt", "sub/7z.exe"] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap();

        let target = root.join("tools");
        install(&archive, &target).unwrap();
        let mut names: Vec<String> = std::fs::read_dir(&target)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(names, ["7z.dll", "7z.exe", "ffmpeg.exe"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
