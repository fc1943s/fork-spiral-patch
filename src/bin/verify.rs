use biturbo_spiral::lexer::Style;
use biturbo_spiral::pe::export_names;
use biturbo_spiral::{BtHighlightedDiff, BtRange, ORIGINAL_FILE_NAME, PROXY_MARKER_EXPORT};
use std::collections::BTreeSet;
use std::ffi::{CString, c_char, c_void};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
}

type HighlightSyntax =
    unsafe extern "C" fn(*const c_char, *const c_char, *const BtRange, i64, *mut BtHighlightedDiff) -> i32;
type ReleaseHighlightSyntax = unsafe extern "C" fn(*mut BtHighlightedDiff);

fn exports(path: &Path) -> Result<BTreeSet<String>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(export_names(&bytes).map_err(|error| format!("{}: {error}", path.display()))?.into_iter().collect())
}

fn highlight_count(
    highlight: HighlightSyntax,
    release: ReleaseHighlightSyntax,
    path: &str,
    text: &str,
) -> Result<(i64, u8), String> {
    let path = CString::new(path).map_err(|error| error.to_string())?;
    let text_c = CString::new(text).map_err(|error| error.to_string())?;
    let ranges = [BtRange { start: 0, end: text.encode_utf16().count() as u32 }];
    let mut result = BtHighlightedDiff { items: std::ptr::null_mut(), items_len: 0, items_cap: 0 };
    let code = unsafe { highlight(path.as_ptr(), text_c.as_ptr(), ranges.as_ptr(), 1, &mut result) };
    if code != 0 {
        return Err(format!("bt_highlight_syntax returned {code}"));
    }
    let first_style = if result.items.is_null() { u8::MAX } else { unsafe { (*result.items).style } };
    let count = result.items_len;
    unsafe { release(&mut result) };
    Ok((count, first_style))
}

fn run(fork_dir: &Path) -> Result<(), String> {
    let proxy_path = fork_dir.join("biturbo.dll");
    let original = exports(&fork_dir.join(ORIGINAL_FILE_NAME))?;
    let proxy = exports(&proxy_path)?;
    if !proxy.contains(PROXY_MARKER_EXPORT) {
        return Err(format!("{} is not the Spiral proxy", proxy_path.display()));
    }
    let missing: Vec<_> = original.difference(&proxy).collect();
    if !missing.is_empty() {
        return Err(format!("proxy lacks {} original exports: {missing:?}", missing.len()));
    }
    println!("exports: proxy forwards all {} original exports", original.len());

    let wide: Vec<u16> = proxy_path.as_os_str().encode_wide().chain([0]).collect();
    let module = unsafe { LoadLibraryW(wide.as_ptr()) };
    if module.is_null() {
        return Err(format!("LoadLibrary failed for {}", proxy_path.display()));
    }
    let highlight = unsafe { GetProcAddress(module, c"bt_highlight_syntax".as_ptr()) };
    let release = unsafe { GetProcAddress(module, c"bt_release_highlight_syntax".as_ptr()) };
    if highlight.is_null() || release.is_null() {
        return Err("highlight exports missing".into());
    }
    let highlight: HighlightSyntax = unsafe { std::mem::transmute(highlight) };
    let release: ReleaseHighlightSyntax = unsafe { std::mem::transmute(release) };
    let (rust_count, _) = highlight_count(highlight, release, "src/main.rs", "fn main() { let x = 1; }")?;
    let (spiral_count, spiral_style) = highlight_count(highlight, release, "lib/main.spi", "inl f x = x + 1i32")?;
    if rust_count == 0 || spiral_count == 0 || spiral_style != Style::Keyword as u8 {
        return Err(format!(
            "highlighting: rust {rust_count} spans, spiral {spiral_count} spans (first style {spiral_style})"
        ));
    }
    println!("highlighting: rust forwarded ({rust_count} spans), spiral handled ({spiral_count} spans)");
    Ok(())
}

fn main() {
    let Some(fork_dir) = std::env::args().nth(1) else {
        eprintln!("usage: verify <fork-dir>");
        std::process::exit(2);
    };
    if let Err(error) = run(Path::new(&fork_dir)) {
        eprintln!("verify failed: {error}");
        std::process::exit(1);
    }
}
