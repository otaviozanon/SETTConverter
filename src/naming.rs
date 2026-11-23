// Output paths: next to the source, never overwrite.

use std::path::{Path, PathBuf};

pub fn unique(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let suffix = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };
    let first = dir.join(format!("{stem}{suffix}"));
    if !first.exists() {
        return first;
    }
    for n in 2..10_000 {
        let candidate = dir.join(format!("{stem} ({n}){suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem} (overflow){suffix}"))
}

pub fn unique_dir(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(stem);
    if !first.exists() {
        return first;
    }
    for n in 2..10_000 {
        let candidate = dir.join(format!("{stem} ({n})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem} (overflow)"))
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
    path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
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
}
