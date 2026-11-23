// Small Win32 helpers shared by the UI, progress dialogs and tools.

use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::WriteConsoleW;
use windows_sys::Win32::UI::Controls::{TASKDIALOGCONFIG, TaskDialogIndirect};
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONQUESTION, MB_OK, MB_YESNO, MessageBoxW};

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn message_box(title: &str, text: &str, flags: u32) -> i32 {
    let title = wide(title);
    let text = wide(text);
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            flags | MB_OK,
        )
    }
}

/// Yes/No question for the fallback dialog.
pub fn confirm(text: &str) -> i32 {
    message_box("SETTConverter", text, MB_YESNO | MB_ICONQUESTION)
}

/// Shows a TaskDialog; returns (status, pressed button). A negative status
/// means comctl32 v6 is unavailable, so nothing was displayed.
pub fn task_dialog(config: &TASKDIALOGCONFIG) -> (i32, i32) {
    let mut pressed = 0i32;
    let status = unsafe {
        TaskDialogIndirect(
            config,
            &mut pressed,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (status, pressed)
}

/// The process is a GUI app; writing to the parent console needs CONOUT$ because
/// the standard handles were born invalid.
pub fn console_out(text: &str) {
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
