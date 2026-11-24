// Default settings file written on first run, one per UI language.

pub(super) const TEMPLATE_PT: &str = r##"# SETTConverter - configuracoes.
# Remova o "#" da linha que quiser mudar. Sem este arquivo, os padroes valem.
# Valores fora da faixa sao ajustados automaticamente.

# ---------- Imagens ----------

# Qualidade JPG (1-100)
# jpg_quality = 90

# Qualidade WEBP (1-100)
# webp_quality = 85

# Qualidade AVIF (1-100; vazio = usa jpg_quality)
# avif_quality = 90

# Compressao PNG (0-9; maior = arquivo menor, mais lento)
# png_compression = 1

# Qualidade de GIF estatico (1-100; maior = melhor)
# gif_quality = 100

# Tamanho do ICO em pixels (1-256)
# ico_size = 256

# ---------- PDF ----------

# Largura padrao de PDF para imagem em pixels (dpi tem prioridade)
# pdf_width = 1600

# DPI padrao de PDF para imagem (vazio = usa pdf_width)
# pdf_dpi = 150

# Imagem para PDF: embutir JPEG em vez de RGB sem perda (usa jpg_quality)
# pdf_jpeg = false

# DPI da pagina ao criar PDF a partir de imagem (96 = tamanho de tela)
# image_pdf_dpi = 96

# ---------- Video e audio ----------

# Qualidade de video (1-100; 75 = CRF 23)
# video_quality = 75

# Preset do H.264: ultrafast, fast, medium, slow, veryslow
# video_preset = "medium"

# Bitrate de audio em AAC, MP3, Opus e Vorbis (ex.: "192k"; vazio = 192k/128k)
# audio_bitrate = "192k"

# GIF a partir de video: quadros por segundo e largura em pixels
# gif_fps = 15
# gif_width = 480

# Usar encoder de GPU (NVIDIA NVENC) quando existir; cai para CPU se falhar
# gpu = false

# Remover metadados (GPS, camera) de video e audio
# strip = false

# Codificacao de entrada das legendas (ex.: "windows-1252"; vazio = detecta)
# subtitle_encoding = "windows-1252"

# ---------- Arquivos ----------

# Pasta de saida padrao (vazio = ao lado do arquivo)
# out_dir = "C:\\Convertidos"

# Sobrescrever arquivos existentes em vez de criar "nome (2)"
# overwrite = false

# ---------- Ferramentas ----------

# Caminho proprio do FFmpeg (vazio = procura no PATH e baixa se faltar)
# ffmpeg = "C:\\tools\\ffmpeg.exe"

# Caminho proprio do 7-Zip (vazio = procura no PATH e baixa se faltar)
# seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"

# Baixar FFmpeg/7-Zip automaticamente quando faltarem
# download_tools = true
"##;

pub(super) const TEMPLATE_EN: &str = r##"# SETTConverter - settings.
# Remove the "#" from the line you want to change. Defaults apply without this file.
# Out-of-range values are adjusted automatically.

# ---------- Images ----------

# JPG quality (1-100)
# jpg_quality = 90

# WEBP quality (1-100)
# webp_quality = 85

# AVIF quality (1-100; empty = use jpg_quality)
# avif_quality = 90

# PNG compression (0-9; higher = smaller file, slower)
# png_compression = 1

# Static GIF quality (1-100; higher = better)
# gif_quality = 100

# ICO size in pixels (1-256)
# ico_size = 256

# ---------- PDF ----------

# Default width when rendering PDF to image, in pixels (dpi has priority)
# pdf_width = 1600

# Default DPI when rendering PDF to image (empty = use pdf_width)
# pdf_dpi = 150

# Image to PDF: embed JPEG instead of lossless RGB (uses jpg_quality)
# pdf_jpeg = false

# Page DPI when creating a PDF from an image (96 = screen size)
# image_pdf_dpi = 96

# ---------- Video and audio ----------

# Video quality (1-100; 75 = CRF 23)
# video_quality = 75

# H.264 preset: ultrafast, fast, medium, slow, veryslow
# video_preset = "medium"

# Audio bitrate for AAC, MP3, Opus and Vorbis (e.g. "192k"; empty = 192k/128k)
# audio_bitrate = "192k"

# GIF from video: frames per second and width in pixels
# gif_fps = 15
# gif_width = 480

# Use the GPU encoder (NVIDIA NVENC) when available; falls back to CPU on failure
# gpu = false

# Remove metadata (GPS, camera) from video and audio
# strip = false

# Subtitle input encoding (e.g. "windows-1252"; empty = detect)
# subtitle_encoding = "windows-1252"

# ---------- Files ----------

# Default output folder (empty = next to the source file)
# out_dir = "C:\\Converted"

# Overwrite existing files instead of creating "name (2)"
# overwrite = false

# ---------- Tools ----------

# Custom FFmpeg path (empty = look in PATH and download when missing)
# ffmpeg = "C:\\tools\\ffmpeg.exe"

# Custom 7-Zip path (empty = look in PATH and download when missing)
# seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"

# Download FFmpeg/7-Zip automatically when they are missing
# download_tools = true
"##;
