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

pub fn profile_dir(local_app_data: &Path, isolated: bool) -> PathBuf {
    local_app_data.join(if isolated { "JusageIsolated" } else { "Jusage" })
}

pub fn data_dir(local_app_data: &Path) -> PathBuf {
    profile_dir(local_app_data, false).join("data")
}

pub fn settings_path(local_app_data: &Path) -> PathBuf {
    profile_dir(local_app_data, false).join("settings.txt")
}

/// What a launch is allowed to change. The product path still copies home and may touch startup.
/// An isolated launch keeps a different profile, mutex, and window, and it does not write the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartupGuard {
    pub copy_home: bool,
    pub touch_registry: bool,
    pub mutex: &'static str,
    pub window_class: &'static str,
    pub profile: &'static str,
}

pub fn startup_guard(isolated: bool) -> StartupGuard {
    if isolated {
        StartupGuard {
            copy_home: false,
            touch_registry: false,
            mutex: r"Local\JusageIsolated",
            window_class: "JusageIsolatedWindow",
            profile: "JusageIsolated",
        }
    } else {
        StartupGuard {
            copy_home: true,
            touch_registry: true,
            mutex: r"Local\JusageSingle",
            window_class: "JusageWindow",
            profile: "Jusage",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    pub isolated: bool,
    pub preview_dir: Option<PathBuf>,
    pub preview_missing: bool,
}

pub fn parse_launch(args: impl IntoIterator<Item = impl AsRef<str>>) -> Launch {
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_string())
        .collect();
    let mut isolated = false;
    let mut preview_dir = None;
    let mut preview_missing = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--isolated" => isolated = true,
            "--preview" => {
                isolated = true;
                match args.get(index + 1) {
                    Some(next) if !next.is_empty() && !next.starts_with('-') => {
                        preview_dir = Some(PathBuf::from(next));
                        index += 1;
                    }
                    _ => preview_missing = true,
                }
            }
            _ => {}
        }
        index += 1;
    }
    Launch {
        isolated,
        preview_dir,
        preview_missing,
    }
}

/// A preview folder must not be the installed profile. `JusageIsolated` is a different name.
/// The path is made absolute on paper first, so `..`, a trailing dot, and `Jusage\data\..` cannot sneak through.
/// A relative path stays refused here. Windows resolves it with `GetFullPathName` before this check.
pub fn isolates_from_product(path: &Path) -> bool {
    match windows_absolute(path) {
        Some(full) => !path_has_component(&full, "Jusage"),
        None => false,
    }
}

/// `true` when `path` is the installed profile or a folder inside it.
pub fn path_is_within(path: &str, root: &str) -> bool {
    let Some(path) = windows_absolute(Path::new(path)) else {
        return false;
    };
    let Some(root) = windows_absolute(Path::new(root)) else {
        return false;
    };
    let path = path.to_ascii_lowercase();
    let root = root.trim_end_matches('\\').to_ascii_lowercase();
    path == root || path.starts_with(&(root + "\\"))
}

/// Absolute Windows path with `.`, `..`, and trailing dots or spaces removed.
/// `None` means the path still depends on the current folder (`..`, `name`, or `C:name`).
pub fn windows_absolute(path: &Path) -> Option<String> {
    let text = path.to_string_lossy().replace('/', "\\");
    let text = strip_extended_prefix(&text);
    let (root, rest) = split_windows_root(&text)?;
    let mut parts = Vec::new();
    for raw in rest.split('\\') {
        if raw.is_empty() || raw == "." {
            continue;
        }
        if raw == ".." {
            parts.pop();
            continue;
        }
        let cleaned = trim_win32_tail(raw);
        if cleaned.is_empty() || cleaned == "." {
            continue;
        }
        if cleaned == ".." {
            parts.pop();
            continue;
        }
        parts.push(cleaned);
    }
    let mut full = root;
    for part in parts {
        if !full.ends_with('\\') {
            full.push('\\');
        }
        full.push_str(&part);
    }
    Some(full)
}

