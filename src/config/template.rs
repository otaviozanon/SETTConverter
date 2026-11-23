// Default settings file written on first run, one per UI language.

pub(super) const TEMPLATE_PT: &str = r##"# SETTConverter - configuracoes opcionais.
# Remova o "#" da linha que quiser mudar. Sem este arquivo, os padroes valem.

# Qualidade JPG (1-100)
# jpg_quality = 90

# Qualidade WEBP (1-100)
# webp_quality = 85

# Pasta de saida padrao (vazio = ao lado do arquivo)
# out_dir = "C:\\Convertidos"

# Sobrescrever arquivos existentes em vez de criar "nome (2)"
# overwrite = false

# Caminho proprio do FFmpeg (vazio = procura no PATH e baixa se faltar)
# ffmpeg = "C:\\tools\\ffmpeg.exe"

# Caminho proprio do 7-Zip (vazio = procura no PATH e baixa se faltar)
# seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"
"##;

pub(super) const TEMPLATE_EN: &str = r##"# SETTConverter - optional settings.
# Remove the "#" from the line you want to change. Defaults apply without this file.

# JPG quality (1-100)
# jpg_quality = 90

# WEBP quality (1-100)
# webp_quality = 85

# Default output folder (empty = next to the source file)
# out_dir = "C:\\Converted"

# Overwrite existing files instead of creating "name (2)"
# overwrite = false

# Custom FFmpeg path (empty = look in PATH and download when missing)
# ffmpeg = "C:\\tools\\ffmpeg.exe"

# Custom 7-Zip path (empty = look in PATH and download when missing)
# seven_zip = "C:\\Program Files\\7-Zip\\7z.exe"
"##;
