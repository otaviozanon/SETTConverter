# SETTConverter

A small Windows file converter: one portable executable, no installer, no background
process. Conversions run from the Explorer context menu or the command line, and the
process exits as soon as the file is written.

## Features

| Family   | Conversions                                                                                       |
| -------- | ------------------------------------------------------------------------------------------------- |
| Image    | `jpg` `png` `webp` `bmp` `tiff` `gif` `ico` `avif` `pdf` `svg` (trace), `jxl`/`heic`/`heif` (read), resize |
| Animated | `gif` `webp` `apng` keep every frame; also to `mp4`/`webm`                                        |
| Vector   | `svg`/`svgz` to `png` `jpg` `webp` `pdf`                                                          |
| PDF      | pages to `png`/`jpg`, one long image, text to `txt`, merge, split, rotate, watermark, metadata, compress |
| Video    | `mp4` `webm` `mkv` `mov` `gif` `apng`, audio extraction, frame at a time, trim, contact sheet      |
| Audio    | `mp3` `m4a` `wav` `flac` `ogg` `opus`, embedded cover to `jpg`                                    |
| Subtitle | `srt` `vtt` `ass`/`ssa`                                                                           |
| Document | `docx` `odt` `epub` to `txt`/`md` (text only)                                                     |
| Font     | `woff2` to `ttf`, `ttf`/`otf` to `woff2`                                                          |
| Archive  | extract `zip` `tar` `gz` `bz2` `xz` (`7z` `rar` via 7-Zip); create `zip` `7z` `tar.gz` `tar.bz2` `tar.xz` |

Output goes next to the source and never overwrites: `photo (2).png`. Parallel
conversions never pick the same name, phone photos are rotated by their EXIF tag, and a
failed conversion removes the reserved output instead of leaving an empty file.
A folder can be converted as a batch, packed into an archive, or turned into one PDF.

Images, vectors, PDF and archives are handled in-process (`image`, `resvg`, `svg2pdf`,
PDFium, `lopdf`, `zip`, `jxl-oxide`). Video, audio, subtitles, animated images, AVIF,
HEIC/HEIF and `7z`/`rar` use FFmpeg and 7-Zip. If the tool is already on this PC (PATH,
`Program Files\7-Zip`, WinGet/Scoop/Chocolatey), the existing copy is used and nothing is
downloaded. Otherwise it is fetched once (about 40 MB) into
`%LOCALAPPDATA%\SETTConverter\tools`, so nothing has to be installed by hand. The
download is checked against the SHA-256 published with the release.

The interface (dialogs, menu and messages) follows the Windows display language:
Portuguese (Brazil) or English. The README stays in English.

## Requirements

To run: Windows 10 or later, x64, and internet only for the first conversion that needs
FFmpeg or 7-Zip.

To build:

- Rust with the `x86_64-pc-windows-msvc` target.
- Windows SDK (`rc.exe`) for the embedded manifest and icon.
- A C++ toolchain (the WOFF2 encoder builds a small C library).

## Downloads

