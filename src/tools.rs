// FFmpeg and 7-Zip: downloaded once into %LOCALAPPDATA%, so a bare exe works.

use anyhow::{Context, Result, bail};
use std::fs::File;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::lang::t;
use crate::win::{CREATE_NO_WINDOW, message_box, wide};

const FILES: &[&str] = &["ffmpeg.exe", "7z.exe", "7z.dll"];

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
    if let Some(path) = locate(name) {
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
        bail!(
            "{} {name}",
            t(
                "não encontrado após o download:",
                "not found after download:"
            )
        )
    }
}

/// Already usable on this PC: on PATH or in a standard install location.
fn locate(name: &str) -> Option<PathBuf> {
    let on_path = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    });
    on_path.or_else(|| installed(name))
}

fn installed(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if name == "7z.exe" {
        for base in [user_env("ProgramFiles"), user_env("ProgramFiles(x86)")]
            .into_iter()
            .flatten()
        {
            candidates.push(base.join("7-Zip").join("7z.exe"));
        }
    } else {
        if let Some(local) = user_env("LOCALAPPDATA") {
            candidates.push(
                local
                    .join("Microsoft")
                    .join("WinGet")
                    .join("Links")
                    .join("ffmpeg.exe"),
            );
        }
        if let Some(home) = user_env("USERPROFILE") {
            candidates.push(home.join("scoop").join("shims").join("ffmpeg.exe"));
        }
        candidates.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin\ffmpeg.exe"));
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn user_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
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
    // One instance downloads; the others wait and then reuse the result.
    let _guard = lock(&dir)?;
    if complete(&dir) {
        return Ok(dir);
    }
    notice();
    let zip_path = std::env::temp_dir().join(format!("settconv-tools-{}.zip", std::process::id()));
    let result = crate::download::bundle(&zip_path).and_then(|()| install(&zip_path, &dir));
    let _ = std::fs::remove_file(&zip_path);
    result?;
    if !complete(&dir) {
        bail!(
            "{}",
            t(
                "download das ferramentas incompleto",
                "tools download incomplete"
            )
        );
    }
    verify(&dir)?;
    Ok(dir)
}

/// Exclusive lock on the tools folder: CreateFileW with no sharing, retried.
fn lock(dir: &Path) -> Result<File> {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::{GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_ALWAYS,
    };

    std::fs::create_dir_all(dir)?;
    let path = wide(&dir.join(".lock").to_string_lossy());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    loop {
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            return Ok(unsafe { File::from_raw_handle(handle as _) });
        }
        if std::time::Instant::now() > deadline {
            bail!(
                "{}",
                t(
                    "outra conversão está baixando as ferramentas; tente de novo em instantes",
                    "another conversion is downloading the tools; try again in a moment"
                )
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
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
        if let Err(error) = std::fs::rename(&temp, dir.join(name)) {
            // Another instance may be replacing the same file right now.
            if dir.join(name).is_file() {
                let _ = std::fs::remove_file(&temp);
            } else {
                return Err(error.into());
            }
        }
    }
    Ok(())
}

fn verify(dir: &Path) -> Result<()> {
    let mut attempts = 0;
    loop {
        let output = std::process::Command::new(dir.join("ffmpeg.exe"))
            .arg("-version")
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        match output {
            Ok(output) if output.status.success() => return Ok(()),
            // A parallel instance may be swapping the file: retry once.
            Err(_) if attempts == 0 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            Ok(_) => bail!(
                "{}",
                t(
                    "o ffmpeg baixado não executou",
                    "the downloaded ffmpeg did not run"
                )
            ),
            Err(error) => {
                return Err(error).context(t(
                    "executar o ffmpeg baixado",
                    "running the downloaded ffmpeg",
                ));
            }
        }
    }
}

/// Explorer runs the exe without a console: warn before a silent 40 MB download.
fn notice() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK};
    if unsafe { !GetConsoleWindow().is_null() } {
        return;
    }
    message_box(
        "SETTConverter",
        t(
            "Baixando FFmpeg e 7-Zip (cerca de 40 MB) uma única vez.\n\n\
             Pode levar alguns minutos.",
            "Downloading FFmpeg and 7-Zip (about 40 MB) once.\n\n\
             This can take a few minutes.",
        ),
        MB_OK | MB_ICONINFORMATION,
    );
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
        for name in [
            "ffmpeg.exe",
            "7z.exe",
            "7z.dll",
            "../evil.txt",
            "sub/7z.exe",
        ] {
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
