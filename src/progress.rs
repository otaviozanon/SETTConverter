// Long conversions: marquee dialog with Cancel, only when there is no console.

use anyhow::{Context, Result, bail};
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Console::GetConsoleWindow;
use windows_sys::Win32::UI::Controls::{
    TASKDIALOGCONFIG, TDCBF_CANCEL_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_CALLBACK_TIMER,
    TDF_SHOW_PROGRESS_BAR, TDM_CLICK_BUTTON, TDM_SET_PROGRESS_BAR_MARQUEE, TDN_CREATED, TDN_TIMER,
    TaskDialogIndirect,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{IDCANCEL, IDOK, SendMessageW};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct State {
    child: Option<Child>,
}

/// Runs an external tool. From Explorer it shows a marquee dialog with Cancel;
/// from a terminal it stays quiet so the shell keeps the output.
pub fn run(label: &str, command: &mut Command) -> Result<Output> {
    command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if unsafe { !GetConsoleWindow().is_null() } {
        return command.output().context("executar a conversão");
    }

    let mut child = command.spawn().context("iniciar a conversão")?;
    // Drain stderr in parallel: a full pipe would block the tool while the dialog waits.
    let stderr = child.stderr.take();
    let reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut stderr) = stderr {
            let _ = stderr.read_to_end(&mut buffer);
        }
        buffer
    });

    let mut state = Box::new(State { child: Some(child) });
    let cancelled = dialog(label, state.as_mut());
    let mut child = state.child.take().expect("child present");
    if cancelled {
        let _ = child.kill();
        let _ = child.wait();
        bail!("conversão cancelada");
    }
    let status = child.wait().context("terminar a conversão")?;
    let stderr = reader.join().unwrap_or_default();
    Ok(Output {
        status,
        stdout: Vec::new(),
        stderr,
    })
}

fn dialog(label: &str, state: &mut State) -> bool {
    let title = wide("SETTConverter");
    let text = wide(label);
    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TDF_SHOW_PROGRESS_BAR | TDF_CALLBACK_TIMER | TDF_ALLOW_DIALOG_CANCELLATION,
        dwCommonButtons: TDCBF_CANCEL_BUTTON,
        pszWindowTitle: title.as_ptr(),
        pszMainInstruction: text.as_ptr(),
        pfCallback: Some(callback),
        lpCallbackData: state as *mut State as isize,
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
    // status < 0 means comctl32 v6 is missing: the dialog never opened.
    status < 0 || pressed == IDCANCEL
}

unsafe extern "system" fn callback(
    hwnd: HWND,
    msg: u32,
    _wparam: usize,
    _lparam: isize,
    data: isize,
) -> i32 {
    unsafe {
        let state = &mut *(data as *mut State);
        if msg == TDN_CREATED as u32 {
            SendMessageW(hwnd, TDM_SET_PROGRESS_BAR_MARQUEE as u32, 1, 0);
        } else if msg == TDN_TIMER as u32
            && let Some(child) = state.child.as_mut()
            && matches!(child.try_wait(), Ok(Some(_)))
        {
            SendMessageW(hwnd, TDM_CLICK_BUTTON as u32, IDOK as usize, 0);
        }
    }
    0
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
