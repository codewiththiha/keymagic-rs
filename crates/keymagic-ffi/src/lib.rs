//! C ABI for the engine, KM2 files and the KMS compiler.
//!
//! Exports take raw pointers and are `unsafe` to call. Pointers must be null
//! or valid for the stated length. A handle serves one thread at a time.
//! Free strings and byte buffers with their `keymagic_free_*` calls.

#![expect(
    clippy::missing_safety_doc,
    reason = "the pointer contract is stated once in the crate docs"
)]

use keymagic_engine::{Action, Engine, KeyEvent, Modifiers, Output};
use keymagic_kms::{Diagnostic, Resolver, Severity};
use keymagic_types::{Hotkey, Keyboard, KeyboardInfo, VirtualKey};
use std::ffi::{CStr, CString, c_char, c_int};
use std::fmt::Write as _;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;
use std::str;

/// Status codes; values match the KM3 `libkeymagic_core` header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMagicResult {
    Success = 0,
    ErrorInvalidParameter = -2,
    ErrorEngineFailure = -3,
    ErrorUtf8Conversion = -4,
    ErrorNoKeyboard = -5,
    ErrorCompileFailed = -6,
    ErrorPanic = -7,
}

/// Result of one key event. Free both strings with `keymagic_free_string`.
#[repr(C)]
pub struct ProcessKeyOutput {
    /// 0 none, 1 insert, 2 delete, 3 delete then insert.
    pub action_type: c_int,
    /// Text to insert, or null.
    pub text: *mut c_char,
    /// Characters to delete before the cursor.
    pub delete_count: c_int,
    /// Composing text after the event.
    pub composing_text: *mut c_char,
    /// 1 when the engine consumed the key, otherwise 0.
    pub is_processed: c_int,
}

/// Engine handle; free with `keymagic_engine_free`.
pub struct EngineHandle {
    engine: Option<Engine>,
}

/// Parsed KM2 file; free with `keymagic_km2_free`.
pub struct Km2FileHandle {
    keyboard: Keyboard,
}

struct NoIncludes;

impl Resolver for NoIncludes {
    fn read(&self, _path: &str) -> Option<Vec<u8>> {
        None
    }
}

fn guarded<T>(fallback: T, body: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(fallback)
}

fn to_c_string(text: &str) -> *mut c_char {
    CString::new(text).map_or(ptr::null_mut(), CString::into_raw)
}