fn path_has_component(path: &str, name: &str) -> bool {
    path.split(['\\', '/'])
        .any(|part| part.eq_ignore_ascii_case(name))
}

fn strip_extended_prefix(text: &str) -> String {
    if text.len() >= 8 {
        let head = &text[..8];
        if head.eq_ignore_ascii_case(r"\\?\UNC\") || head.eq_ignore_ascii_case(r"\\.\UNC\") {
            return format!(r"\\{}", &text[8..]);
        }
    }
    if text.len() >= 4 {
        let head = &text[..4];
        if head.eq_ignore_ascii_case(r"\\?\") || head.eq_ignore_ascii_case(r"\\.\") {
            return text[4..].to_string();
        }
    }
    text.to_string()
}

fn split_windows_root(text: &str) -> Option<(String, &str)> {
    if let Some(rest) = text.strip_prefix(r"\\") {
        let mut parts = rest.splitn(3, '\\');
        let server = parts.next().unwrap_or("");
        let share = parts.next().unwrap_or("");
        if server.is_empty() || share.is_empty() {
            return None;
        }
        let tail = parts.next().unwrap_or("");
        return Some((format!(r"\\{server}\{share}"), tail));
    }
    let mut chars = text.chars();
    let drive = chars.next()?;
    if !drive.is_ascii_alphabetic() || chars.next() != Some(':') || !text[2..].starts_with('\\') {
        return None;
    }
    Some((format!("{}:\\", drive.to_ascii_uppercase()), &text[3..]))
}

fn trim_win32_tail(raw: &str) -> String {
    raw.trim_end_matches([' ', '.']).to_string()
}

/// A long reset date wraps under the day so a half-width tile does not cut the clock.
pub fn tile_clock(line: &str) -> String {
    match line.split_once("일 ") {
        Some((date, time)) if date.contains('월') => format!("{date}일\n{time}"),
        _ => line.to_string(),
    }
}

/// The first screen hides pace, the ledger, and the week strip until the detail row is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupFold {
    pub pace: bool,
    pub ledger: bool,
    pub days: bool,
}

