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
PDFium, `lopdf`, `zip`). Video and audio need FFmpeg; `7z`/`rar` need 7-Zip. If the tool
is already on this PC (PATH, `Program Files\7-Zip`, WinGet/Scoop/Chocolatey), the existing
copy is used and nothing is downloaded. Otherwise it is fetched once (about 40 MB) into
`%LOCALAPPDATA%\SETTConverter\tools`, so nothing has to be installed by hand.

## Requirements

To run: Windows 10 or later, x64, and internet only for the first media or `7z`/`rar`
conversion.

## Downloads

Grab `settconv.exe` from the [latest release](https://github.com/otaviozanon/SETTConverter/releases/latest).
The release also carries `tools-win64.zip` (FFmpeg and 7-Zip), which the exe fetches on demand.

## Install the Explorer menu

Double-click `settconv.exe` and choose **Install menu** in the dialog. This writes only
to `HKCU` (no administrator rights) and removes cleanly with **Remove menu**.

On Windows 11 the entry lives under **Show more options**.

## Configuration

Optional `settconv.toml` next to the executable:

```toml
jpg_quality = 90
webp_quality = 85
ffmpeg = "C:\\tools\\ffmpeg.exe"
seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"
```

## Limitations

- Image to GIF writes the first frame only; video to GIF is animated.
- PDF to image renders each page at 1600 px width.
- No OCR, Office, e-book, 3D, CAD, font, subtitle, metadata, translation or upscale support.
- FFmpeg (GPLv3) and 7-Zip (LGPL) are downloaded on demand, not embedded in the exe.
