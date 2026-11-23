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
use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, GetConsoleWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK};

fn main() {
    std::panic::set_hook(Box::new(crash));
    // Explorer starts the process without a console. From a terminal, inherit the parent's.
    let attached = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0;
    let has_console = attached || unsafe { !GetConsoleWindow().is_null() };
    config::ensure();
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Double click without arguments: install the menu instead of doing nothing.
    let result = if args.is_empty() && !has_console {
        menu::dialog::install_by_double_click()
    } else {
        execute(&args)
    };
    match result {
        Ok(message) => {
            if has_console && !message.is_empty() {
                win::console_out(&message);
            }
        }
        Err(error) => {
            let text = format!("{error:#}");
            if has_console {
                win::console_out(&format!("SETTConverter: {text}"));
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
