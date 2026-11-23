// SETTConverter.exe — also the Explorer menu name.
// No own window: uses the parent console when present, otherwise MessageBoxW.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod config;
mod convert;
mod download;
mod formats;
mod lang;
mod menu;
mod naming;
mod progress;
mod tools;
mod win;

use anyhow::Result;
use lang::t;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, GetConsoleWindow, WriteConsoleW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO,
};

fn main() {
    std::panic::set_hook(Box::new(crash));
    // Explorer starts the process without a console. From a terminal, inherit the parent's.
    let attached = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;
    let has_console = attached || unsafe { !GetConsoleWindow().is_null() };
    config::ensure();
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Double click without arguments: install the menu instead of doing nothing.
    let result = if args.is_empty() && !has_console {
        install_by_double_click()
    } else {
        execute(&args)
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
                win::message_box("SETTConverter", &text, MB_OK | MB_ICONERROR);
            }
            std::process::exit(1);
        }
    }
}

fn execute(args: &[String]) -> Result<String> {
    match cli::parse(args)? {
        cli::Command::MenuInstall => {
            menu::install(&std::env::current_exe()?)?;
            Ok(t(
                "Menu SETTConverter instalado. Abra o Explorer: botao direito > Mostrar mais opcoes.",
                "SETTConverter menu installed. Open Explorer: right-click > Show more options.",
            )
            .into())
        }
        cli::Command::MenuUninstall => {
            menu::uninstall()?;
            Ok(t(
                "Menu SETTConverter removido.",
                "SETTConverter menu removed.",
            )
            .into())
        }
        cli::Command::Extract {
            mut options,
            inputs,
        }
        | cli::Command::Convert {
            mut options,
            inputs,
        } => {
            let config = config::load()?;
            naming::set_overwrite(options.force || config.overwrite.unwrap_or(false));
            if options.out.is_none() {
                options.out = config.out_dir.clone().map(PathBuf::from);
            }
            report(convert::run(&options, &inputs, &config)?)
        }
        cli::Command::Formats => Ok(cli::formats_help()),
        cli::Command::Version => Ok(format!("SETTConverter {}", env!("CARGO_PKG_VERSION"))),
        cli::Command::Help => Ok(cli::help()),
    }
}

/// A panic used to exit silently under `windows_subsystem`; log it and tell the user.
fn crash(info: &std::panic::PanicHookInfo<'_>) {
    let text = format!(
        "{}\n\n{info}",
        t("SETTConverter travou.", "SETTConverter crashed.")
    );
    let log = std::env::temp_dir().join("SETTConverter-crash.log");
    let _ = std::fs::write(&log, &text);
    if unsafe { GetConsoleWindow().is_null() } {
        win::message_box(
            "SETTConverter",
            &format!("{text}\n\n{}: {}", t("Detalhes", "Details"), log.display()),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn install_by_double_click() -> Result<String> {
    use windows_sys::Win32::UI::Controls::{
        TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TD_INFORMATION_ICON,
        TDF_ALLOW_DIALOG_CANCELLATION,
    };

    let installed = menu::is_installed();
    let title = win::wide("SETTConverter");
    let instruction = win::wide(if installed {
        t(
            "SETTConverter já está no menu do Windows.",
            "SETTConverter is already in the Windows menu.",
        )
    } else {
        t(
            "SETTConverter não está no menu do Windows.",
            "SETTConverter is not in the Windows menu.",
        )
    });
    let content = win::wide(t(
        "Botao direito em um arquivo > Mostrar mais opcoes > SETTConverter.\n\n\
         O menu aponta para o caminho deste exe. Se mover o arquivo, instale de novo.",
        "Right-click a file > Show more options > SETTConverter.\n\n\
         The menu points at this exe path. If you move the file, install again.",
    ));
    let install_label = win::wide(t("Instalar no menu", "Install menu"));
    let remove_label = win::wide(t("Remover do menu", "Remove menu"));
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
        Anonymous1: TASKDIALOGCONFIG_0 {
            pszMainIcon: TD_INFORMATION_ICON,
        },
        ..Default::default()
    };
    let (status, pressed) = win::task_dialog(&config);

    let pressed = if status < 0 {
        // No comctl32 v6 (manifest missing): fall back to Yes/No.
        let question = if installed {
            t(
                "Remover SETTConverter do menu?",
                "Remove SETTConverter from the menu?",
            )
        } else {
            t(
                "Adicionar SETTConverter ao menu?",
                "Add SETTConverter to the menu?",
            )
        };
        if confirm(question) == IDYES {
            if installed { 2 } else { 1 }
        } else {
            0
        }
    } else {
        pressed
    };

    match pressed {
        1 => {
            menu::install(&std::env::current_exe()?)?;
            win::message_box(
                "SETTConverter",
                t(
                    "Menu instalado.\n\nBotao direito em um arquivo > Mostrar mais opcoes > SETTConverter.",
                    "Menu installed.\n\nRight-click a file > Show more options > SETTConverter.",
                ),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        2 => menu::uninstall()?,
        _ => {}
    }
    Ok(String::new())
}

/// The process is a GUI app; writing to the parent console needs CONOUT$ because
/// the standard handles were born invalid.
fn console_out(text: &str) {
    use std::io::Write;
    // Redirected stdout (pipe, file) works directly; a real console may need CONOUT$.
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

fn report(written: Vec<PathBuf>) -> Result<String> {
    if written.is_empty() {
        return Ok(t("nada foi escrito", "nothing was written").into());
    }
    let mut text = String::from(t("pronto:", "done:"));
    for path in &written {
        text.push_str("\n  ");
        text.push_str(&path.display().to_string());
    }
    Ok(text)
}

fn confirm(text: &str) -> i32 {
    win::message_box("SETTConverter", text, MB_YESNO | MB_ICONQUESTION)
}
