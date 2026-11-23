# Real-file checklist

Manual checks with real files, outside the unit tests. Run them on a clean Windows VM
before calling a release done.

| Check | How | Status |
| --- | --- | --- |
| HEIC input fails with a clear message | `convert --to png photo.heic` | passed: decodes to JPG/PNG/WEBP |
| 30 s 720p video to WEBM | `convert --to webm clip.mp4` | passed |
| Image folder to one PDF | `convert --to pdf folder` | passed |
| Scanned-style PDF (image pages) to PNG/JPG | `convert --to jpg --dpi 40 file.pdf` | passed |
| DOCX text extraction | `convert --to txt file.docx` | passed (synthetic DOCX) |
| Real TTF to WOFF2 and back | `convert --to woff2 arial.ttf` | passed |
| AVIF round trip | `convert --to avif image.png` then back | passed |
| JPEG XL decode | `convert --to png sample.jxl` | passed (libjxl test data) |
| Animated GIF keeps frames | `convert --to mp4 anim.gif` | passed |
| 7z / tar.bz2 / tar.xz create + extract | menu / `--to` then `extract` | passed |
| Path over 260 characters | deep folder | requires `LongPathsEnabled=1`; documented |
| RAR extraction | `extract file.rar` | **pending** (no sample archive available) |
| 100+ page PDF render | `convert --to png big.pdf` | **pending** |
| Antivirus / SmartScreen on a clean VM | download and convert | out of scope (unsigned build) |
