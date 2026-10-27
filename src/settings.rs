use std::path::{Path, PathBuf};

/// Where Codex login files are read. The value is a choice, not a secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Windows,
    Wsl,
}

/// Which remaining number the big figure follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusPref {
    Auto,
    Session,
    Week,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemePref {
    System,
    Dark,
    Light,
}

/// `Keep` means the registry run key is left untouched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutostartPref {
    Keep,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutostartAction {
    Leave,
    Enable,
    Disable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub source: Source,
    pub distro: String,
    pub codex_home: String,
    pub focus: FocusPref,
    pub autostart: AutostartPref,
    pub theme: ThemePref,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            source: Source::Windows,
            distro: String::new(),
            codex_home: String::new(),
            focus: FocusPref::Auto,
            autostart: AutostartPref::Keep,
            theme: ThemePref::System,
        }
    }
}

pub fn data_dir(local_app_data: &Path) -> PathBuf {
    local_app_data.join("Jusage").join("data")
}

pub fn settings_path(local_app_data: &Path) -> PathBuf {
    local_app_data.join("Jusage").join("settings.txt")
}

pub fn parse_settings(text: &str) -> Settings {
    let mut settings = Settings::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "source" => {
                settings.source = if value.eq_ignore_ascii_case("wsl") {
                    Source::Wsl
                } else {
                    Source::Windows
                };
            }
            "distro" => {
                if let Some(name) = safe_distro(value) {
                    settings.distro = name;
                }
            }
            "codex_home" => {
                if let Some(path) = override_path(value) {
                    settings.codex_home = path;
                }
            }
            "focus" => {
                settings.focus = match value {
                    "session" => FocusPref::Session,
                    "week" => FocusPref::Week,
                    _ => FocusPref::Auto,
                };
            }
            "autostart" => {
                settings.autostart = match value {
                    "on" => AutostartPref::On,
                    "off" => AutostartPref::Off,
                    _ => AutostartPref::Keep,
                };
            }
            "theme" => {
                settings.theme = match value {
                    "dark" => ThemePref::Dark,
                    "light" => ThemePref::Light,
                    _ => ThemePref::System,
                };
            }
            _ => {}
        }
    }
    settings
}

pub fn format_settings(settings: &Settings) -> String {
    let source = match settings.source {
        Source::Windows => "windows",
        Source::Wsl => "wsl",
    };
    let focus = match settings.focus {
        FocusPref::Auto => "auto",
        FocusPref::Session => "session",
        FocusPref::Week => "week",
    };
    let autostart = match settings.autostart {
        AutostartPref::Keep => "keep",
        AutostartPref::On => "on",
        AutostartPref::Off => "off",
    };
    let theme = match settings.theme {
        ThemePref::System => "system",
        ThemePref::Dark => "dark",
        ThemePref::Light => "light",
    };
    let distro = safe_distro(&settings.distro).unwrap_or_default();
    let home = override_path(&settings.codex_home).unwrap_or_default();
    format!(
        "source={source}\ndistro={distro}\ncodex_home={home}\nfocus={focus}\nautostart={autostart}\ntheme={theme}\n"
    )
}

/// A new file records what is already true. It does not turn startup on by itself.
pub fn initial_autostart(run_key_exists: bool) -> AutostartPref {
    if run_key_exists {
        AutostartPref::On
    } else {
        AutostartPref::Off
    }
}

pub fn autostart_action(pref: AutostartPref) -> AutostartAction {
    match pref {
        AutostartPref::Keep => AutostartAction::Leave,
        AutostartPref::On => AutostartAction::Enable,
        AutostartPref::Off => AutostartAction::Disable,
    }
}

pub fn safe_distro(name: &str) -> Option<String> {
    let name = name.trim().trim_matches('\u{0}').trim();
    if name.is_empty() || name.len() > 64 {
        return None;
    }
    let ok = name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' || ch == ' ');
    if !ok {
        return None;
    }
    Some(name.to_string())
}

pub fn override_path(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.len() > 240 || raw.contains('\0') || raw.contains('\n') || raw.contains('\r') {
        return None;
    }
    if raw
        .split(['\\', '/'])
        .any(|part| part == ".." || part == ".")
    {
        return None;
    }
    Some(raw.to_string())
}

/// `/home/user/.codex` becomes `\\wsl$\Ubuntu\home\user\.codex`.
pub fn linux_to_unc(distro: &str, linux_path: &str) -> Option<String> {
    let distro = safe_distro(distro)?;
    let linux_path = linux_path.trim();
    if !linux_path.starts_with('/') || linux_path.contains('\0') || linux_path.contains('\\') {
        return None;
    }
    if linux_path
        .split('/')
        .any(|part| part == ".." || part == ".")
    {
        return None;
    }
    let rest = linux_path.trim_start_matches('/');
    if rest.is_empty() {
        return None;
    }
    Some(format!(
        r"\\wsl$\{distro}\{rest}",
        rest = rest.replace('/', "\\")
    ))
}

