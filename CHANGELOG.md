# Changelog

## 1.2.0

### Added

- Animated GIF/WebP/APNG keep every frame and convert to `mp4`/`webm`.
- AVIF read/write (FFmpeg) and JPEG XL read (jxl-oxide).
- Subtitle conversion: `srt` `vtt` `ass`/`ssa`.
- DOCX/ODT/EPUB to `txt`/`md` (text only).
- Fonts: WOFF2 to TTF and TTF/OTF to WOFF2.
- PDF: one long image (`--long`), folder of images to one PDF, rotate (`--rotate`),
  title/author (`--title`, `--author`), text watermark (`--watermark`), compress.
- Video/audio: frame at a time (`--at`), trim (`--start`/`--end`), contact sheet
  (`--sheet`), audio cover extraction (`--cover`), `--gpu` with CPU fallback, `--strip`.
- Image resize (`--size WxH`).
- Create `7z`, `tar.bz2` and `tar.xz`; folder background menu covers them too.
- Batch conversion of a folder's contents; folder to PDF.
- New menu families: Subtitle, Document, Font.

### Fixed

- JPEG/TIFF photos are rotated by their EXIF orientation instead of being saved sideways.
- Merging PDFs keeps page attributes inherited from the page tree (`MediaBox`, `CropBox`,
  `Resources`, `Rotate`), so those files no longer come out blank or cropped.
- A malformed `SETTConverter.toml` is reported with its path instead of being ignored.
- Two conversions running at the same time can no longer pick the same output name.
- `extract` accepts `--out DIR`, like every other command.
- A corrupt `pdfium.dll` is detected and extracted again.
- A panic in the windowless build writes `%TEMP%\SETTConverter-crash.log` and shows a message.
- The Explorer menu detects a moved exe and offers to reinstall.

### Release

- Marquee progress dialog with **Cancel** for FFmpeg and 7-Zip runs started from Explorer.
- `SHA256SUMS` in releases; CI verifies the FFmpeg/7-Zip downloads and runs
  `cargo fmt --check` and `cargo clippy -D warnings`.
- `LICENSE` (MIT).

## 1.1.0

### Fixed

- EXIF orientation for JPEG/TIFF, inherited PDF page attributes when merging, malformed
  config reported, atomic output names, `extract --out`, corrupt PDFium re-extract,
  panic log, moved-exe detection.

### Added

- `--width`/`--dpi` for PDF to image, progress dialog with Cancel for FFmpeg/7-Zip,
  folder background menu, parallel image batches, SHA256SUMS, fmt/clippy/test gates,
  LICENSE and CHANGELOG.

## 1.0.2

- Executable renamed to `SETTConverter.exe` (release assets follow).
- First run creates `SETTConverter\Settings\SETTConverter.toml` with every key commented out.

## 1.0.1

- Use FFmpeg/7-Zip already installed on the PC; download only when missing.
- The download notice no longer appears when the tools are already available.
- Concurrent first runs serialize on a lock file.

## 1.0.0

- First public release: image, vector, PDF, video, audio and archive conversion, plus the
  Explorer context menu.
