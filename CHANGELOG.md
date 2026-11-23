# Changelog

## 1.1.0

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

### Added

- `--width` and `--dpi` for PDF to image.
- Marquee progress dialog with **Cancel** for FFmpeg and 7-Zip runs started from Explorer.
- Right-click menu on the empty area of a folder.
- Image batches convert in parallel.
- `SHA256SUMS` in releases; CI verifies the FFmpeg/7-Zip downloads and runs
  `cargo fmt --check` and `cargo clippy -D warnings`.
- `LICENSE` (MIT).

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
