# SETTConverter

A small Windows file converter: one portable executable, no installer, no background
process. Conversions run from the Explorer context menu or the command line, and the
process exits as soon as the file is written.

## Features

| Family  | Conversions                                                                                           |
| ------- | ----------------------------------------------------------------------------------------------------- |
| Image   | `jpg` `png` `webp` `bmp` `tiff` `gif` `ico` `pdf` `svg` (trace)                                       |
| Vector  | `svg`/`svgz` to `png` `jpg` `webp` `pdf`                                                              |
| PDF     | pages to `png`/`jpg`, text to `txt`, merge, split every N pages                                       |
| Video   | `mp4` `webm` `mkv` `mov` `gif`, audio extraction, single frame, all frames                            |
| Audio   | `mp3` `m4a` `wav` `flac` `ogg` `opus`                                                                 |
| Archive | extract `zip` `tar` `gz` `bz2` `xz` (`7z` `rar` via 7-Zip), repack to `tar.gz`, pack folders to `zip` |

Output goes next to the source and never overwrites: `photo (2).png`.

Images, vectors, PDF and archives are handled in-process (`image`, `resvg`, `svg2pdf`,
PDFium, `lopdf`, `zip`). Video and audio need FFmpeg; `7z`/`rar` need 7-Zip. Those two
are downloaded automatically on first use (about 40 MB) into
`%LOCALAPPDATA%\SETTConverter\tools`, so nothing has to be installed by hand.

## Requirements

To run: Windows 10 or later, x64, and internet only for the first media or `7z`/`rar`
conversion.

To build:

- Rust with the `x86_64-pc-windows-msvc` target.
- Windows SDK (`rc.exe`) for the embedded manifest and icon.

## Downloads

Grab `settconv.exe` from the [latest release](../../releases/latest). The release also
carries `tools-win64.zip` (FFmpeg and 7-Zip), which the exe fetches on demand.

## Build

```powershell
cargo build --release
```

Result: `target\release\settconv.exe`, about 12 MB, static CRT, no side-by-side DLLs.
PDFium ships inside the exe and is extracted to `%TEMP%\settconv\pdfium` on first use.

## Releases

Pushing a `v*` tag (or running the workflow manually) builds and tests on
`windows-latest`, then publishes the exe and `tools-win64.zip` as release assets.
See `.github/workflows/release.yml`.

## Install the Explorer menu

Double-click `settconv.exe` and choose **Install menu** in the dialog. This writes only
to `HKCU` (no administrator rights) and removes cleanly with **Remove menu**.

On Windows 11 the entry lives under **Show more options**.

Command line equivalent:

```powershell
settconv.exe menu install
settconv.exe menu uninstall
```

The registered command points at the executable path, so reinstall after moving the file.

## Command line

```
settconv convert --to <target> [--out DIR] [--quality 1-100] [--frames] FILE...
settconv convert --to pdf --merge DIR
settconv convert --to pdf --split-every 10 FILE.pdf
settconv convert --to zip DIR
settconv convert --to tar.gz FILE.zip
settconv extract FILE.zip
settconv formats
settconv version
```

## Configuration

Optional `settconv.toml` next to the executable:

```toml
jpg_quality = 90
webp_quality = 85
ffmpeg = "C:\\tools\\ffmpeg.exe"
seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"
```

## Layout

```
src/main.rs        entry point, argument parsing, dialogs
src/formats.rs     extension to family, menu targets
src/convert/       image, pdf, media, archive converters
src/menu.rs        HKCU context-menu registration
src/naming.rs      output naming and collision handling
src/config.rs      settconv.toml
src/tools.rs       one-time download of FFmpeg and 7-Zip
assets/            icon sources and the embedded PDFium archive
```

## Icon

Drawn in `assets/icon.svg`: two opposing orange arrows on a transparent background.
Rasterized to `assets/icon.png`, packed into `assets/icon.ico` with seven sizes
(16 to 256), embedded by `build.rs` together with the manifest.
To change it: edit the SVG, run `settconv convert --to png assets/icon.svg --out assets`,
rebuild the ICO and compile again.

## Limitations

- Image to GIF writes the first frame only; video to GIF is animated.
- PDF to image renders each page at 1600 px width.
- No OCR, Office, e-book, 3D, CAD, font, subtitle, metadata, translation or upscale support.
- FFmpeg (GPLv3) and 7-Zip (LGPL) are downloaded on demand, not embedded in the exe.