pub fn popup_fold(detail_open: bool) -> PopupFold {
    PopupFold {
        pace: detail_open,
        ledger: detail_open,
        days: detail_open,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

pub fn clamp_popup(
    mut x: i32,
    mut y: i32,
    width: i32,
    height: i32,
    work: WorkArea,
    margin: i32,
) -> (i32, i32) {
    if x + width > work.right - margin {
        x = work.right - width - margin;
    }
    if x < work.left + margin {
        x = work.left + margin;
    }
    if y + height > work.bottom - margin {
        y = work.bottom - height - margin;
    }
    if y < work.top + margin {
        y = work.top + margin;
    }
    (x, y)
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

    #[test]
    fn isolated_launch_leaves_the_installed_profile_alone() {
        let root = Path::new(r"C:\Users\user\AppData\Local");
        let product = profile_dir(root, false);
        let isolated = profile_dir(root, true);
        assert_eq!(
            product.file_name().and_then(|name| name.to_str()),
            Some("Jusage")
        );
        assert_eq!(
            isolated.file_name().and_then(|name| name.to_str()),
            Some("JusageIsolated")
        );
        assert!(!isolated.starts_with(&product));
        assert_eq!(data_dir(root), product.join("data"));
        assert_eq!(settings_path(root), product.join("settings.txt"));

        let product_boot = startup_guard(false);
        assert!(product_boot.copy_home);
        assert!(product_boot.touch_registry);
        assert_eq!(product_boot.mutex, r"Local\JusageSingle");
        assert_eq!(product_boot.window_class, "JusageWindow");
        assert_eq!(product_boot.profile, "Jusage");

        let qa = startup_guard(true);
        assert!(!qa.copy_home);
        assert!(!qa.touch_registry);
        assert_eq!(qa.mutex, r"Local\JusageIsolated");
        assert_eq!(qa.window_class, "JusageIsolatedWindow");
        assert_ne!(qa.mutex, product_boot.mutex);
        assert_ne!(qa.window_class, product_boot.window_class);

        let plain = parse_launch(Vec::<String>::new());
        assert!(!plain.isolated);
        assert!(plain.preview_dir.is_none());
        let flagged = parse_launch(["--isolated"]);
        assert!(flagged.isolated);
        assert!(flagged.preview_dir.is_none());
        let preview = parse_launch(["--preview", r"C:\Users\user\src\jusage-preview"]);
        assert!(preview.isolated);
        assert!(preview.preview_dir.is_some());
        assert!(isolates_from_product(
            preview.preview_dir.as_deref().unwrap()
        ));
        assert!(!isolates_from_product(Path::new(
            r"C:\Users\user\AppData\Local\Jusage"
        )));
        assert!(!isolates_from_product(Path::new(
            r"C:\Users\user\AppData\Local\Jusage\data"
        )));
        assert!(parse_launch(["--preview"]).preview_missing);
    }

    #[test]
    fn a_preview_path_cannot_fold_back_onto_the_installed_profile() {
        let product = r"C:\Users\user\AppData\Local\Jusage";
        assert!(!isolates_from_product(Path::new(
            r"C:\Users\user\AppData\Local\Jusage."
        )));
        assert!(!isolates_from_product(Path::new(
            r"C:\Users\user\AppData\Local\Jusage "
        )));
        assert!(!isolates_from_product(Path::new(
            r"C:\Users\user\AppData\Local\Jusage\data\.."
        )));
        assert!(!isolates_from_product(Path::new(r"..")));
        assert!(!isolates_from_product(Path::new(r"C:Jusage")));
        assert!(!isolates_from_product(Path::new(
            r"\\?\C:\Users\user\AppData\Local\Jusage\data"
        )));
        assert!(isolates_from_product(Path::new(
            r"C:\Users\user\AppData\Local\Jusage\..\jusage-preview"
        )));
        assert!(isolates_from_product(Path::new(
            r"C:\Users\user\AppData\Local\JusageIsolated"
        )));
        assert!(path_is_within(
            r"C:\Users\user\AppData\Local\Jusage\data",
            product
        ));
        assert!(path_is_within(
            r"C:\Users\user\AppData\Local\Jusage.",
            product
        ));
        assert!(!path_is_within(
            r"C:\Users\user\AppData\Local\JusageIsolated",
            product
        ));
        assert!(!path_is_within(
            r"C:\Users\user\src\jusage-preview",
            product
        ));
        assert_eq!(
            tile_clock("10월 30일 오전 5시 12분"),
            "10월 30일\n오전 5시 12분"
        );
        assert_eq!(tile_clock("오늘 오전 8시 39분"), "오늘 오전 8시 39분");
        assert_eq!(tile_clock("내일 오전 6시"), "내일 오전 6시");
        assert_eq!(tile_clock("지금 채워져요"), "지금 채워져요");
    }

    #[test]
    fn the_first_screen_hides_pace_and_history_until_detail_opens() {
        let closed = popup_fold(false);
        assert!(!closed.pace);
        assert!(!closed.ledger);
        assert!(!closed.days);
        let open = popup_fold(true);
        assert!(open.pace && open.ledger && open.days);
    }

    #[test]
    fn a_popup_on_the_other_monitor_stays_inside_that_work_area() {
        let right = WorkArea {
            left: 1920,
            top: 0,
            right: 3840,
            bottom: 1080,
        };
        let (x, y) = clamp_popup(1800, -40, 400, 700, right, 8);
        assert_eq!(x, 1928);
        assert_eq!(y, 8);
        let (x, y) = clamp_popup(3600, 900, 400, 700, right, 8);
        assert_eq!(x, 3432);
        assert_eq!(y, 372);
        let left = WorkArea {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        let (x, y) = clamp_popup(20, 20, 400, 500, left, 8);
        assert_eq!((x, y), (20, 20));
    }
}
