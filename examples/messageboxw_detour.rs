#![cfg(windows)]
//! A `MessageBoxW` detour example.
//!
//! Ensure the crate is compiled as a 'cdylib' library to allow C interop.
use detour::static_detour;
use std::error::Error;
use std::ffi::c_void;
use std::{ffi::CString, iter, mem};
use windows_sys::Win32::Foundation::{FARPROC, HINSTANCE, HWND};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::Win32::UI::WindowsAndMessaging::{MESSAGEBOX_RESULT, MESSAGEBOX_STYLE};
use windows_sys::core::{PCSTR, PCWSTR};

static_detour! {
  static MessageBoxWHook: unsafe extern "system" fn(HWND, PCWSTR, PCWSTR, MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT;
}

// A type alias for `MessageBoxW` (makes the transmute easy on the eyes)
type FnMessageBoxW =
    unsafe extern "system" fn(HWND, PCWSTR, PCWSTR, MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT;

/// Called when the DLL is attached to the process.
fn main() -> Result<(), Box<dyn Error>> {
    // Retrieve an absolute address of `MessageBoxW`. This is required for
    // libraries due to the import address table. If `MessageBoxW` would be
    // provided directly as the target, it would only hook this DLL's
    // `MessageBoxW`. Using the method below an absolute address is retrieved
    // instead, detouring all invocations of `MessageBoxW` in the active process.
    let address = get_module_symbol_address("user32.dll", "CreateWindowExW")
        .expect("could not find 'MessageBoxW' address");
    let target: FnMessageBoxW = unsafe { mem::transmute(address) };

    // Initialize AND enable the detour (the 2nd parameter can also be a closure)
    unsafe {
        MessageBoxWHook
            .initialize(target, messageboxw_detour)?
            .enable()?;
    }
    Ok(())
}

/// Called whenever `MessageBoxW` is invoked in the process.
fn messageboxw_detour(
    (hwnd, lptext, _lpcaption, utype): (HWND, PCWSTR, PCWSTR, MESSAGEBOX_STYLE),
) -> MESSAGEBOX_RESULT {
    // Call the original `MessageBoxW`, but replace the caption
    let replaced_caption = "Detoured!\0".encode_utf16().collect::<Vec<u16>>();
    MessageBoxWHook.call(hwnd, lptext, replaced_caption.as_ptr() as _, utype)
}

/// Returns a module symbol's absolute address.
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
        // A console may be useful for printing to 'stdout'
        // winapi::um::consoleapi::AllocConsole();

        // Preferably a thread should be created here instead, since as few
        // operations as possible should be performed within `DllMain`.
        main().is_ok()
    } else {
        true
    }
}
