// SETTConverter.exe — tambem o nome do menu do Explorer.
// Sem janela propria: usa o console do pai quando existe, senao MessageBoxW.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod convert;
mod formats;
mod menu;
mod naming;
mod progress;
mod tools;

use anyhow::{Result, bail};
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, GetConsoleWindow, WriteConsoleW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO, MessageBoxW,
};

fn main() {
    std::panic::set_hook(Box::new(crash));
    // Explorador inicia o processo sem console. Se veio do terminal, herda o do pai.
    let attached = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;
    let has_console = attached || unsafe { !GetConsoleWindow().is_null() };
    config::ensure();
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Duplo clique sem argumentos: instala o menu em vez de nao fazer nada.
    let result = if args.is_empty() && !has_console {
        install_by_double_click()
    } else {
        run(&args)
    };
    match result {
        Ok(message) => {
            if has_console && !message.is_empty() {
                console_out(&message);
            }
        }
        Err(error) => {
            let text = format!("{error:#}");
            if has_console {
                console_out(&format!("SETTConverter: {text}"));
            } else {
                message_box("SETTConverter", &text, MB_OK | MB_ICONERROR);
            }
            std::process::exit(1);
        }
    }
}

/// A panic used to exit silently under `windows_subsystem`; log it and tell the user.
fn crash(info: &std::panic::PanicHookInfo<'_>) {
    let text = format!("SETTConverter travou.\n\n{info}");
    let log = std::env::temp_dir().join("SETTConverter-crash.log");
    let _ = std::fs::write(&log, &text);
    if unsafe { GetConsoleWindow().is_null() } {
        message_box(
            "SETTConverter",
            &format!("{text}\n\nDetalhes: {}", log.display()),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn install_by_double_click() -> Result<String> {
    use windows_sys::Win32::UI::Controls::{
        TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TD_INFORMATION_ICON, TDF_ALLOW_DIALOG_CANCELLATION,
        TaskDialogIndirect,
    };

    let installed = menu::is_installed();
    let title = wide("SETTConverter");
    let instruction = wide(if installed {
        "SETTConverter já está no menu do Windows."
    } else {
        "SETTConverter não está no menu do Windows."
    });
    let content = wide(
        "Botao direito em um arquivo > Mostrar mais opcoes > SETTConverter.\n\n\
         O menu aponta para o caminho deste exe. Se mover o arquivo, instale de novo.",
    );
    let install_label = wide("Instalar no menu");
    let remove_label = wide("Remover do menu");
    let buttons = [
        TASKDIALOG_BUTTON {
            nButtonID: 1,
            pszButtonText: install_label.as_ptr(),
        },
        TASKDIALOG_BUTTON {
            nButtonID: 2,
            pszButtonText: remove_label.as_ptr(),
        },
    ];

    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
        pszWindowTitle: title.as_ptr(),
        pszMainInstruction: instruction.as_ptr(),
        pszContent: content.as_ptr(),
        cButtons: buttons.len() as u32,
        pButtons: buttons.as_ptr(),
        nDefaultButton: if installed { 2 } else { 1 },
        Anonymous1: windows_sys::Win32::UI::Controls::TASKDIALOGCONFIG_0 {
            pszMainIcon: TD_INFORMATION_ICON,
        },
        ..Default::default()
    };

    let mut pressed = 0i32;
    let status = unsafe {
        TaskDialogIndirect(
            &config,
            &mut pressed,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if status < 0 {
        // Sem comctl32 v6 (manifesto) o TaskDialog nao existe: cai no Sim/Nao.
        let question = if installed {
            "Remover SETTConverter do menu?"
        } else {
            "Adicionar SETTConverter ao menu?"
        };
        pressed = if confirm(question) == IDYES {
            if installed { 2 } else { 1 }
        } else {
            0
        };
    }

    match pressed {
        1 => {
            menu::install(&std::env::current_exe()?)?;
            message_box(
                "SETTConverter",
                "Menu instalado.\n\nBotao direito em um arquivo > Mostrar mais opcoes > SETTConverter.",
                MB_OK | MB_ICONINFORMATION,
            );
        }
        2 => menu::uninstall()?,
        _ => {}
    }
    Ok(String::new())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Converte o processo em GUI (sem console) escrevendo no console do pai exige
/// reabrir CONOUT$: os handles std do Rust ja nasceram invalidos.
fn console_out(text: &str) {
    use std::io::Write;
    // stdout redirecionado (pipe, arquivo) funciona direto; console real pode precisar de CONOUT$.
    if std::io::stdout()
        .write_all(format!("{text}\n").as_bytes())
        .is_ok()
    {
        return;
    }
    unsafe {
        let name: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
        let handle = CreateFileW(
            name.as_ptr(),
            FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            return;
        }
        let mut written = 0u32;
        for line in text.split('\n') {
            let wide: Vec<u16> = format!("{line}\r\n").encode_utf16().collect();
            WriteConsoleW(
                handle,
                wide.as_ptr(),
                wide.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            );
        }
        CloseHandle(handle);
    }
}

fn run(args: &[String]) -> Result<String> {
    let Some(command) = args.first().map(String::as_str) else {
        return help();
    };
    let rest = &args[1..];
    match command {
        "menu" => match rest.first().map(String::as_str) {
            Some("install") => {
                menu::install(&std::env::current_exe()?)?;
                Ok("Menu SETTConverter instalado. Abra o Explorer: botao direito > Mostrar mais opcoes.".into())
            }
            Some("uninstall") => {
                menu::uninstall()?;
                Ok("Menu SETTConverter removido.".into())
            }
            _ => bail!("uso: SETTConverter menu install|uninstall"),
        },
        "extract" => {
            let cfg = config::load()?;
            let mut out = None;
            let mut inputs: Vec<PathBuf> = Vec::new();
            let mut index = 0;
            while index < rest.len() {
                let arg = rest[index].clone();
                match arg.as_str() {
                    "--out" => {
                        index += 1;
                        let value = rest
                            .get(index)
                            .ok_or_else(|| anyhow::anyhow!("--out sem valor"))?;
                        out = Some(PathBuf::from(value));
                    }
                    other if other.starts_with('-') => bail!("opcao desconhecida: {other}"),
                    other => inputs.push(PathBuf::from(other)),
                }
                index += 1;
            }
            if inputs.is_empty() {
                bail!("nenhum arquivo informado");
            }
            report(convert::run(
                &convert::Options {
                    to: "extract".into(),
                    out,
                    quality: None,
                    split_every: None,
                    merge: false,
                    frames: false,
                    width: None,
                    dpi: None,
                    size: None,
                    strip: false,
                    at: None,
                    start: None,
                    end: None,
                    rotate: None,
                    title: None,
                    author: None,
                    watermark: None,
                    long: false,
                    sheet: false,
                    cover: false,
                    gpu: false,
                },
                &inputs,
                &cfg,
            )?)
        }
        "convert" => {
            let cfg = config::load()?;
            let mut to = None;
            let mut out = None;
            let mut quality = None;
            let mut split_every = None;
            let mut merge = false;
            let mut frames = false;
            let mut width = None;
            let mut dpi = None;
            let mut size = None;
            let mut strip = false;
            let mut at = None;
            let mut start = None;
            let mut end = None;
            let mut rotate = None;
            let mut title = None;
            let mut author = None;
            let mut watermark = None;
            let mut long = false;
            let mut sheet = false;
            let mut cover = false;
            let mut gpu = false;
            let mut inputs: Vec<PathBuf> = Vec::new();
            let mut index = 0;

            while index < rest.len() {
                let arg = rest[index].clone();
                let value = |index: &mut usize| -> Result<String> {
                    *index += 1;
                    rest.get(*index)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("{arg} sem valor"))
                };
                match arg.as_str() {
                    "--to" => to = Some(value(&mut index)?),
                    "--out" => out = Some(PathBuf::from(value(&mut index)?)),
                    "--quality" => quality = Some(value(&mut index)?.parse()?),
                    "--split-every" => split_every = Some(value(&mut index)?.parse()?),
                    "--width" => width = Some(value(&mut index)?.parse()?),
                    "--dpi" => dpi = Some(value(&mut index)?.parse()?),
                    "--size" => size = Some(parse_size(&value(&mut index)?)?),
                    "--strip" => strip = true,
                    "--at" => at = Some(value(&mut index)?),
                    "--start" => start = Some(value(&mut index)?),
                    "--end" => end = Some(value(&mut index)?),
                    "--rotate" => rotate = Some(value(&mut index)?.parse()?),
                    "--title" => title = Some(value(&mut index)?),
                    "--author" => author = Some(value(&mut index)?),
                    "--watermark" => watermark = Some(value(&mut index)?),
                    "--long" => long = true,
                    "--sheet" => sheet = true,
                    "--cover" => cover = true,
                    "--gpu" => gpu = true,
                    "--merge" => merge = true,
                    "--frames" => frames = true,
                    other if other.starts_with('-') => bail!("opcao desconhecida: {other}"),
                    other => inputs.push(PathBuf::from(other)),
                }
                index += 1;
            }

            let Some(to) = to else {
                bail!("falta --to <formato>");
            };
            if inputs.is_empty() {
                bail!("nenhum arquivo informado");
            }
            report(convert::run(
                &convert::Options {
                    to,
                    out,
                    quality,
                    split_every,
                    merge,
                    frames,
                    width,
                    dpi,
                    size,
                    strip,
                    at,
                    start,
                    end,
                    rotate,
                    title,
                    author,
                    watermark,
                    long,
                    sheet,
                    cover,
                    gpu,
                },
                &inputs,
                &cfg,
            )?)
        }
        "formats" => Ok(formats_help()),
        "version" | "-V" | "--version" => {
            Ok(format!("SETTConverter {}", env!("CARGO_PKG_VERSION")))
        }
        "-h" | "--help" | "help" => help(),
        other => bail!("comando desconhecido: {other}"),
    }
}

fn report(written: Vec<PathBuf>) -> Result<String> {
    if written.is_empty() {
        return Ok("nada foi escrito".into());
    }
    let mut text = String::from("pronto:");
    for path in &written {
        text.push_str("\n  ");
        text.push_str(&path.display().to_string());
    }
    Ok(text)
}

fn parse_size(text: &str) -> Result<(u32, u32)> {
    let lower = text.to_lowercase();
    match lower.split_once('x') {
        Some((width, height)) => Ok((width.parse()?, height.parse()?)),
        None => Ok((lower.parse()?, 0)),
    }
}

fn help() -> Result<String> {
    Ok(format!(
        "SETTConverter {}\n\n{}\n\n\
         Opcoes: --out DIR --size 1920x1080 --quality 1-100 --strip\n\
         \x20        --at 00:01:30 --start 10 --end 60 --gpu\n\
         \x20        --width 1600 --dpi 300 --long --sheet\n\
         \x20        --rotate 90 --title TEXTO --author TEXTO --watermark TEXTO\n\n\
         Menu do Explorer: SETTConverter menu install | uninstall\n\
         Config: SETTConverter\\Settings\\SETTConverter.toml ao lado do exe \
         (jpg_quality, webp_quality, ffmpeg, seven_zip)\n\
         FFmpeg e 7-Zip: baixados uma vez em %LOCALAPPDATA%\\SETTConverter\\tools",
        env!("CARGO_PKG_VERSION"),
        formats_help()
    ))
}

fn formats_help() -> String {
    use formats::Family::*;
    let mut text = String::from("alvos por familia:");
    for family in [
        Image, Vector, Pdf, Video, Audio, Subtitle, Document, Font, Archive,
    ] {
        let targets: Vec<&str> = formats::menu_targets(family)
            .iter()
            .map(|(_, target)| *target)
            .collect();
        text.push_str(&format!("\n  {family:?}: {}", targets.join(" ")));
    }
    text
}

fn message_box(title: &str, text: &str, flags: u32) -> i32 {
    let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let text: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), flags) }
}

fn confirm(text: &str) -> i32 {
    message_box("SETTConverter", text, MB_YESNO | MB_ICONQUESTION)
}
