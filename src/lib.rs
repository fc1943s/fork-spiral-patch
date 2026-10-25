pub mod highlight;
pub mod lexer;
pub mod pe;
pub mod theme;

use std::collections::HashSet;
use std::ffi::{c_char, c_void, CStr};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BtRange {
    pub start: u32,
    pub end: u32,
}

#[repr(C)]
pub struct BtHighlightedRange {
    pub range_utf16: BtRange,
    pub style: u8,
}

#[repr(C)]
pub struct BtHighlightedDiff {
    pub items: *mut BtHighlightedRange,
    pub items_len: i64,
    pub items_cap: i64,
}

#[repr(i32)]
enum BtResult {
    Ok = 0,
    ErrNotFound = 3,
}

type HighlightSyntax =
    unsafe extern "C" fn(*const c_char, *const c_char, *const BtRange, i64, *mut BtHighlightedDiff) -> i32;
type ReleaseHighlightSyntax = unsafe extern "C" fn(*mut BtHighlightedDiff);

struct Biturbo {
    highlight_syntax: Option<HighlightSyntax>,
    release_highlight_syntax: Option<ReleaseHighlightSyntax>,
}

const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x2;
const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x4;
pub const ORIGINAL_FILE_NAME: &str = "biturbo_original.dll";
pub const PROXY_MARKER_EXPORT: &str = "biturbo_spiral_proxy";

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleExW(flags: u32, name: *const u16, module: *mut *mut c_void) -> i32;
    fn GetModuleFileNameW(module: *mut c_void, name: *mut u16, size: u32) -> u32;
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
}

static OWNED_ITEMS: Mutex<Option<HashSet<usize>>> = Mutex::new(None);

fn own(items: *mut BtHighlightedRange) {
    let mut owned = OWNED_ITEMS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    owned.get_or_insert_with(HashSet::new).insert(items as usize);
}

fn disown(items: *mut BtHighlightedRange) -> bool {
    let mut owned = OWNED_ITEMS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    owned.as_mut().is_some_and(|owned| owned.remove(&(items as usize)))
}

fn own_directory() -> Option<PathBuf> {
    let mut module = std::ptr::null_mut();
    let address = own_directory as *const () as *const u16;
    let flags = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
    if unsafe { GetModuleHandleExW(flags, address, &mut module) } == 0 {
        return None;
    }
    let mut buffer = vec![0u16; 32768];
    let length = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    (length > 0).then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..length])).parent().map(PathBuf::from))?
}

fn biturbo() -> &'static Biturbo {
    static BITURBO: OnceLock<Biturbo> = OnceLock::new();
    BITURBO.get_or_init(|| {
        let path =
            own_directory().map_or_else(|| PathBuf::from(ORIGINAL_FILE_NAME), |dir| dir.join(ORIGINAL_FILE_NAME));
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let module = unsafe { LoadLibraryW(wide.as_ptr()) };
        let export = |name: &CStr| {
            if module.is_null() {
                std::ptr::null_mut()
            } else {
                unsafe { GetProcAddress(module, name.as_ptr()) }
            }
        };
        let highlight = export(c"bt_highlight_syntax");
        let release = export(c"bt_release_highlight_syntax");
        Biturbo {
            highlight_syntax: (!highlight.is_null())
                .then(|| unsafe { std::mem::transmute::<*mut c_void, HighlightSyntax>(highlight) }),
            release_highlight_syntax: (!release.is_null())
                .then(|| unsafe { std::mem::transmute::<*mut c_void, ReleaseHighlightSyntax>(release) }),
        }
    })
}

unsafe fn spiral_highlight(diff: *const c_char, ranges: *const BtRange, ranges_len: i64) -> Vec<BtHighlightedRange> {
    let text = unsafe { CStr::from_ptr(diff) }.to_string_lossy();
    let ranges: Vec<(u32, u32)> = if ranges.is_null() || ranges_len <= 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(ranges, ranges_len as usize) }.iter().map(|r| (r.start, r.end)).collect()
    };
    let theme = theme::current_theme();
    highlight::highlight(&text, &ranges)
        .into_iter()
        .map(|span| BtHighlightedRange {
            range_utf16: BtRange { start: span.start, end: span.end },
            style: theme::themed(span.style, theme) as u8,
        })
        .collect()
}

#[unsafe(no_mangle)]
pub extern "C" fn biturbo_spiral_proxy() -> u32 {
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bt_highlight_syntax(
    file_path: *const c_char,
    diff: *const c_char,
    ranges: *const BtRange,
    ranges_len: i64,
    out_result: *mut BtHighlightedDiff,
) -> i32 {
    let is_spiral = !file_path.is_null()
        && highlight::is_spiral_path(&unsafe { CStr::from_ptr(file_path) }.to_string_lossy());
    if !is_spiral {
        return match biturbo().highlight_syntax {
            Some(forward) => unsafe { forward(file_path, diff, ranges, ranges_len, out_result) },
            None => BtResult::ErrNotFound as i32,
        };
    }
    if diff.is_null() || out_result.is_null() {
        return BtResult::ErrNotFound as i32;
    }
    let Ok(items) = std::panic::catch_unwind(|| unsafe { spiral_highlight(diff, ranges, ranges_len) }) else {
        return BtResult::ErrNotFound as i32;
    };
    let mut items = std::mem::ManuallyDrop::new(items);
    let result = if items.is_empty() {
        BtHighlightedDiff { items: std::ptr::null_mut(), items_len: 0, items_cap: 0 }
    } else {
        own(items.as_mut_ptr());
        BtHighlightedDiff {
            items: items.as_mut_ptr(),
            items_len: items.len() as i64,
            items_cap: items.capacity() as i64,
        }
    };
    unsafe { out_result.write(result) };
    BtResult::Ok as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bt_release_highlight_syntax(result: *mut BtHighlightedDiff) {
    if result.is_null() {
        return;
    }
    let diff = unsafe { &mut *result };
    if diff.items.is_null() {
        return;
    }
    if disown(diff.items) {
        drop(unsafe { Vec::from_raw_parts(diff.items, diff.items_len as usize, diff.items_cap as usize) });
        *diff = BtHighlightedDiff { items: std::ptr::null_mut(), items_len: 0, items_cap: 0 };
    } else if let Some(forward) = biturbo().release_highlight_syntax {
        unsafe { forward(result) };
    }
}
