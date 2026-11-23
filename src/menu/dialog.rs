// Double-click dialog: install or remove the Explorer menu.

use anyhow::Result;
use windows_sys::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONINFORMATION, MB_OK};

use crate::lang::t;
use crate::win;

use super::{install, is_installed, uninstall};

pub fn install_by_double_click() -> Result<String> {
    use windows_sys::Win32::UI::Controls::{
        TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TD_INFORMATION_ICON,
        TDF_ALLOW_DIALOG_CANCELLATION,
    };

    let installed = is_installed();
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
        if win::confirm(question) == IDYES {
            if installed { 2 } else { 1 }
        } else {
            0
        }
    } else {
        pressed
    };

    match pressed {
        1 => {
            install(&std::env::current_exe()?)?;
            win::message_box(
                "SETTConverter",
                t(
                    "Menu instalado.\n\nBotao direito em um arquivo > Mostrar mais opcoes > SETTConverter.",
                    "Menu installed.\n\nRight-click a file > Show more options > SETTConverter.",
                ),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        2 => uninstall()?,
        _ => {}
    }
    Ok(String::new())
}