Grab `SETTConverter.exe` from the [latest release](https://github.com/otaviozanon/SETTConverter/releases/latest).
The release also carries `tools-win64.zip` (FFmpeg and 7-Zip), its `tools-win64.sha256`,
`SHA256SUMS` and a winget singleton manifest (`OtavioZanon.SETTConverter.yaml`):

```powershell
winget install --manifest .\OtavioZanon.SETTConverter.yaml
```

## Install the Explorer menu

Double-click `SETTConverter.exe` and choose **Install menu** in the dialog. This writes only
to `HKCU` (no administrator rights) and removes cleanly with **Remove menu**. If the exe is
moved, double-click it again to update the registered path.

On Windows 11 the entry lives under **Show more options**.

## Command line

```
SETTConverter convert --to <target> [--out DIR] [--quality 1-100] [--frames] FILE...
SETTConverter convert --to jpg --size 1920x1080 --force FILE.png
SETTConverter convert --to png|jpg --width 1600 --dpi 300 FILE.pdf
SETTConverter convert --to png --long FILE.pdf
SETTConverter convert --to pdf DIR                      (one page per image)
SETTConverter convert --to pdf --merge DIR
SETTConverter convert --to pdf --split-every 10 FILE.pdf
SETTConverter convert --to pdf --rotate 90 --watermark DRAFT FILE.pdf
SETTConverter convert --to pdf --title TEXT --author TEXT FILE.pdf
SETTConverter convert --to png --at 00:01:30 FILE.mp4
SETTConverter convert --to mp4 --start 10 --end 60 FILE.mkv
SETTConverter convert --to jpg --sheet FILE.mp4
SETTConverter convert --to mp4 --strip --gpu FILE.mov
SETTConverter convert --to zip|7z|tar.gz|tar.bz2|tar.xz DIR
SETTConverter extract [--out DIR] FILE.zip
SETTConverter formats
SETTConverter version
```

Long video/archive conversions show a progress dialog with **Cancel** when started from
Explorer; PDF rendering does too. From a terminal they stay quiet.

## Configuration

The first run creates `SETTConverter\Settings\SETTConverter.toml` next to the exe
(`%LOCALAPPDATA%\SETTConverter\Settings` when the exe folder is read-only). Every key
starts commented out; remove the `#` from what you want to change:

```toml
jpg_quality = 90
webp_quality = 85
out_dir = "C:\\Converted"
overwrite = false
ffmpeg = "C:\\tools\\ffmpeg.exe"
seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"
```

A malformed file is reported with its path instead of being silently ignored.
`overwrite = true` (or `--force`) reuses the plain name instead of ` (2)`.

## Build

```powershell
cargo build --release
```

Result: `target\release\SETTConverter.exe`, about 15 MB, static CRT, no side-by-side DLLs.
PDFium ships inside the exe and is extracted to `%TEMP%\settconv\pdfium` on first use.

## Releases

Pushing a `v*` tag (or running the workflow manually) runs `cargo fmt --check`,
`cargo clippy -D warnings` and the tests on `windows-latest`, then publishes the exe,
the tools bundle, its SHA-256, `SHA256SUMS` and the winget manifest.
Every push and pull request also runs CI with `cargo audit`.
See `.github/workflows/release.yml` and `.github/workflows/ci.yml`.

## Layout

```
src/main.rs        entry point, dialogs, console output
src/cli.rs         argument parsing and help
src/formats.rs     extension to family, targets, menu tables
src/lang.rs        Windows display language (Portuguese/English)
src/win.rs         wide strings, message box, TaskDialog, console
src/download.rs    HTTP download with SHA-256 check
src/tools/         locate FFmpeg/7-Zip, install the bundle
src/menu/          install/uninstall, command strings, registry, dialog
src/convert/       pipeline, folder, image dispatch, document, font, jxl
src/convert/pdf/   render, edit, merge, inheritance helper
src/convert/media/ video/audio, codecs, subtitles
src/convert/archive/ extract and pack
src/naming.rs      output naming, collision handling, cleanup
src/progress.rs    marquee progress dialog for long work
src/config/        SETTConverter.toml and its templates
assets/            icon sources and the embedded PDFium archive
```

## Icon

Drawn in `assets/icon.svg`: two opposing orange arrows on a transparent background.
Rasterized to `assets/icon.png`, packed into `assets/icon.ico` with seven sizes
(16 to 256), embedded by `build.rs` together with the manifest.
To change it: edit the SVG, run `SETTConverter convert --to png assets/icon.svg --out assets`,
rebuild the ICO and compile again.

## License

MIT. The downloaded FFmpeg (GPLv3) and 7-Zip (LGPL) keep their own licenses.
The WOFF2 encoder inside `woofwoof` is Apache-2.0.

## Limitations

- Static images to GIF/APNG produce a single frame; animated sources keep every frame.
- AVIF/HEIC to PDF is not supported; convert to PNG/JPG first.
- JXL and HEIC/HEIF are read-only; no JXL or HEIC encoder.
- DOCX/ODT/EPUB extraction loses layout, tables and images.
- PDF to image renders each page at 1600 px width unless `--width`/`--dpi` is given.
- Paths longer than 260 characters need `LongPathsEnabled` in Windows; the manifest is
  long-path aware, but FFmpeg/PDFium still depend on the system policy.
- No OCR, Office rendering, 3D, CAD, metadata editing beyond PDF title/author, translation
  or upscale support.
- FFmpeg (GPLv3) and 7-Zip (LGPL) are downloaded on demand, not embedded in the exe.
- The executable is not code-signed, so Windows SmartScreen may warn on first run.
