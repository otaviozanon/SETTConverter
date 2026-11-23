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

/// Portuguese label, English label and target format for each menu entry.
pub fn menu_targets(family: Family) -> &'static [(&'static str, &'static str, &'static str)] {
    match family {
        Family::Image => &[
            ("JPG", "JPG", "jpg"),
            ("PNG", "PNG", "png"),
            ("WEBP", "WEBP", "webp"),
            ("BMP", "BMP", "bmp"),
            ("TIFF", "TIFF", "tiff"),
            ("GIF (animado)", "GIF (animated)", "gif"),
            ("APNG (animado)", "APNG (animated)", "apng"),
            ("MP4 (animação)", "MP4 (animation)", "mp4"),
            ("ICO", "ICO", "ico"),
            ("AVIF", "AVIF", "avif"),
            ("PDF", "PDF", "pdf"),
            ("SVG (vetorizar)", "SVG (trace)", "svg"),
        ],
        Family::Vector => &[
            ("PNG", "PNG", "png"),
            ("JPG", "JPG", "jpg"),
            ("WEBP", "WEBP", "webp"),
            ("PDF", "PDF", "pdf"),
        ],
        Family::Pdf => &[
            ("PNG (páginas)", "PNG (pages)", "png"),
            ("JPG (páginas)", "JPG (pages)", "jpg"),
            ("PNG (imagem longa)", "PNG (long image)", "long:png"),
            ("TXT (texto)", "TXT (text)", "txt"),
            (
                "Dividir a cada 1 página",
                "Split every 1 page",
                "pdf-split:1",
            ),
            (
                "Dividir a cada 10 páginas",
                "Split every 10 pages",
                "pdf-split:10",
            ),
            (
                "Dividir a cada 50 páginas",
                "Split every 50 pages",
                "pdf-split:50",
            ),
            ("Rotacionar 90°", "Rotate 90°", "rotate:90"),
            ("Rotacionar 180°", "Rotate 180°", "rotate:180"),
            ("Comprimir", "Compress", "compress"),
        ],
        Family::Video => &[
            ("MP4", "MP4", "mp4"),
            ("WEBM", "WEBM", "webm"),
            ("MKV", "MKV", "mkv"),
            ("MOV", "MOV", "mov"),
            ("GIF", "GIF", "gif"),
            ("APNG", "APNG", "apng"),
            ("MP3", "MP3", "mp3"),
            ("M4A", "M4A", "m4a"),
            ("WAV", "WAV", "wav"),
            ("FLAC", "FLAC", "flac"),
            ("PNG (frame)", "PNG (frame)", "png"),
            ("JPG (frame)", "JPG (frame)", "jpg"),
            ("PNG (todos os frames)", "PNG (all frames)", "frames:png"),
            ("JPG (todos os frames)", "JPG (all frames)", "frames:jpg"),
            (
                "JPG (folha de contatos)",
                "JPG (contact sheet)",
                "sheet:jpg",
            ),
        ],
        Family::Audio => &[
            ("MP3", "MP3", "mp3"),
            ("M4A", "M4A", "m4a"),
            ("WAV", "WAV", "wav"),
            ("FLAC", "FLAC", "flac"),
            ("OGG", "OGG", "ogg"),
            ("OPUS", "OPUS", "opus"),
            ("Capa (JPG)", "Cover (JPG)", "cover"),
        ],
        Family::Subtitle => &[
            ("SRT", "SRT", "srt"),
            ("VTT", "VTT", "vtt"),
            ("ASS", "ASS", "ass"),
        ],
        Family::Document => &[("TXT", "TXT", "txt"), ("Markdown", "Markdown", "md")],
        Family::Font => &[("TTF", "TTF", "ttf"), ("WOFF2", "WOFF2", "woff2")],
        Family::Archive => &[
            ("Extrair aqui", "Extract here", "extract"),
            ("TAR.GZ", "TAR.GZ", "tar.gz"),
            ("TAR.BZ2", "TAR.BZ2", "tar.bz2"),
            ("TAR.XZ", "TAR.XZ", "tar.xz"),
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

pub fn is_media_target(to: &str) -> bool {
    matches!(
        to,
        "mp4"
            | "webm"
            | "mkv"
            | "mov"
            | "avi"
            | "gif"
            | "apng"
            | "mp3"
            | "m4a"
            | "wav"
            | "flac"
            | "ogg"
            | "opus"
            | "png"
            | "jpg"
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