#[unsafe(no_mangle)]
pub extern "C" fn keymagic_engine_new() -> *mut EngineHandle {
    Box::into_raw(Box::new(EngineHandle { engine: None }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_free(handle: *mut EngineHandle) {
    if !handle.is_null() {
        // SAFETY: the caller frees each handle once.
        drop(unsafe { Box::from_raw(handle) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_load_keyboard(
    handle: *mut EngineHandle,
    km2_path: *const c_char,
) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        // SAFETY: handle and km2_path are null or valid per the crate contract.
        let engine = unsafe { handle.as_mut() };
        if km2_path.is_null() {
            return KeyMagicResult::ErrorInvalidParameter;
        }
        let Some(engine) = engine else {
            return KeyMagicResult::ErrorInvalidParameter;
        };
        // SAFETY: km2_path is non-null and NUL-terminated.
        let Ok(path) = unsafe { CStr::from_ptr(km2_path) }.to_str() else {
            return KeyMagicResult::ErrorUtf8Conversion;
        };
        fs::read(path).map_or(KeyMagicResult::ErrorNoKeyboard, |bytes| {
            install(engine, &bytes)
        })
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_load_keyboard_from_memory(
    handle: *mut EngineHandle,
    km2_data: *const u8,
    data_len: usize,
) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        // SAFETY: handle is null or valid per the crate contract.
        let engine = unsafe { handle.as_mut() };
        let Some(engine) = engine else {
            return KeyMagicResult::ErrorInvalidParameter;
        };
        if km2_data.is_null() || data_len == 0 {
            return KeyMagicResult::ErrorInvalidParameter;
        }
        // SAFETY: km2_data is valid for data_len bytes per the crate contract.
        let bytes = unsafe { slice::from_raw_parts(km2_data, data_len) };
        install(engine, bytes)
    })
}

fn install(handle: &mut EngineHandle, bytes: &[u8]) -> KeyMagicResult {
    let Ok(keyboard) = keymagic_km2::parse(bytes) else {
        return KeyMagicResult::ErrorEngineFailure;
    };
    match Engine::new(std::sync::Arc::new(keyboard)) {
        Ok(engine) => {
            handle.engine = Some(engine);
            KeyMagicResult::Success
        }
        Err(_) => KeyMagicResult::ErrorEngineFailure,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_process_key(
    handle: *mut EngineHandle,
    key_code: c_int,
    character: c_char,
    shift: c_int,
    ctrl: c_int,
    alt: c_int,
    caps_lock: c_int,
    output: *mut ProcessKeyOutput,
) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        // SAFETY: handle and output are null or valid per the crate contract.
        let engine = unsafe { handle.as_mut() };
        let output = unsafe { output.as_mut() };
        let event = key_event(
            key_from_code(key_code),
            character,
            [shift, ctrl, alt, caps_lock],
        );
        run_key(engine, output, event, Step::Commit)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_process_key_test(
    handle: *mut EngineHandle,
    key_code: c_int,
    character: c_char,
    shift: c_int,
    ctrl: c_int,
    alt: c_int,
    caps_lock: c_int,
    output: *mut ProcessKeyOutput,
) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        // SAFETY: handle and output are null or valid per the crate contract.
        let engine = unsafe { handle.as_mut() };
        let output = unsafe { output.as_mut() };
        let event = key_event(
            key_from_code(key_code),
            character,
            [shift, ctrl, alt, caps_lock],
        );
        run_key(engine, output, event, Step::Peek)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_process_key_win(
    handle: *mut EngineHandle,
    vk_code: c_int,
    character: c_char,
    shift: c_int,
    ctrl: c_int,
    alt: c_int,
    caps_lock: c_int,
    output: *mut ProcessKeyOutput,
) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        // SAFETY: handle and output are null or valid per the crate contract.
        let engine = unsafe { handle.as_mut() };
        let output = unsafe { output.as_mut() };
        win_key(vk_code).map_or(KeyMagicResult::ErrorInvalidParameter, |key| {
            let flags = [shift, ctrl, alt, caps_lock];
            let event = key_event(Some(key), character, flags);
            run_key(engine, output, event, Step::Commit)
        })
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_process_key_test_win(
    handle: *mut EngineHandle,
    vk_code: c_int,
    character: c_char,
    shift: c_int,
    ctrl: c_int,
    alt: c_int,
    caps_lock: c_int,
    output: *mut ProcessKeyOutput,
) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        // SAFETY: handle and output are null or valid per the crate contract.
        let engine = unsafe { handle.as_mut() };
        let output = unsafe { output.as_mut() };
        win_key(vk_code).map_or(KeyMagicResult::ErrorInvalidParameter, |key| {
            let flags = [shift, ctrl, alt, caps_lock];
            let event = key_event(Some(key), character, flags);
            run_key(engine, output, event, Step::Peek)
        })
    })
}

#[derive(Clone, Copy)]
enum Step {
    Commit,
    Peek,
}

fn key_from_code(key_code: c_int) -> Option<VirtualKey> {
    u16::try_from(key_code).ok().and_then(VirtualKey::from_u16)
}

fn win_key(vk_code: c_int) -> Option<VirtualKey> {
    u16::try_from(vk_code)
        .ok()
        .and_then(keymagic_types::win_vk::from_code)
}

fn key_event(key: Option<VirtualKey>, character: c_char, flags: [c_int; 4]) -> KeyEvent {
    let byte = u8::try_from(i32::from(character) & 0xff).ok();
    KeyEvent {
        key,
        ch: byte.filter(|byte| *byte != 0).map(char::from),
        mods: Modifiers {
            shift: flags[0] != 0,
            ctrl: flags[1] != 0,
            alt: flags[2] != 0,
            caps_lock: flags[3] != 0,
        },
    }
}

fn run_key(
    handle: Option<&mut EngineHandle>,
    output: Option<&mut ProcessKeyOutput>,
    event: KeyEvent,
    step: Step,
) -> KeyMagicResult {
    let Some(output) = output else {
        return KeyMagicResult::ErrorInvalidParameter;
    };
    let Some(handle) = handle else {
        return KeyMagicResult::ErrorInvalidParameter;
    };
    let Some(engine) = handle.engine.as_mut() else {
        return KeyMagicResult::ErrorNoKeyboard;
    };
    let result = match step {
        Step::Commit => engine.process(event),
        Step::Peek => engine.peek(event),
    };
    fill(output, &result);
    KeyMagicResult::Success
}

fn fill(output: &mut ProcessKeyOutput, result: &Output) {
    let (action_type, text, delete) = match &result.action {
        Action::None => (0, None, 0),
        Action::Insert(text) => (1, Some(text.as_str()), 0),
        Action::Delete(count) => (2, None, *count),
        Action::Replace { delete, insert } => (3, Some(insert.as_str()), *delete),
    };
    output.action_type = action_type;
    output.text = text.map_or(ptr::null_mut(), to_c_string);
    output.delete_count = c_int::try_from(delete).unwrap_or(c_int::MAX);
    output.composing_text = to_c_string(&result.composing);
    output.is_processed = c_int::from(result.consumed);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_reset(handle: *mut EngineHandle) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        // SAFETY: handle is null or valid per the crate contract.
        let Some(handle) = (unsafe { handle.as_mut() }) else {
            return KeyMagicResult::ErrorInvalidParameter;
        };
        handle
            .engine
            .as_mut()
            .map_or(KeyMagicResult::ErrorNoKeyboard, |engine| {
                engine.reset();
                KeyMagicResult::Success
            })
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_get_composition(handle: *mut EngineHandle) -> *mut c_char {
    guarded(ptr::null_mut(), || {
        // SAFETY: handle is null or valid per the crate contract.
        let handle = unsafe { handle.as_ref() };
        handle
            .and_then(|handle| handle.engine.as_ref())
            .map_or(ptr::null_mut(), |engine| to_c_string(engine.composing()))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_set_composition(
    handle: *mut EngineHandle,
    text: *const c_char,
) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        // SAFETY: handle is null or valid per the crate contract.
        let Some(handle) = (unsafe { handle.as_mut() }) else {
            return KeyMagicResult::ErrorInvalidParameter;
        };
        if text.is_null() {
            return KeyMagicResult::ErrorInvalidParameter;
        }
        // SAFETY: text is non-null and NUL-terminated.
        let Ok(text) = unsafe { CStr::from_ptr(text) }.to_str() else {
            return KeyMagicResult::ErrorUtf8Conversion;
        };
        handle
            .engine
            .as_mut()
            .map_or(KeyMagicResult::ErrorNoKeyboard, |engine| {
                engine.set_composing(text);
                KeyMagicResult::Success
            })
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_free_string(text: *mut c_char) {
    if !text.is_null() {
        // SAFETY: text came from to_c_string and is freed once.
        drop(unsafe { CString::from_raw(text) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_get_composing_text(
    handle: *mut EngineHandle,
) -> *mut c_char {
    // SAFETY: handle is null or valid per the crate contract.
    unsafe { keymagic_engine_get_composition(handle) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_engine_free_string(text: *mut c_char) {
    // SAFETY: text came from this library per the crate contract.
    unsafe { keymagic_free_string(text) };
}

#[unsafe(no_mangle)]
pub extern "C" fn keymagic_virtual_key_to_string(key_code: c_int) -> *mut c_char {
    key_from_code(key_code).map_or(ptr::null_mut(), |key| to_c_string(key.kms_name()))
}

/// Parsed hotkey; `key_code` is the `VirtualKey` ID.
#[repr(C)]
pub struct HotkeyInfo {
    pub key_code: c_int,
    pub ctrl: c_int,
    pub alt: c_int,
    pub shift: c_int,
    pub meta: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_parse_hotkey(
    hotkey: *const c_char,
    info: *mut HotkeyInfo,
) -> c_int {
    guarded(0, || {
        if hotkey.is_null() || info.is_null() {
            return 0;
        }
        // SAFETY: hotkey is non-null and NUL-terminated.
        let Ok(text) = unsafe { CStr::from_ptr(hotkey) }.to_str() else {
            return 0;
        };
        let Ok(parsed) = text.parse::<Hotkey>() else {
            return 0;
        };
        // SAFETY: info is non-null and writable per the crate contract.
        let Some(info) = (unsafe { info.as_mut() }) else {
            return 0;
        };
        info.key_code = c_int::from(parsed.key as u16);
        info.ctrl = c_int::from(parsed.ctrl);
        info.alt = c_int::from(parsed.alt);
        info.shift = c_int::from(parsed.shift);
        info.meta = c_int::from(parsed.meta);
        1
    })
}

static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[unsafe(no_mangle)]
pub extern "C" fn keymagic_get_version() -> *const c_char {
    VERSION.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_km2_load(path: *const c_char) -> *mut Km2FileHandle {
    guarded(ptr::null_mut(), || {
        if path.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: path is non-null and NUL-terminated.
        let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else {
            return ptr::null_mut();
        };
        let Ok(bytes) = fs::read(path) else {
            return ptr::null_mut();
        };
        keymagic_km2::parse(&bytes).map_or(ptr::null_mut(), |keyboard| {
            Box::into_raw(Box::new(Km2FileHandle { keyboard }))
        })
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_km2_free(handle: *mut Km2FileHandle) {
    if !handle.is_null() {
        // SAFETY: the caller frees each handle once.
        drop(unsafe { Box::from_raw(handle) });
    }
}

fn info_text(handle: *mut Km2FileHandle, pick: fn(&KeyboardInfo) -> Option<&str>) -> *mut c_char {
    guarded(ptr::null_mut(), || {
        // SAFETY: handle is null or valid per the crate contract.
        let handle = unsafe { handle.as_ref() };
        handle
            .and_then(|handle| pick(&handle.keyboard.info))
            .map_or(ptr::null_mut(), to_c_string)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_km2_get_name(handle: *mut Km2FileHandle) -> *mut c_char {
    info_text(handle, KeyboardInfo::name)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_km2_get_description(handle: *mut Km2FileHandle) -> *mut c_char {
    info_text(handle, KeyboardInfo::description)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_km2_get_hotkey(handle: *mut Km2FileHandle) -> *mut c_char {
    info_text(handle, KeyboardInfo::hotkey)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_km2_get_icon_data(
    handle: *mut Km2FileHandle,
    buffer: *mut u8,
    buffer_size: usize,
) -> usize {
    guarded(0, || {
        // SAFETY: handle is null or valid per the crate contract.
        let Some(handle) = (unsafe { handle.as_ref() }) else {
            return 0;
        };
        let Some(icon) = handle.keyboard.info.icon() else {
            return 0;
        };
        if !buffer.is_null() && buffer_size >= icon.len() {
            // SAFETY: buffer is valid for buffer_size bytes per the contract.
            unsafe { ptr::copy_nonoverlapping(icon.as_ptr(), buffer, icon.len()) };
        }
        icon.len()
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_kms_compile(
    src: *const u8,
    len: usize,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
    out_diag_json: *mut *mut c_char,
) -> KeyMagicResult {
    guarded(KeyMagicResult::ErrorPanic, || {
        if src.is_null() || out_bytes.is_null() || out_len.is_null() {
            return KeyMagicResult::ErrorInvalidParameter;
        }
        // SAFETY: src is valid for len bytes per the crate contract.
        let source = unsafe { slice::from_raw_parts(src, len) };
        let Ok(text) = str::from_utf8(source) else {
            return KeyMagicResult::ErrorUtf8Conversion;
        };
        // SAFETY: the out pointers are non-null and writable.
        unsafe {
            out_bytes.write(ptr::null_mut());
            out_len.write(0);
        }
        let outcome = keymagic_kms::compile(text, &NoIncludes)
            .and_then(|keyboard| keymagic_km2::write(&keyboard).map_err(|_| Vec::new()));
        match outcome {
            Ok(bytes) => {
                let boxed = bytes.into_boxed_slice();
                let length = boxed.len();
                let raw = Box::into_raw(boxed).cast::<u8>();
                // SAFETY: the out pointers are non-null and writable.
                unsafe {
                    out_bytes.write(raw);
                    out_len.write(length);
                    if !out_diag_json.is_null() {
                        out_diag_json.write(ptr::null_mut());
                    }
                }
                KeyMagicResult::Success
            }
            Err(diagnostics) => {
                if !out_diag_json.is_null() {
                    // SAFETY: out_diag_json is non-null and writable.
                    unsafe { out_diag_json.write(to_c_string(&diagnostics_json(&diagnostics))) };
                }
                KeyMagicResult::ErrorCompileFailed
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn keymagic_free_bytes(data: *mut u8, len: usize) {
    if !data.is_null() {
        // SAFETY: data and len come from keymagic_kms_compile, freed once.
        drop(unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(data, len)) });
    }
}

fn diagnostics_json(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("[");
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let _ = write!(
            out,
            "{{\"severity\":\"{severity}\",\"start\":{},\"end\":{},\"message\":\"",
            diagnostic.span.start, diagnostic.span.end
        );
        push_escaped(&diagnostic.message, &mut out);
        out.push_str("\"}");
    }
    out.push(']');
    out
}

fn push_escaped(text: &str, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(control));
            }
            other => out.push(other),
        }
    }
}
