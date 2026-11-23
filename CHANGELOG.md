# Changelog

## 1.2.5

- `--force` writes to a temporary file and renames on success: a failed conversion
  no longer destroys the existing output.
- HEIC/HEIF and AVIF convert to PDF (decoded to a temporary PNG first).
- `--to` is case-insensitive (`--to JPG` works).

## 1.2.4

- HEIC/HEIF input: decodes to JPG, PNG, WEBP, AVIF, BMP, TIFF and the other image
  targets through FFmpeg (already used by AVIF). The Explorer menu appears for
  `.heic`/`.heif` files.
- `--quality` is honored for JPG/WEBP output of FFmpeg-decoded images.
- HEIC/AVIF to PDF still needs a PNG/JPG step and says so.

## 1.2.3

Internal organization, no behavior change:

- Target predicates (`is_media_target`) live in `formats.rs`.
- PDF page-tree inheritance shared by merge and edit (`pdf/inherit.rs`).
- Settings templates moved to `config/template.rs`.
- Conversion tests moved to `convert/tests.rs`; image dispatch to `convert/image_family.rs`.
- Explorer menu split into `menu/{mod,commands,registry,dialog}.rs`.
- Archives split into `archive/{mod,pack}.rs`.
- Tools split into `tools/{mod,locate}.rs`.
- `main.rs` is now the entry point only (121 lines); dialogs live in `menu/dialog.rs`
  and console output in `win.rs`.

## 1.2.2

Internal cleanup, no behavior change:

- `main.rs` split into `cli.rs` (argument parsing, help, tested) and `win.rs`
  (wide strings, message box, TaskDialog).
- `wide()` and `CREATE_NO_WINDOW` no longer duplicated across modules.
- `Options.force` is now actually used; menu labels live next to their formats,
  removing a duplicated translation table.
- `tools.rs` download/hash code moved to `download.rs`.
- `convert/pdf.rs` split into `render`, `edit` and `merge`.
- `convert/media.rs` split into `mod`, `codecs` and `subtitle`.
- Folder handling moved to `convert/folder.rs`.
- Tests for the CLI parser and Explorer command strings (30 total).

## 1.2.1

### Fixed

- Failed or cancelled conversions delete the reserved output instead of leaving a 0 KB file.
- `--quality` is honored when rendering PDF pages to JPG.
- `--at` seeks before the input, so a frame from a long video is fast.
- The manifest is long-path aware (Windows still needs `LongPathsEnabled`).
- PDF merge keeps the first document's title/author metadata.
- Extracted archives no longer create `folder\folder`; ZIP/TAR/7z store folder contents.
- `--out` with a trailing backslash works.
- Progress dialogs no longer report "cancelled" when comctl32 v6 is unavailable.
- Huge images fail with a clear message instead of a decoder allocation error.

### Added

- `--force` and `overwrite = true` reuse the plain output name.
- `out_dir` in the settings file for a default output folder.
- Progress and Cancel for in-process PDF rendering.
- SHA-256 check of the downloaded tools bundle; versioned tools URL with fallback.
- CI on every push/PR, `cargo audit` and Dependabot.
- Winget singleton manifest published with each release.
- Interface follows the Windows language: Portuguese (Brazil) or English.
- Tests for FFmpeg argument building and archive round trips.
- `docs/REAL-FILES.md` manual checklist.

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
