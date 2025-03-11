#![cfg(windows)]
extern crate alloc;

use detour::static_detour;
use std::ffi::{CString, c_void};
use std::{iter, mem};
use windows_sys::Win32::Foundation::{FARPROC, HINSTANCE, HWND};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::Win32::UI::WindowsAndMessaging::{HMENU, WINDOW_EX_STYLE, WINDOW_STYLE};
use windows_sys::core::PCSTR;

static_detour! {
    static HookCreateWindowExA: unsafe extern "system" fn(
        WINDOW_EX_STYLE,
        PCSTR,
        PCSTR,
        WINDOW_STYLE,
        i32,
        i32,
        i32,
        i32,
        HWND,
        HMENU,
        HINSTANCE,
        *const c_void
    ) -> HWND;
}

type FnCreateWindowExA = unsafe extern "system" fn(
    WINDOW_EX_STYLE,
    PCSTR,
    PCSTR,
    WINDOW_STYLE,
    i32,
    i32,
    i32,
    i32,
    HWND,
    HMENU,
    HINSTANCE,
    *const c_void,
) -> HWND;

fn main() {
    let address = get_module_symbol_address("user32.dll", "CreateWindowExA")
        .expect("could not find 'CreateWindowExW' address");

    let target: FnCreateWindowExA = unsafe { mem::transmute(address) };

    if let Ok(hook) = HookCreateWindowExA.initialize(target, create_window_ex_w_detour) {
        if let Err(e) = hook.enable() {
            panic!("{}", e);
        }
    }
}

/// Called whenever `MessageBoxW` is invoked in the process.
fn create_window_ex_w_detour(
    (
        dwexstyle,
        lpclassname,
        _lpwindowname,
        dwstyle,
        x,
        y,
        nwidth,
        nheight,
        hwndparent,
        hmenu,
        hinstance,
        lpparam,
    ): (
        WINDOW_EX_STYLE,
        PCSTR,
        PCSTR,
        WINDOW_STYLE,
        i32,
        i32,
        i32,
        i32,
        HWND,
        HMENU,
        HINSTANCE,
        *const c_void,
    ),
) -> HWND {
    // Call the original `MessageBoxW`, but replace the caption
    let replaced_caption = Vec::from("Detoured!\0");
    HookCreateWindowExA.call(
        dwexstyle,
        lpclassname,
        replaced_caption.as_ptr() as _,
        dwstyle,
        x,
        y,
        nwidth,
        nheight,
        hwndparent,
        hmenu,
        hinstance,
        lpparam,
    )
}

fn get_module_symbol_address(module: &str, symbol: &str) -> FARPROC {
    let module = module
        .encode_utf16()
        .chain(iter::once(0))
        .collect::<Vec<u16>>();
    let symbol = CString::new(symbol).unwrap();
    unsafe { GetProcAddress(GetModuleHandleW(module.as_ptr()), symbol.as_ptr() as PCSTR) }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _module: HINSTANCE,
    call_reason: u32,
    _reserved: *mut c_void,
) -> bool {
    if call_reason == DLL_PROCESS_ATTACH {
        main()
    }
    true
}