pub fn parse_wsl_list(bytes: &[u8]) -> Vec<String> {
    let text = decode_wsl_output(bytes);
    let mut names = Vec::new();
    for line in text.lines() {
        if let Some(name) = safe_distro(line) {
            if !names.iter().any(|have: &String| have == &name) {
                names.push(name);
            }
        }
    }
    names
}

fn decode_wsl_output(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return utf16_le(&bytes[2..]);
    }
    if bytes.len() >= 4 && bytes.len() % 2 == 0 {
        let zeros = bytes
            .iter()
            .skip(1)
            .step_by(2)
            .filter(|byte| **byte == 0)
            .count();
        if zeros * 2 > bytes.len() / 2 {
            return utf16_le(bytes);
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn utf16_le(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

pub fn fresh_line(age_secs: i64) -> String {
    if age_secs < 45 {
        return "방금 확인했어요".to_string();
    }
    let minutes = (age_secs / 60).max(1);
    if minutes < 10 {
        return format!("{minutes}분 전 확인");
    }
    if minutes < 60 {
        return format!("{minutes}분 전 확인 · 오래됐어요");
    }
    format!("{}시간 전 확인 · 오래됐어요", (minutes / 60).max(1))
}

/// Idle gap after repeated failures. The first wait is two minutes, then it grows, and it stops at five.
pub fn idle_backoff(fails: u32) -> u64 {
    let shift = fails.saturating_sub(1).min(2);
    (120u64 << shift).min(300)
}

pub fn fail_note(code: u16) -> &'static str {
    match code {
        0 => "Codex에 로그인한 뒤 다시 확인해 주세요.",
        1 => "네트워크에 연결하지 못했어요. 잠시 뒤 다시 확인해 주세요.",
        2 => "사용량 응답을 읽지 못했어요. 잠시 뒤 다시 확인해 주세요.",
        _ => "서버가 응답하지 않았어요. 잠시 뒤 다시 확인해 주세요.",
    }
}

pub fn source_label(source: Source) -> &'static str {
    match source {
        Source::Windows => "Windows",
        Source::Wsl => "WSL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_reject_unsafe_names() {
        let text = "source=wsl\ndistro=Ubuntu-22.04\ncodex_home=/home/user/.codex\nfocus=week\nautostart=off\ntheme=dark\n";
        let parsed = parse_settings(text);
        assert_eq!(parsed.source, Source::Wsl);
        assert_eq!(parsed.distro, "Ubuntu-22.04");
        assert_eq!(parsed.focus, FocusPref::Week);
        assert_eq!(parsed.autostart, AutostartPref::Off);
        assert_eq!(parsed.theme, ThemePref::Dark);
        let again = parse_settings(&format_settings(&parsed));
        assert_eq!(again, parsed);
        assert!(safe_distro("Ubuntu;calc").is_none());
        assert!(safe_distro("../etc").is_none());
        assert!(override_path("/home/../etc").is_none());
        assert_eq!(
            linux_to_unc("Ubuntu", "/home/user/.codex").as_deref(),
            Some(r"\\wsl$\Ubuntu\home\user\.codex")
        );
        assert!(linux_to_unc("Ubuntu", "/home/../.codex").is_none());
        assert!(linux_to_unc("Ubuntu;x", "/home/user/.codex").is_none());
    }

    #[test]
    fn wsl_names_accept_utf16_and_drop_junk() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "Ubuntu\nDebian\0\n".encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
        assert_eq!(parse_wsl_list(&bytes), ["Ubuntu", "Debian"]);
        assert_eq!(parse_wsl_list(b"Ubuntu\nnot a name!\n"), ["Ubuntu"]);
    }

    #[test]
    fn freshness_backoff_and_startup_do_not_invent_consent() {
        assert_eq!(fresh_line(10), "방금 확인했어요");
        assert_eq!(fresh_line(120), "2분 전 확인");
        assert_eq!(fresh_line(700), "11분 전 확인 · 오래됐어요");
        assert_eq!(idle_backoff(1), 120);
        assert_eq!(idle_backoff(2), 240);
        assert_eq!(idle_backoff(8), 300);
        assert_eq!(initial_autostart(true), AutostartPref::On);
        assert_eq!(initial_autostart(false), AutostartPref::Off);
        assert_eq!(
            autostart_action(AutostartPref::Keep),
            AutostartAction::Leave
        );
        assert_eq!(
            autostart_action(AutostartPref::Off),
            AutostartAction::Disable
        );
        assert_eq!(
            fail_note(1),
            "네트워크에 연결하지 못했어요. 잠시 뒤 다시 확인해 주세요."
        );
        assert!(data_dir(Path::new(r"C:\Users\user\AppData\Local")).ends_with("data"));
    }
}
