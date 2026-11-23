// Output paths: next to the source, never overwrite, safe when several
// conversions run at the same time.

use anyhow::{Result, bail};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

/// Reserves a fresh file name by creating it exclusively; the caller then
/// overwrites it. Two parallel conversions never get the same path.
pub fn claim(dir: &Path, stem: &str, ext: &str) -> Result<PathBuf> {
    let suffix = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };
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
            Ok(_) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    bail!("nenhum nome livre para {stem}{suffix} em {}", dir.display())
}

pub fn claim_dir(dir: &Path, stem: &str) -> Result<PathBuf> {
    for n in 1..10_000 {
        let name = match n {
            1 => stem.to_string(),
            n => format!("{stem} ({n})"),
        };
        let candidate = dir.join(name);
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    bail!("nenhum nome livre para a pasta {stem} em {}", dir.display())
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

        let folder = claim_dir(&dir, "frames").unwrap();
        let again = claim_dir(&dir, "frames").unwrap();
        assert_eq!(folder.file_name().unwrap(), "frames");
        assert_eq!(again.file_name().unwrap(), "frames (2)");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
