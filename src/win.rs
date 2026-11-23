// Small Win32 helpers shared by the UI, progress dialogs and tools.

use windows_sys::Win32::UI::Controls::{TASKDIALOGCONFIG, TaskDialogIndirect};
use windows_sys::Win32::UI::WindowsAndMessaging::{MB_OK, MessageBoxW};

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
