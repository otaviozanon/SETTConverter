// UI language follows Windows: Portuguese (Brazil) or English.

use std::sync::OnceLock;
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Pt,
    En,
}

pub fn current() -> Lang {
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(|| {
        let id = unsafe { GetUserDefaultUILanguage() } as u32;
        // Primary language id 0x16 is Portuguese; everything else falls back to English.
        if id & 0x3ff == 0x16 {
            Lang::Pt
        } else {
            Lang::En
        }
    })
}

pub fn is_pt() -> bool {
    current() == Lang::Pt
}

/// Picks the string for the current Windows UI language.
pub fn t(pt: &'static str, en: &'static str) -> &'static str {
    if is_pt() { pt } else { en }
}
