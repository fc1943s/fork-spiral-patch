use crate::lexer::Style;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

const FORK_SETTINGS: &str = "Fork/settings.json";
const FORK_DARK_THEME: &str = "1";
const HKEY_CURRENT_USER: isize = 0x8000_0001u32 as i32 as isize;
const RRF_RT_REG_DWORD: u32 = 0x10;
const PERSONALIZE_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const APPS_USE_LIGHT_THEME: &str = "AppsUseLightTheme";

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegGetValueW(
        key: isize,
        sub_key: *const u16,
        value: *const u16,
        flags: u32,
        kind: *mut u32,
        data: *mut c_void,
        size: *mut u32,
    ) -> i32;
}

fn json_value<'a>(settings: &'a str, key: &str) -> Option<&'a str> {
    let after_key = &settings[settings.find(&format!("\"{key}\""))? + key.len() + 2..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}', '\n', '\r']).unwrap_or(after_colon.len());
    Some(after_colon[..end].trim())
}

pub fn theme_from_settings(settings: Option<&str>, system: impl FnOnce() -> Theme) -> Theme {
    let Some(settings) = settings else { return system() };
    match (json_value(settings, "FollowSystemTheme"), json_value(settings, "Theme")) {
        (Some("false"), Some(FORK_DARK_THEME)) => Theme::Dark,
        (Some("false"), Some(_)) => Theme::Light,
        _ => system(),
    }
}

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text).encode_wide().chain([0]).collect()
}

pub fn system_theme() -> Theme {
    let (key, value) = (wide(PERSONALIZE_KEY), wide(APPS_USE_LIGHT_THEME));
    let mut light = 1u32;
    let mut size = size_of::<u32>() as u32;
    let data = (&raw mut light).cast::<c_void>();
    let (key, value, kind) = (key.as_ptr(), value.as_ptr(), std::ptr::null_mut());
    let status = unsafe { RegGetValueW(HKEY_CURRENT_USER, key, value, RRF_RT_REG_DWORD, kind, data, &mut size) };
    if status == 0 && light == 0 { Theme::Dark } else { Theme::Light }
}

pub fn current_theme() -> Theme {
    let settings_path = std::env::var_os("LOCALAPPDATA").map(|local| Path::new(&local).join(FORK_SETTINGS));
    let settings = settings_path.and_then(|path| std::fs::read_to_string(path).ok());
    theme_from_settings(settings.as_deref(), system_theme)
}

pub fn themed(style: Style, theme: Theme) -> Style {
    match (theme, style) {
        (Theme::Dark, Style::Keyword) => Style::Value,
        _ => style,
    }
}
