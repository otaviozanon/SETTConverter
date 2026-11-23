// Extension to family, and the targets the Explorer menu offers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Image,
    Vector,
    Pdf,
    Video,
    Audio,
    Subtitle,
    Document,
    Font,
    Archive,
}

pub const IMAGE_IN: &[&str] = &[
    "jpg", "jpeg", "jpe", "jfif", "png", "webp", "gif", "bmp", "dib", "tiff", "tif", "ico", "tga",
    "icb", "vda", "vst", "ppm", "pgm", "pbm", "pnm", "dds", "hdr", "exr", "qoi", "avif", "jxl",
];
pub const VECTOR_IN: &[&str] = &["svg", "svgz"];
pub const PDF_IN: &[&str] = &["pdf"];
pub const VIDEO_IN: &[&str] = &[
    "mp4", "m4v", "mov", "qt", "mkv", "webm", "avi", "divx", "wmv", "asf", "flv", "mpg", "mpeg",
    "mpe", "ts", "m2ts", "mts", "3gp", "3g2", "ogv", "vob", "mxf",
];
pub const AUDIO_IN: &[&str] = &[
    "mp3", "m4a", "m4b", "aac", "wav", "wave", "flac", "ogg", "oga", "opus", "wma", "aiff", "aif",
    "ac3", "amr", "au", "mka", "wv", "tta", "mp2",
];
pub const SUBTITLE_IN: &[&str] = &["srt", "vtt", "ass", "ssa"];
pub const DOCUMENT_IN: &[&str] = &["docx", "odt", "epub"];
pub const FONT_IN: &[&str] = &["ttf", "otf", "woff2"];
pub const ARCHIVE_IN: &[&str] = &[
    "zip", "7z", "rar", "tar", "gz", "tgz", "tar.gz", "bz2", "tar.bz2", "tbz2", "xz", "tar.xz",
    "txz",
];

const COMPOUND: &[&str] = &["tar.gz", "tar.bz2", "tar.xz"];

/// Extension of a path, lower case, without the dot. Handles "tar.gz".
pub fn extension_of(path: &std::path::Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    for compound in COMPOUND {
        if name.ends_with(&format!(".{compound}")) {
            return (*compound).to_string();
        }
    }
    name.rsplit_once('.')
        .map(|(_, ext)| ext)
        .unwrap_or("")
        .to_string()
}

pub fn family(ext: &str) -> Option<Family> {
    if IMAGE_IN.contains(&ext) {
        Some(Family::Image)
    } else if VECTOR_IN.contains(&ext) {
        Some(Family::Vector)
    } else if PDF_IN.contains(&ext) {
        Some(Family::Pdf)
    } else if VIDEO_IN.contains(&ext) {
        Some(Family::Video)
    } else if AUDIO_IN.contains(&ext) {
        Some(Family::Audio)
    } else if SUBTITLE_IN.contains(&ext) {
        Some(Family::Subtitle)
    } else if DOCUMENT_IN.contains(&ext) {
        Some(Family::Document)
    } else if FONT_IN.contains(&ext) {
        Some(Family::Font)
    } else if ARCHIVE_IN.contains(&ext) {
        Some(Family::Archive)
    } else {
        None
    }
}

/// Label and target format for each menu entry of a family.
pub fn menu_targets(family: Family) -> &'static [(&'static str, &'static str)] {
    match family {
        Family::Image => &[
            ("JPG", "jpg"),
            ("PNG", "png"),
            ("WEBP", "webp"),
            ("BMP", "bmp"),
            ("TIFF", "tiff"),
            ("GIF (animado)", "gif"),
            ("APNG (animado)", "apng"),
            ("MP4 (animação)", "mp4"),
            ("ICO", "ico"),
            ("AVIF", "avif"),
            ("PDF", "pdf"),
            ("SVG (vetorizar)", "svg"),
        ],
        Family::Vector => &[
            ("PNG", "png"),
            ("JPG", "jpg"),
            ("WEBP", "webp"),
            ("PDF", "pdf"),
        ],
        Family::Pdf => &[
            ("PNG (páginas)", "png"),
            ("JPG (páginas)", "jpg"),
            ("PNG (imagem longa)", "long:png"),
            ("TXT (texto)", "txt"),
            ("Dividir a cada 1 página", "pdf-split:1"),
            ("Dividir a cada 10 páginas", "pdf-split:10"),
            ("Dividir a cada 50 páginas", "pdf-split:50"),
            ("Rotacionar 90°", "rotate:90"),
            ("Rotacionar 180°", "rotate:180"),
            ("Comprimir", "compress"),
        ],
        Family::Video => &[
            ("MP4", "mp4"),
            ("WEBM", "webm"),
            ("MKV", "mkv"),
            ("MOV", "mov"),
            ("GIF", "gif"),
            ("APNG", "apng"),
            ("MP3", "mp3"),
            ("M4A", "m4a"),
            ("WAV", "wav"),
            ("FLAC", "flac"),
            ("PNG (frame)", "png"),
            ("JPG (frame)", "jpg"),
            ("PNG (todos os frames)", "frames:png"),
            ("JPG (todos os frames)", "frames:jpg"),
            ("JPG (folha de contatos)", "sheet:jpg"),
        ],
        Family::Audio => &[
            ("MP3", "mp3"),
            ("M4A", "m4a"),
            ("WAV", "wav"),
            ("FLAC", "flac"),
            ("OGG", "ogg"),
            ("OPUS", "opus"),
            ("Capa (JPG)", "cover"),
        ],
        Family::Subtitle => &[("SRT", "srt"), ("VTT", "vtt"), ("ASS", "ass")],
        Family::Document => &[("TXT", "txt"), ("Markdown", "md")],
        Family::Font => &[("TTF", "ttf"), ("WOFF2", "woff2")],
        Family::Archive => &[
            ("Extrair aqui", "extract"),
            ("TAR.GZ", "tar.gz"),
            ("TAR.BZ2", "tar.bz2"),
            ("TAR.XZ", "tar.xz"),
        ],
    }
}

pub fn is_image_target(to: &str) -> bool {
    matches!(
        to,
        "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "gif"
            | "bmp"
            | "tiff"
            | "tif"
            | "ico"
            | "tga"
            | "pnm"
            | "dds"
            | "hdr"
            | "exr"
            | "qoi"
            | "avif"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn compound_extension() {
        assert_eq!(extension_of(Path::new("a/b/backup.tar.gz")), "tar.gz");
        assert_eq!(extension_of(Path::new("photo.JPG")), "jpg");
        assert_eq!(extension_of(Path::new("noext")), "");
    }

    #[test]
    fn families() {
        assert_eq!(family("png"), Some(Family::Image));
        assert_eq!(family("svg"), Some(Family::Vector));
        assert_eq!(family("mp4"), Some(Family::Video));
        assert_eq!(family("zip"), Some(Family::Archive));
        assert_eq!(family("srt"), Some(Family::Subtitle));
        assert_eq!(family("docx"), Some(Family::Document));
        assert_eq!(family("woff2"), Some(Family::Font));
        assert_eq!(family("docx"), Some(Family::Document));
        assert_eq!(family("bin"), None);
    }

    #[test]
    fn every_menu_target_is_a_format() {
        for f in [
            Family::Image,
            Family::Vector,
            Family::Pdf,
            Family::Video,
            Family::Audio,
            Family::Subtitle,
            Family::Document,
            Family::Font,
            Family::Archive,
        ] {
            assert!(!menu_targets(f).is_empty());
        }
    }
}
