// Long conversions: marquee dialog with Cancel, only when there is no console.

use anyhow::{Context, Result, bail};
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Console::GetConsoleWindow;
use windows_sys::Win32::UI::Controls::{
    TASKDIALOGCONFIG, TDCBF_CANCEL_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_CALLBACK_TIMER,
    TDF_SHOW_PROGRESS_BAR, TDM_CLICK_BUTTON, TDM_SET_PROGRESS_BAR_MARQUEE, TDN_CREATED, TDN_TIMER,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{IDCANCEL, IDOK, SendMessageW};

use crate::lang::t;
use crate::win::{CREATE_NO_WINDOW, task_dialog, wide};

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
        return command
            .output()
            .context(t("executar a conversão", "running the conversion"));
    }

    let mut child = command
        .spawn()
        .context(t("iniciar a conversão", "starting the conversion"))?;
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
        bail!("{}", t("conversão cancelada", "conversion cancelled"));
    }
    let status = child
        .wait()
        .context(t("terminar a conversão", "finishing the conversion"))?;
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

    let (status, pressed) = task_dialog(&config);
    // status < 0 means comctl32 v6 is missing: the dialog never opened, so
    // the work must simply run to the end.
    status >= 0 && pressed == IDCANCEL
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

struct DoneState {
    done: Arc<AtomicBool>,
}

/// Runs in-process work (PDF rendering) behind the same marquee dialog.
/// The worker checks `cancel` between pages and stops early.
pub fn run_cancellable<T, F>(label: &str, work: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&AtomicBool) -> Result<T> + Send + 'static,
{
    if unsafe { !GetConsoleWindow().is_null() } {
        let flag = AtomicBool::new(false);
        return work(&flag);
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    let worker_cancel = Arc::clone(&cancel);
    let worker_done = Arc::clone(&done);
    let handle = std::thread::spawn(move || {
        let result = work(&worker_cancel);
        let _ = sender.send(result);
        worker_done.store(true, Ordering::Relaxed);
    });

    let cancelled = dialog_done(label, &done);
    if cancelled {
        cancel.store(true, Ordering::Relaxed);
        let _ = handle.join();
        bail!("{}", t("conversão cancelada", "conversion cancelled"));
    }
    let result = receiver.recv().unwrap_or_else(|_| {
        Err(anyhow::anyhow!(
            "{}",
            t("conversão falhou", "conversion failed")
        ))
    });
    let _ = handle.join();
    result
}

fn dialog_done(label: &str, done: &Arc<AtomicBool>) -> bool {
    let title = wide("SETTConverter");
    let text = wide(label);
    let state = Box::new(DoneState {
        done: Arc::clone(done),
    });
    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TDF_SHOW_PROGRESS_BAR | TDF_CALLBACK_TIMER | TDF_ALLOW_DIALOG_CANCELLATION,
        dwCommonButtons: TDCBF_CANCEL_BUTTON,
        pszWindowTitle: title.as_ptr(),
        pszMainInstruction: text.as_ptr(),
        pfCallback: Some(done_callback),
        lpCallbackData: Box::into_raw(state) as isize,
        ..Default::default()
    };

    let (status, pressed) = task_dialog(&config);
    unsafe {
        drop(Box::from_raw(config.lpCallbackData as *mut DoneState));
    }
    status >= 0 && pressed == IDCANCEL
}

unsafe extern "system" fn done_callback(
    hwnd: HWND,
    msg: u32,
    _wparam: usize,
    _lparam: isize,
    data: isize,
) -> i32 {
    unsafe {
        let state = &*(data as *const DoneState);
        if msg == TDN_CREATED as u32 {
            SendMessageW(hwnd, TDM_SET_PROGRESS_BAR_MARQUEE as u32, 1, 0);
        } else if msg == TDN_TIMER as u32 && state.done.load(Ordering::Relaxed) {
            SendMessageW(hwnd, TDM_CLICK_BUTTON as u32, IDOK as usize, 0);
        }
    }
    0
}
