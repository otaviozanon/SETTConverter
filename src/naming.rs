// Output paths: next to the source, never overwrite, safe when several
// conversions run at the same time.

use anyhow::{Result, bail};
use std::fs::OpenOptions;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static OVERWRITE: AtomicBool = AtomicBool::new(false);

/// `--force` / `overwrite = true`: reuse the plain name instead of ` (2)`.
pub fn set_overwrite(value: bool) {
    OVERWRITE.store(value, Ordering::Relaxed);
}

fn overwrite() -> bool {
    OVERWRITE.load(Ordering::Relaxed)
}

/// A claimed path that deletes itself unless the conversion commits it.
/// `Deref` keeps the call sites using `&path` unchanged.
pub struct Reserved {
    path: PathBuf,
    dir: bool,
    keep: std::cell::Cell<bool>,
}

impl Reserved {
    fn file(path: PathBuf, created: bool) -> Self {
        Self {
            path,
            dir: false,
            keep: std::cell::Cell::new(!created),
        }
    }

    fn folder(path: PathBuf, created: bool) -> Self {
        Self {
            path,
            dir: true,
            keep: std::cell::Cell::new(!created),
        }
    }

    /// Marks the output as finished and hands the path back.
    pub fn into_path(self) -> PathBuf {
        self.keep.set(true);
        self.path.clone()
    }
}

impl Deref for Reserved {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Reserved {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<std::ffi::OsStr> for Reserved {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.path.as_os_str()
    }
}

impl Drop for Reserved {
    fn drop(&mut self) {
        if self.keep.get() {
            return;
        }
        let _ = if self.dir {
            std::fs::remove_dir_all(&self.path)
        } else {
            std::fs::remove_file(&self.path)
        };
    }
}

/// Reserves a fresh file name by creating it exclusively; the caller then
/// overwrites it. Two parallel conversions never get the same path.
pub fn claim(dir: &Path, stem: &str, ext: &str) -> Result<Reserved> {
    let suffix = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };
    if overwrite() {
        let target = dir.join(format!("{stem}{suffix}"));
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&target)?;
        // Never delete an existing file the user asked to overwrite.
        return Ok(Reserved::file(target, false));
    }
    for n in 1..10_000 {
        let name = match n {
            1 => format!("{stem}{suffix}"),
            n => format!("{stem} ({n}){suffix}"),
        };
        let candidate = dir.join(name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(_) => return Ok(Reserved::file(candidate, true)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    bail!(
        "{} {stem}{suffix} {} {}",
        crate::lang::t("nenhum nome livre para", "no free name for"),
        crate::lang::t("em", "in"),
        dir.display()
    )
}

pub fn claim_dir(dir: &Path, stem: &str) -> Result<Reserved> {
    if overwrite() {
        let target = dir.join(stem);
        std::fs::create_dir_all(&target)?;
        return Ok(Reserved::folder(target, false));
    }
    for n in 1..10_000 {
        let name = match n {
            1 => stem.to_string(),
            n => format!("{stem} ({n})"),
        };
        let candidate = dir.join(name);
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(Reserved::folder(candidate, true)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    bail!(
        "{} {stem} {} {}",
        crate::lang::t("nenhum nome livre para a pasta", "no free folder name for"),
        crate::lang::t("em", "in"),
        dir.display()
    )
}

pub fn stem_of(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = crate::formats::extension_of(path);
    if ext.is_empty() {
        name
    } else {
        name[..name.len().saturating_sub(ext.len() + 1)].to_string()
    }
}

pub fn parent_of(path: &Path) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_handles_compound_extension() {
        assert_eq!(stem_of(Path::new("a/backup.tar.gz")), "backup");
        assert_eq!(stem_of(Path::new("a/photo.jpg")), "photo");
        assert_eq!(stem_of(Path::new("a/photo")), "photo");
    }

    #[test]
    fn claim_never_repeats_a_name() {
        let dir = std::env::temp_dir().join(format!("settconv-claim-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let first = claim(&dir, "photo", "png").unwrap();
        let second = claim(&dir, "photo", "png").unwrap();
        assert_eq!(first.file_name().unwrap(), "photo.png");
        assert_eq!(second.file_name().unwrap(), "photo (2).png");
        let _ = first.into_path();
        let _ = second.into_path();

        let folder = claim_dir(&dir, "frames").unwrap();
        let again = claim_dir(&dir, "frames").unwrap();
        assert_eq!(folder.file_name().unwrap(), "frames");
        assert_eq!(again.file_name().unwrap(), "frames (2)");
        let _ = folder.into_path();
        let _ = again.into_path();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dropped_claim_leaves_nothing() {
        let dir = std::env::temp_dir().join(format!("settconv-drop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            let _reserved = claim(&dir, "failed", "jpg").unwrap();
        }
        assert!(!dir.join("failed.jpg").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
