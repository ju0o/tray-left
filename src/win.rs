use std::ffi::c_void;
use std::sync::atomic::{
    AtomicBool, AtomicI32, AtomicI64, AtomicU64, AtomicU8, AtomicUsize, Ordering,
};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use windows_sys::Win32::Foundation::{
    GetLastError, COLORREF, ERROR_ALREADY_EXISTS, FALSE, FILETIME, HWND, LPARAM, RECT, SIZE,
    SYSTEMTIME, TRUE, WPARAM,
};
use windows_sys::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows_sys::Win32::Graphics::Gdi::{
    AlphaBlend, BeginPaint, CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW,
    CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
    GetDC, GetDeviceCaps, GetMonitorInfoW, GetTextExtentPoint32W, InvalidateRect, MonitorFromRect,
    ReleaseDC, RoundRect, SelectObject, SetBkMode, SetTextCharacterExtra, SetTextColor,
    SetWindowRgn, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    DIB_RGB_COLORS, DT_BOTTOM, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT,
    DT_SINGLELINE, DT_TOP, DT_VCENTER, DT_WORDBREAK, FW_NORMAL, FW_SEMIBOLD, HFONT, HGDIOBJ,
    LOGPIXELSX, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
};
use windows_sys::Win32::Storage::FileSystem::FileTimeToLocalFileTime;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteKeyValueW, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, RegSetKeyValueW,
    RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_ENUMERATE_SUB_KEYS, KEY_QUERY_VALUE, KEY_READ,
    KEY_SET_VALUE, REG_DWORD, REG_SZ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::System::Time::FileTimeToSystemTime;
use windows_sys::Win32::UI::Controls::{MARGINS, WM_MOUSELEAVE};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconGetRect, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, NOTIFYICONIDENTIFIER,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, CreateWindowExW, DefWindowProcW, DestroyIcon, DispatchMessageW,
    FindWindowW, GetMessageW, GetSystemMetrics, IsWindowVisible, KillTimer, LoadCursorW,
    PostMessageW, PostQuitMessage, RegisterClassW, SendMessageW, SetCursor, SetForegroundWindow,
    SetTimer, SetWindowPos, ShowWindow, SystemParametersInfoW, TranslateMessage, CS_DROPSHADOW,
    CS_HREDRAW, CS_VREDRAW, HICON, ICONINFO, IDC_ARROW, IDC_HAND, SM_CXSMICON, SPI_GETWORKAREA,
    SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOW, WA_INACTIVE, WM_ACTIVATE, WM_CLOSE, WM_DESTROY,
    WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_PAINT, WM_RBUTTONUP, WM_SETCURSOR, WM_SETICON, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSW,
    WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_POPUP,
};

use crate::icon::remaining_icon;
use crate::usage::{self, Civil, FetchFail, Pane, Reading, Snapshot};

const CARD_W: i32 = 392;
const MOTION_FRAMES: u8 = 16;
const COUNT_FRAMES: u8 = 20;
const TRAY_MSG: u32 = 0x8000 + 1;
const REFRESH_MSG: u32 = 0x8000 + 2;
const SHOW_MSG: u32 = 0x8000 + 3;
const ICON_ID: u32 = 1;
const HIT_CLOSE: u8 = 1;
const HIT_REFRESH: u8 = 2;
const HIT_QUIT: u8 = 3;
const HIT_SETTINGS: u8 = 4;
const ROW_SOURCE: u8 = 10;
const ROW_DISTRO: u8 = 11;
const ROW_FOCUS: u8 = 12;
const ROW_START: u8 = 13;
const ROW_THEME: u8 = 14;
const ROW_DONE: u8 = 15;
const ROW_PATH: u8 = 16;

#[derive(Clone)]
enum View {
    Loading,
    Ready(Snapshot),
    NeedLogin,
    Failed(u16),
}

struct Fonts {
    dpi: i32,
    brand: HFONT,
    hero: HFONT,
    unit: HFONT,
    figure: HFONT,
    lead: HFONT,
    label: HFONT,
    body: HFONT,
    quiet: HFONT,
    digit: HFONT,
}

unsafe impl Send for Fonts {}

struct Tile {
    spot: Spot,
    label: String,
    value: String,
    mood: String,
    clock: String,
    accent: u32,
    hot: bool,
}

#[derive(Clone, Copy)]
struct Spot {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Spot {
    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    fn rect(self) -> RECT {
        RECT {
            left: self.x,
            top: self.y,
            right: self.x + self.w,
            bottom: self.y + self.h,
        }
    }
}

struct Fact {
    label: Spot,
    value: Spot,
    rule: Option<Spot>,
    name: String,
    text: String,
    wrap: bool,
}

struct DayCol {
    name_box: Spot,
    open_box: Spot,
    close_box: Spot,
    tick: Option<Spot>,
    name: String,
    open: String,
    close: String,
    today: bool,
}

struct Card {
    width: i32,
    height: i32,
    close: Spot,
    gear: Spot,
    refresh: Spot,
    quit: Spot,
    brand_box: Spot,
    plan_box: Spot,
    hero_box: Spot,
    scope_box: Option<Spot>,
    rail: Option<Spot>,
    mood_box: Option<Spot>,
    bar: Option<Spot>,
    charge_box: Option<Spot>,
    charge_bar: Option<Spot>,
    top_rule: Option<Spot>,
    bottom_rule: Option<Spot>,
    refill_box: Option<Spot>,
    note_box: Option<Spot>,
    trust_box: Option<Spot>,
    ledger_rule: Option<Spot>,
    facts: Vec<Fact>,
    days_rule: Option<Spot>,
    days_label: Option<Spot>,
    days_open_label: Option<Spot>,
    days_close_label: Option<Spot>,
    days: Vec<DayCol>,
    story_box: Option<Spot>,
    story: String,
    pace_caption_box: Option<Spot>,
    pace_box: Option<Spot>,
    pace: String,
    cycle_box: Option<Spot>,
    cycle: String,
    tiles: Vec<Tile>,
    rows: Vec<SettingRow>,
    status_box: Option<Spot>,
    status: String,
    hero: String,
    scope: String,
    refill: String,
    charge: String,
    charge_fill: f32,
    note: String,
    trust: String,
    plan: String,
    hero_big: bool,
    remaining: u32,
}

struct Motion {
    tick: u8,
    live: bool,
}

struct CountUp {
    from: i32,
    to: i32,
    tick: u8,
}

struct Palette {
    light: bool,
    ink: u32,
    muted: u32,
    soft: u32,
    plate: [u8; 4],
    tile: [u8; 4],
    track: [u8; 4],
    line: [u8; 4],
}

static VIEW: Mutex<View> = Mutex::new(View::Loading);
static FONTS: Mutex<Option<Fonts>> = Mutex::new(None);
static HWND_SLOT: AtomicUsize = AtomicUsize::new(0);
static ICON_SLOT: AtomicUsize = AtomicUsize::new(0);
static QUIT_FLAG: AtomicBool = AtomicBool::new(false);
static HOVER: AtomicU8 = AtomicU8::new(0);
static PRESSED: AtomicU8 = AtomicU8::new(0);
static TRACKING: AtomicBool = AtomicBool::new(false);
static WAKE: OnceLock<Sender<()>> = OnceLock::new();
static DELTA: Mutex<String> = Mutex::new(String::new());
static LAST: Mutex<Option<Snapshot>> = Mutex::new(None);
static NOTICE: Mutex<String> = Mutex::new(String::new());
static CHECKING: AtomicBool = AtomicBool::new(false);
static CHECK_GEN: AtomicU64 = AtomicU64::new(0);
static FRESH_AT: AtomicI64 = AtomicI64::new(0);
static LAST_KIND: AtomicU8 = AtomicU8::new(0);
static SETTINGS_OPEN: AtomicBool = AtomicBool::new(false);
static CONFIG: Mutex<Option<crate::settings::Settings>> = Mutex::new(None);
static LAST_SIG: Mutex<String> = Mutex::new(String::new());
static DISTROS: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct SettingRow {
    spot: Spot,
    id: u8,
    label: String,
    value: String,
}
static TRUST_SEEN: AtomicBool = AtomicBool::new(false);
static TRUST_LOADED: AtomicBool = AtomicBool::new(false);
static MOTION: Mutex<Motion> = Mutex::new(Motion {
    tick: MOTION_FRAMES,
    live: false,
});
static COUNT: Mutex<CountUp> = Mutex::new(CountUp {
    from: 0,
    to: -1,
    tick: COUNT_FRAMES,
});
static SHOWN: AtomicI32 = AtomicI32::new(-1);

pub fn run() {
    settle_home();
    unsafe {
        let _ = set_process_dpi();
        let class_name = wide("JusageWindow");
        let title = wide("Jusage");
        let instance = GetModuleHandleW(std::ptr::null());
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_DROPSHADOW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        if RegisterClassW(&class) == 0 {
            return;
        }
        let mutex_name = wide("Local\\JusageSingle");
        let mutex = CreateMutexW(std::ptr::null(), FALSE, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let existing = FindWindowW(class_name.as_ptr(), std::ptr::null());
            if !existing.is_null() {
                let _ = PostMessageW(existing, SHOW_MSG, 0, 0);
            }
            return;
        }
        let _ = mutex;

        let card = card_for(&View::Loading, screen_dpi());
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CLIPSIBLINGS,
            80,
            80,
            card.width,
            card.height,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }
        HWND_SLOT.store(hwnd as usize, Ordering::SeqCst);
        let pref = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &pref as *const i32 as *const c_void,
            4,
        );
        enable_glass(hwnd);
        let icon = make_icon(
            icon_px(),
            0,
            usage::mood_rgb_on(None, system_is_light()),
            false,
        );
        ICON_SLOT.store(icon as usize, Ordering::SeqCst);
        assign_window_icon(hwnd, icon);
        add_tray(hwnd, icon, "Jusage 확인 중");
        boot_autostart();
        apply_view();
        let (tx, rx) = mpsc::channel();
        let _ = WAKE.set(tx);
        std::thread::spawn(move || worker(rx));
        std::thread::spawn(promote_loop);

        let mut msg = windows_sys::Win32::UI::WindowsAndMessaging::MSG::default();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        remove_tray(hwnd);
    }
}

struct ResetWatch {
    at: Option<i64>,
    remaining: Option<u32>,
    late_tries: u8,
}

fn worker(rx: mpsc::Receiver<()>) {
    let mut watch = ResetWatch {
        at: None,
        remaining: None,
        late_tries: 0,
    };
    let mut seen = current_activity();
    let mut due: Option<Instant> = None;
    let mut next_idle = Instant::now();
    let mut idle_fails = 0u32;
    let mut started = false;
    loop {
        if QUIT_FLAG.load(Ordering::Relaxed) {
            break;
        }
        if !started {
            started = true;
            let fetched = publish();
            watch = watch_after(fetched, None, 0);
            next_idle = Instant::now() + idle_gap(&mut idle_fails);
            seen = current_activity();
            continue;
        }
        let wait = wait_budget(due, watch.at, next_idle);
        let woke = rx.recv_timeout(wait).is_ok();
        if woke {
            due = None;
            let fetched = publish();
            while rx.try_recv().is_ok() {}
            if fetched.is_some() {
                watch = watch_after(fetched, None, 0);
            }
            next_idle = Instant::now() + idle_gap(&mut idle_fails);
            seen = current_activity();
            continue;
        }
        let current = current_activity();
        if current > seen {
            seen = current;
            due = Some(Instant::now() + Duration::from_secs(15));
        }
        if watch.at.is_some_and(|at| now_unix() >= at) {
            let previous = watch.remaining;
            let tries = watch.late_tries;
            let fetched = publish();
            if fetched.is_none() {
                let late_tries = tries.saturating_add(1);
                watch.late_tries = if late_tries >= 5 { 0 } else { late_tries };
                watch.at = if late_tries >= 5 {
                    None
                } else {
                    Some(now_unix().saturating_add(15))
                };
            } else {
                watch = watch_after(fetched, previous, tries);
            }
            next_idle = Instant::now() + idle_gap(&mut idle_fails);
            continue;
        }
        if due.is_some_and(|moment| Instant::now() >= moment) || next_idle <= Instant::now() {
            due = None;
            let fetched = publish();
            if fetched.is_some() {
                watch = watch_after(fetched, watch.remaining, 0);
            }
            next_idle = Instant::now() + idle_gap(&mut idle_fails);
        }
    }
}

fn idle_gap(fails: &mut u32) -> Duration {
    let kind = LAST_KIND.load(Ordering::Relaxed);
    if kind == 0 {
        *fails = 0;
        return Duration::from_secs(60);
    }
    if kind == 1 {
        *fails = 0;
        return Duration::from_secs(180);
    }
    *fails = fails.saturating_add(1);
    Duration::from_secs(crate::settings::idle_backoff(*fails))
}

fn current_activity() -> Option<std::time::SystemTime> {
    let Ok(home) = current_home() else {
        return None;
    };
    usage::newest_activity(&home)
}

fn wait_budget(due: Option<Instant>, reset_at: Option<i64>, idle_at: Instant) -> Duration {
    let activity = due
        .map(|moment| moment.saturating_duration_since(Instant::now()))
        .unwrap_or(Duration::from_secs(20));
    let mut wait = activity.min(Duration::from_secs(20));
    wait = wait.min(idle_at.saturating_duration_since(Instant::now()));
    if let Some(at) = reset_at {
        let now = now_unix();
        if at <= now {
            return Duration::from_secs(0);
        }
        wait = wait.min(Duration::from_secs((at - now) as u64));
    }
    wait
}

fn watch_after(snapshot: Option<Snapshot>, previous: Option<u32>, late_tries: u8) -> ResetWatch {
    let Some(snapshot) = snapshot else {
        return ResetWatch {
            at: None,
            remaining: previous,
            late_tries: 0,
        };
    };
    let remaining = shown_focus(&snapshot).remaining;
    let now = now_unix();
    let (at, late_tries) = usage::next_reset_watch(
        now,
        usage::soonest_reset(&snapshot, now),
        remaining,
        previous,
        late_tries,
    );
    ResetWatch {
        at,
        remaining: Some(remaining),
        late_tries,
    }
}

fn publish() -> Option<Snapshot> {
    let started = CHECK_GEN.load(Ordering::Relaxed);
    let home = match current_home() {
        Ok(path) => path,
        Err(message) => {
            note_home_problem(message);
            LAST_KIND.store(3, Ordering::Relaxed);
            finish_publish(started);
            return None;
        }
    };
    let signature = source_signature();
    let switched = LAST_SIG.lock().ok().is_some_and(|have| *have != signature);
    if switched {
        if let Ok(mut last) = LAST.lock() {
            *last = None;
        }
    }
    let had_number = LAST.lock().ok().is_some_and(|slot| slot.is_some());
    let (kind, snapshot, code) = match usage::fetch_usage(&home) {
        Ok(snapshot) => (usage::FetchKind::Ok, Some(snapshot), 0),
        Err(FetchFail::NoLogin) => (usage::FetchKind::NoLogin, None, 0),
        Err(fail) => (usage::FetchKind::Failed, None, fail.code()),
    };
    let returned = match usage::after_fetch(had_number, kind) {
        usage::AfterFetch::Ready => {
            let snapshot = snapshot?;
            note_change(&snapshot);
            if let Ok(mut last) = LAST.lock() {
                *last = Some(snapshot.clone());
            }
            clear_notice();
            FRESH_AT.store(now_unix(), Ordering::Relaxed);
            LAST_KIND.store(0, Ordering::Relaxed);
            if let Ok(mut sig) = LAST_SIG.lock() {
                *sig = signature;
            }
            if let Ok(mut view) = VIEW.lock() {
                *view = View::Ready(snapshot.clone());
            }
            Some(snapshot)
        }
        usage::AfterFetch::NeedLogin => {
            clear_delta();
            if let Ok(mut last) = LAST.lock() {
                *last = None;
            }
            clear_notice();
            LAST_KIND.store(1, Ordering::Relaxed);
            if let Ok(mut view) = VIEW.lock() {
                *view = View::NeedLogin;
            }
            None
        }
        usage::AfterFetch::KeepLast => {
            LAST_KIND.store(2, Ordering::Relaxed);
            if let Ok(mut notice) = NOTICE.lock() {
                *notice = crate::settings::fail_note(code).to_string();
            }
            if let Ok(last) = LAST.lock() {
                if let Some(snapshot) = last.clone() {
                    if let Ok(mut view) = VIEW.lock() {
                        *view = View::Ready(snapshot);
                    }
                }
            }
            None
        }
        usage::AfterFetch::FirstFail => {
            clear_delta();
            LAST_KIND.store(2, Ordering::Relaxed);
            if let Ok(mut notice) = NOTICE.lock() {
                *notice = crate::settings::fail_note(code).to_string();
            }
            if let Ok(mut view) = VIEW.lock() {
                *view = View::Failed(code);
            }
            None
        }
    };
    finish_publish(started);
    returned
}

fn finish_publish(started: u64) {
    if CHECK_GEN.load(Ordering::Relaxed) == started {
        CHECKING.store(false, Ordering::Relaxed);
    }
    let hwnd = HWND_SLOT.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() {
        unsafe {
            let _ = PostMessageW(hwnd, REFRESH_MSG, 0, 0);
        }
    }
}

fn note_home_problem(message: &str) {
    let had_number = LAST.lock().ok().is_some_and(|slot| slot.is_some());
    if let Ok(mut notice) = NOTICE.lock() {
        *notice = message.to_string();
    }
    if had_number {
        if let Ok(last) = LAST.lock() {
            if let Some(snapshot) = last.clone() {
                if let Ok(mut view) = VIEW.lock() {
                    *view = View::Ready(snapshot);
                }
            }
        }
        return;
    }
    clear_delta();
    if let Ok(mut view) = VIEW.lock() {
        *view = View::Failed(1);
    }
}

fn clear_notice() {
    if let Ok(mut notice) = NOTICE.lock() {
        notice.clear();
    }
}

fn request_refresh() {
    let had_number = LAST.lock().ok().is_some_and(|slot| slot.is_some());
    if usage::manual_refresh_keeps_number(had_number) {
        CHECK_GEN.fetch_add(1, Ordering::Relaxed);
        CHECKING.store(true, Ordering::Relaxed);
    } else {
        let Ok(mut view) = VIEW.lock() else {
            return;
        };
        if matches!(*view, View::Loading) {
            return;
        }
        *view = View::Loading;
    }
    if let Some(tx) = WAKE.get() {
        let _ = tx.send(());
    }
    let hwnd = HWND_SLOT.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() {
        unsafe {
            InvalidateRect(hwnd, std::ptr::null(), FALSE);
        }
    }
}

unsafe fn apply_view() {
    let Some(view) = VIEW.lock().ok().map(|view| view.clone()) else {
        return;
    };
    let (tip, remaining, rgb, kind, left, plan, code) = describe(&view);
    let hwnd = HWND_SLOT.load(Ordering::SeqCst) as HWND;
    let show_number = matches!(view, View::Ready(_));
    let icon = make_icon(icon_px(), remaining, rgb, show_number);
    if !hwnd.is_null() {
        update_tray(hwnd, icon, &tip);
        assign_window_icon(hwnd, icon);
    }
    let old = ICON_SLOT.swap(icon as usize, Ordering::SeqCst) as HICON;
    if !old.is_null() && old != icon {
        DestroyIcon(old);
    }
    let focus_name = match &view {
        View::Ready(snapshot) => shown_focus(snapshot).name.to_string(),
        _ => String::new(),
    };
    let week_left = match &view {
        View::Ready(snapshot) => usage::readings(snapshot).1.map(|reading| reading.remaining),
        _ => None,
    };
    write_state(kind, left, &plan, code, &focus_name, week_left);
    if hwnd.is_null() {
        return;
    }
    let visible = IsWindowVisible(hwnd) != 0;
    let counting = arm_count(visible);
    if visible {
        place(hwnd);
    }
    InvalidateRect(hwnd, std::ptr::null(), FALSE);
    if counting {
        SetTimer(hwnd, 1, 16, None);
    }
}

fn describe(
    view: &View,
) -> (
    String,
    u32,
    [u8; 3],
    &'static str,
    Option<u32>,
    String,
    Option<u16>,
) {
    let light = theme_is_light();
    match view {
        View::Loading => (
            "Jusage 확인 중".into(),
            0,
            usage::mood_rgb_on(None, light),
            "loading",
            None,
            String::new(),
            None,
        ),
        View::NeedLogin => (
            "Jusage 로그인 필요".into(),
            0,
            usage::mood_rgb_on(None, light),
            "login",
            None,
            String::new(),
            None,
        ),
        View::Failed(code) => (
            "Jusage 확인 실패".into(),
            0,
            usage::mood_rgb_on(None, light),
            "failed",
            None,
            String::new(),
            Some(*code),
        ),
        View::Ready(snapshot) => {
            let focus = shown_focus(snapshot);
            let now = now_unix();
            let pane_key = match focus.pane {
                Pane::Session => "session",
                Pane::Week => "week",
            };
            let brief = usage::pace_brief(&pace_for(pane_key), now, focus.remaining);
            (
                usage::tray_tip(&usage::hover_tip(snapshot), &brief),
                focus.remaining,
                usage::mood_rgb_on(Some(focus.remaining), light),
                "ready",
                Some(focus.remaining),
                snapshot.plan.clone(),
                None,
            )
        }
    }
}

fn rgb_color(rgb: [u8; 3]) -> u32 {
    rgb[0] as u32 | ((rgb[1] as u32) << 8) | ((rgb[2] as u32) << 16)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn reset_unix(at: Option<i64>, after: Option<i64>, now: i64) -> Option<i64> {
    if let Some(at) = at.filter(|value| *value > 1_000_000_000) {
        return Some(at);
    }
    after.map(|seconds| now.saturating_add(seconds))
}

fn local_civil(unix: i64) -> Option<Civil> {
    if unix < 0 {
        return None;
    }
    let ticks = (unix as u64)
        .saturating_add(11_644_473_600)
        .saturating_mul(10_000_000);
    let utc = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut local = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut system = SYSTEMTIME {
        wYear: 0,
        wMonth: 0,
        wDayOfWeek: 0,
        wDay: 0,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    unsafe {
        if FileTimeToLocalFileTime(&utc, &mut local) == 0 {
            return None;
        }
        if FileTimeToSystemTime(&local, &mut system) == 0 {
            return None;
        }
    }
    Some(Civil {
        year: system.wYear as i32,
        month: system.wMonth as u8,
        day: system.wDay as u8,
        hour: system.wHour as u8,
        minute: system.wMinute as u8,
    })
}

fn card_for(view: &View, dpi: i32) -> Card {
    if SETTINGS_OPEN.load(Ordering::Relaxed) {
        return settings_card(dpi);
    }
    let s = |value: i32| scale(value, dpi);
    let mut tiles = Vec::new();
    let hero;
    let mut scope = String::new();
    let mut refill = String::new();
    let mut charge = String::new();
    let mut charge_fill = None;
    let mut note = String::new();
    let mut delta = String::new();
    let mut fact_rows: Vec<(String, String)> = Vec::new();
    let mut pace_lines: Vec<String> = Vec::new();
    let mut cycle = String::new();
    let mut day_slots: Vec<usage::DaySlot> = Vec::new();
    let mut plan = String::new();
    let mut status = String::new();
    let mut hero_big = false;
    let mut remaining = 0u32;
    let light = theme_is_light();
    match view {
        View::Loading => hero = "확인 중".to_string(),
        View::NeedLogin => {
            hero = "로그인이 필요해요".to_string();
            note = "Codex에 로그인한 뒤 다시 확인해 주세요.".to_string();
        }
        View::Failed(_) => {
            hero = "확인하지 못했어요".to_string();
            note = NOTICE
                .lock()
                .ok()
                .filter(|notice| !notice.is_empty())
                .map(|notice| notice.clone())
                .unwrap_or_else(|| "잠시 뒤 다시 확인해 주세요.".to_string());
        }
        View::Ready(snapshot) => {
            let (session, week) = usage::readings(snapshot);
            let focus = shown_focus(snapshot);
            remaining = focus.remaining;
            hero = format!("{}%", focus.remaining);
            hero_big = true;
            scope = focus.name.to_string();
            plan = snapshot.plan.clone();
            let now = now_unix();
            let window_secs = match focus.pane {
                Pane::Session => snapshot.session_window_seconds,
                Pane::Week => snapshot.weekly_window_seconds,
            };
            if let Some(at) = reset_unix(focus.reset_at, focus.reset_after, now) {
                if let Some(state) = usage::charge_state(now, at, window_secs) {
                    charge = state.phrase;
                    charge_fill = state.fraction;
                    if state.seconds_left > 0 {
                        if let (Some(today), Some(moment)) = (local_civil(now), local_civil(at)) {
                            refill = usage::clock_when(today, moment);
                        }
                    }
                }
            }
            if let Some(line) = DELTA.lock().ok() {
                delta = line.clone();
            }
            fact_rows.push((
                "쓴 양".into(),
                format!("{}%", 100u32.saturating_sub(focus.remaining)),
            ));
            let pane_key = match focus.pane {
                Pane::Session => "session",
                Pane::Week => "week",
            };
            let today_key = local_civil(now)
                .map(|time| format!("{:04}-{:02}-{:02}", time.year, time.month, time.day))
                .unwrap_or_default();
            if !today_key.is_empty() {
                if let Some(time) = local_civil(now) {
                    day_slots =
                        usage::recent_days(&read_days(), pane_key, time.year, time.month, time.day)
                            .to_vec();
                }
            }
            if let Some(mark) = read_day() {
                if mark.day == today_key && mark.pane == pane_key {
                    fact_rows.push((
                        "오늘".into(),
                        usage::account_move_value(mark.open, focus.remaining),
                    ));
                }
            }
            if let Some(low) = day_slots
                .iter()
                .find(|slot| slot.today)
                .and_then(|slot| slot.low)
            {
                fact_rows.push(("오늘 최저".into(), format!("{low}%")));
                if let Some(up) = usage::lifted_since_low(low, focus.remaining) {
                    fact_rows.push(("최저 이후".into(), up));
                }
            }
            if let Some(close) = usage::yesterday_close(&day_slots) {
                fact_rows.push(("어제".into(), format!("{close}%로 마감")));
            }
            if let Some((name, low)) = usage::earlier_low(&day_slots) {
                fact_rows.push(("기록 최저".into(), format!("{name} {low}%")));
            }
            if let Some(value) = usage::average_day_value(&day_slots) {
                fact_rows.push(("하루 평균".into(), value));
            }
            let reset_at = reset_unix(focus.reset_at, focus.reset_after, now);
            let pace_samples = pace_for(pane_key);
            pace_lines.push(usage::pace_line(
                &pace_samples,
                now,
                focus.remaining,
                reset_at,
            ));
            if let (Some(window), Some(at)) = (window_secs, reset_at) {
                if let Some(line) =
                    usage::window_balance(window, at.saturating_sub(now), focus.remaining)
                {
                    pace_lines.push(line);
                }
            }
            if let Some(line) = usage::refill_guess(&pace_samples, now, focus.remaining, reset_at) {
                pace_lines.push(line);
            }
            if let Some(shift) = usage::pace_shift(&pace_samples, now, focus.remaining) {
                pace_lines.push(shift);
            }
            cycle = usage::cycle_line(&read_cycle(), pane_key, focus.remaining);
            if !delta.is_empty() {
                fact_rows.push(("지난 확인".into(), delta.clone()));
            }
            if let Some(week) = week {
                let mut session_tile = tile_from(session, light, reset_clock(session, now));
                let mut week_tile = tile_from(week, light, reset_clock(week, now));
                let session_brief = usage::tile_brief(
                    true,
                    &usage::pace_brief(&pace_for("session"), now, session.remaining),
                );
                let week_brief = usage::tile_brief(
                    true,
                    &usage::pace_brief(&pace_for("week"), now, week.remaining),
                );
                if !session_brief.is_empty() {
                    session_tile.mood = session_brief;
                }
                if !week_brief.is_empty() {
                    week_tile.mood = week_brief;
                }
                session_tile.hot = focus.pane == Pane::Session;
                week_tile.hot = focus.pane == Pane::Week;
                tiles.push(session_tile);
                tiles.push(week_tile);
            }
            if CHECKING.load(Ordering::Relaxed) {
                note = usage::CHECKING_NOTE.to_string();
            } else if let Ok(notice) = NOTICE.lock() {
                if !notice.is_empty() {
                    note = notice.clone();
                }
            }
            let fresh_at = FRESH_AT.load(Ordering::Relaxed);
            if fresh_at > 0 {
                let age = now.saturating_sub(fresh_at);
                status = format!(
                    "{} · {}",
                    crate::settings::source_label(config().source),
                    crate::settings::fresh_line(age)
                );
            }
        }
    }
    let pace = pace_lines.join("\n");
    let show_charge = charge_fill.is_some();
    let charge_fill = charge_fill.unwrap_or(0.0);
    let width = s(CARD_W);
    let pad = s(32);
    let close_size = s(40);
    let close = Spot {
        x: width - close_size - s(16),
        y: s(14),
        w: close_size,
        h: close_size,
    };
    let gear = Spot {
        x: close.x - s(8) - s(72),
        y: close.y,
        w: s(72),
        h: close_size,
    };
    let brand_box = Spot {
        x: pad,
        y: s(22),
        w: s(150),
        h: s(16),
    };
    let plan_box = Spot {
        x: pad + s(150),
        y: s(22),
        w: (gear.x - pad - s(150) - s(8)).max(s(48)),
        h: s(16),
    };
    let content_x = pad + s(16);
    let content_w = width - pad - content_x;
    let rule_h = s(1).max(2);
    let trust = if trust_seen() {
        String::new()
    } else {
        usage::TRUST_LINE.to_string()
    };
    let trust_box = if trust.is_empty() {
        None
    } else {
        Some(Spot {
            x: content_x,
            y: s(60),
            w: content_w,
            h: s(44),
        })
    };
    let mut y = trust_box
        .as_ref()
        .map(|spot| spot.y + spot.h + s(8))
        .unwrap_or(s(72));
    let scope_box;
    let rail;
    let mood_box;
    let bar;
    let hero_box;
    if hero_big {
        scope_box = Some(Spot {
            x: content_x,
            y,
            w: content_w,
            h: s(16),
        });
        y += s(16) + s(2);
        let number_h = s(72);
        let mood_w = s(88);
        hero_box = Spot {
            x: content_x,
            y,
            w: content_w - mood_w,
            h: number_h,
        };
        mood_box = Some(Spot {
            x: content_x + content_w - mood_w,
            y,
            w: mood_w,
            h: number_h,
        });
        rail = Some(Spot {
            x: pad,
            y: y + s(8),
            w: s(3).max(3),
            h: number_h - s(16),
        });
        y += number_h + s(12);
        bar = Some(Spot {
            x: content_x,
            y,
            w: content_w,
            h: s(11),
        });
        y += s(11) + s(12);
    } else {
        hero_box = Spot {
            x: content_x,
            y,
            w: content_w,
            h: s(36),
        };
        y += hero_box.h + s(8);
        scope_box = None;
        rail = None;
        mood_box = None;
        bar = None;
    }
    let note_box = if note.is_empty() {
        None
    } else {
        let lines = wrapped_line_count(&note, content_w, s(15));
        let spot = Spot {
            x: content_x,
            y,
            w: content_w,
            h: lines * s(22) + s(4),
        };
        y += spot.h + s(8);
        Some(spot)
    };
    let has_charge = !charge.is_empty() || !refill.is_empty();
    let top_rule = if has_charge || !tiles.is_empty() {
        let spot = Spot {
            x: content_x,
            y,
            w: content_w,
            h: rule_h,
        };
        y += rule_h + s(16);
        Some(spot)
    } else {
        None
    };
    let charge_box;
    let charge_bar;
    let refill_box;
    if !has_charge {
        charge_box = None;
        charge_bar = None;
        refill_box = None;
    } else {
        let row_h = s(24);
        let phrase_w = content_w * 58 / 100;
        charge_box = if charge.is_empty() {
            None
        } else {
            Some(Spot {
                x: content_x,
                y,
                w: phrase_w,
                h: row_h,
            })
        };
        refill_box = if refill.is_empty() {
            None
        } else {
            Some(Spot {
                x: content_x + phrase_w,
                y,
                w: content_w - phrase_w,
                h: row_h,
            })
        };
        if charge_box.is_some() || refill_box.is_some() {
            y += row_h + s(8);
        }
        charge_bar = if show_charge {
            let spot = Spot {
                x: content_x,
                y,
                w: content_w,
                h: s(11),
            };
            y += spot.h;
            Some(spot)
        } else {
            None
        };
        y += s(16);
    }
    if !tiles.is_empty() {
        let gap = s(10);
        let tile_w = (content_w - gap) / tiles.len() as i32;
        let tile_h = s(128);
        for (index, tile) in tiles.iter_mut().enumerate() {
            tile.spot = Spot {
                x: content_x + (tile_w + gap) * index as i32,
                y,
                w: tile_w,
                h: tile_h,
            };
        }
        y += tile_h + s(12);
    }
    let pace_caption_box;
    let pace_box;
    if pace.is_empty() {
        pace_caption_box = None;
        pace_box = None;
    } else {
        let caption = Spot {
            x: content_x,
            y,
            w: content_w,
            h: s(16),
        };
        y += caption.h + s(4);
        let lines = wrapped_line_count(&pace, content_w, s(15));
        let body = Spot {
            x: content_x,
            y,
            w: content_w,
            h: lines * s(22) + s(4),
        };
        y += body.h + s(12);
        pace_caption_box = Some(caption);
        pace_box = Some(body);
    }
    let cycle_box = if cycle.is_empty() {
        None
    } else {
        let lines = wrapped_line_count(&cycle, content_w, s(15));
        let spot = Spot {
            x: content_x,
            y,
            w: content_w,
            h: lines * s(22) + s(4),
        };
        y += spot.h + s(10);
        Some(spot)
    };
    let mut facts = Vec::new();
    let ledger_rule = if fact_rows.is_empty() {
        None
    } else {
        let spot = Spot {
            x: content_x,
            y,
            w: content_w,
            h: rule_h,
        };
        y += rule_h + s(12);
        Some(spot)
    };
    let fact_count = fact_rows.len();
    for (index, (name, text)) in fact_rows.into_iter().enumerate() {
        let label_w = content_w * 30 / 100;
        let row_h = s(32);
        let last = index + 1 == fact_count;
        let rule = if last {
            None
        } else {
            Some(Spot {
                x: content_x,
                y: y + row_h + s(4),
                w: content_w,
                h: rule_h,
            })
        };
        facts.push(Fact {
            label: Spot {
                x: content_x,
                y,
                w: label_w,
                h: row_h,
            },
            value: Spot {
                x: content_x + label_w,
                y,
                w: content_w - label_w,
                h: row_h,
            },
            rule,
            name,
            text,
            wrap: false,
        });
        y += row_h + s(4);
        if !last {
            y += rule_h + s(4);
        }
    }
    if !facts.is_empty() {
        y += s(8);
    }
    let days_rule;
    let days_label;
    let days_open_label;
    let days_close_label;
    let mut days = Vec::new();
    if day_slots.is_empty() {
        days_rule = None;
        days_label = None;
        days_open_label = None;
        days_close_label = None;
    } else {
        let spot = Spot {
            x: content_x,
            y,
            w: content_w,
            h: rule_h,
        };
        y += rule_h + s(12);
        days_rule = Some(spot);
        let label = Spot {
            x: content_x,
            y,
            w: content_w,
            h: s(18),
        };
        y += label.h + s(6);
        days_label = Some(label);
        let gap = s(4);
        let side = s(36);
        let col_w = (content_w - side - gap * 7) / 7;
        let x0 = content_x + side + gap;
        let name_h = s(16);
        let num_h = s(18);
        let open_y = y + name_h + s(4);
        let close_y = open_y + num_h;
        days_open_label = Some(Spot {
            x: content_x,
            y: open_y,
            w: side,
            h: num_h,
        });
        days_close_label = Some(Spot {
            x: content_x,
            y: close_y,
            w: side,
            h: num_h,
        });
        for (index, slot) in day_slots.iter().enumerate() {
            let x = x0 + (col_w + gap) * index as i32;
            let tick = if slot.today {
                Some(Spot {
                    x: x + (col_w - s(12)) / 2,
                    y: y + name_h,
                    w: s(12),
                    h: s(2).max(2),
                })
            } else {
                None
            };
            days.push(DayCol {
                name_box: Spot {
                    x,
                    y,
                    w: col_w,
                    h: name_h,
                },
                open_box: Spot {
                    x,
                    y: open_y,
                    w: col_w,
                    h: num_h,
                },
                close_box: Spot {
                    x,
                    y: close_y,
                    w: col_w,
                    h: num_h,
                },
                tick,
                name: slot.name.to_string(),
                open: slot
                    .open
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "·".to_string()),
                close: slot
                    .close
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "·".to_string()),
                today: slot.today,
            });
        }
        y = close_y + num_h + s(6);
    }
    let story = usage::week_story(&day_slots);
    let story_box = if story.is_empty() {
        None
    } else {
        let lines = wrapped_line_count(&story, content_w, s(15));
        let spot = Spot {
            x: content_x,
            y,
            w: content_w,
            h: lines * s(22) + s(4),
        };
        y += spot.h + s(8);
        Some(spot)
    };
    let status_box = if status.is_empty() {
        None
    } else {
        let lines = wrapped_line_count(&status, content_w, s(15));
        let spot = Spot {
            x: content_x,
            y,
            w: content_w,
            h: lines * s(20) + s(2),
        };
        y += spot.h + s(8);
        Some(spot)
    };
    let bottom_rule = Some(Spot {
        x: content_x,
        y,
        w: content_w,
        h: rule_h,
    });
    y += rule_h + s(12);
    let button_w = s(108);
    let refresh = Spot {
        x: width - content_x - button_w,
        y,
        w: button_w,
        h: s(32),
    };
    y += refresh.h;
    let quit = Spot {
        x: content_x,
        y,
        w: content_w,
        h: s(32),
    };
    y += quit.h + s(12);
    Card {
        width,
        height: y,
        close,
        gear,
        refresh,
        quit,
        brand_box,
        plan_box,
        hero_box,
        scope_box,
        rail,
        mood_box,
        bar,
        charge_box,
        charge_bar,
        top_rule,
        bottom_rule,
        refill_box,
        note_box,
        trust_box,
        ledger_rule,
        facts,
        days_rule,
        days_label,
        days_open_label,
        days_close_label,
        days,
        story_box,
        story,
        pace_caption_box,
        pace_box,
        pace,
        cycle_box,
        cycle,
        tiles,
        rows: Vec::new(),
        status_box,
        status,
        hero,
        scope,
        refill,
        charge,
        charge_fill,
        note,
        trust,
        plan,
        hero_big,
        remaining,
    }
}

fn tile_from(reading: Reading, light: bool, clock: String) -> Tile {
    Tile {
        spot: Spot {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        },
        label: reading.name.to_string(),
        value: format!("{}%", reading.remaining),
        mood: usage::mood_word(reading.remaining).to_string(),
        clock,
        accent: rgb_color(usage::mood_rgb_on(Some(reading.remaining), light)),
        hot: false,
    }
}

fn reset_clock(reading: Reading, now: i64) -> String {
    let Some(at) = reset_unix(reading.reset_at, reading.reset_after, now) else {
        return String::new();
    };
    if at <= now {
        return "지금 채워져요".to_string();
    }
    let (Some(today), Some(moment)) = (local_civil(now), local_civil(at)) else {
        return String::new();
    };
    usage::clock_when(today, moment)
}

fn settings_card(dpi: i32) -> Card {
    let s = |value: i32| scale(value, dpi);
    let cfg = config();
    let width = s(CARD_W);
    let pad = s(32);
    let close_size = s(40);
    let close = Spot {
        x: width - close_size - s(16),
        y: s(14),
        w: close_size,
        h: close_size,
    };
    let gear = Spot {
        x: close.x - s(8) - s(72),
        y: close.y,
        w: s(72),
        h: close_size,
    };
    let content_x = pad + s(16);
    let content_w = width - pad - content_x;
    let mut y = s(78);
    let hero_box = Spot {
        x: content_x,
        y,
        w: content_w,
        h: s(36),
    };
    y += hero_box.h + s(6);
    let note = "비밀번호는 받지 않습니다. 항목을 누르면 바뀝니다.".to_string();
    let note_h = wrapped_line_count(&note, content_w, s(15)) * s(22) + s(4);
    let note_box = Spot {
        x: content_x,
        y,
        w: content_w,
        h: note_h,
    };
    y += note_h + s(12);
    let mut rows = Vec::new();
    let add = |id: u8, label: &str, value: String, y: &mut i32, rows: &mut Vec<SettingRow>| {
        let spot = Spot {
            x: content_x,
            y: *y,
            w: content_w,
            h: s(44),
        };
        *y += spot.h + s(8);
        rows.push(SettingRow {
            spot,
            id,
            label: label.to_string(),
            value,
        });
    };
    add(
        ROW_SOURCE,
        "환경",
        match cfg.source {
            crate::settings::Source::Windows => "Windows Codex".to_string(),
            crate::settings::Source::Wsl => "WSL Codex".to_string(),
        },
        &mut y,
        &mut rows,
    );
    if cfg.source == crate::settings::Source::Wsl {
        let known = DISTROS
            .lock()
            .ok()
            .map(|slot| slot.clone())
            .unwrap_or_default();
        let value = if !cfg.distro.is_empty() {
            cfg.distro.clone()
        } else if known.is_empty() {
            "찾지 못했어요".to_string()
        } else {
            "눌러서 고르기".to_string()
        };
        add(ROW_DISTRO, "배포판", value, &mut y, &mut rows);
    }
    add(
        ROW_FOCUS,
        "대표 사용량",
        match cfg.focus {
            crate::settings::FocusPref::Auto => "자동".to_string(),
            crate::settings::FocusPref::Session => "첫 창".to_string(),
            crate::settings::FocusPref::Week => "한 주".to_string(),
        },
        &mut y,
        &mut rows,
    );
    add(
        ROW_START,
        "시작 시 실행",
        match cfg.autostart {
            crate::settings::AutostartPref::On => "켜짐".to_string(),
            crate::settings::AutostartPref::Off => "꺼짐".to_string(),
            crate::settings::AutostartPref::Keep => "그대로".to_string(),
        },
        &mut y,
        &mut rows,
    );
    add(
        ROW_THEME,
        "테마",
        match cfg.theme {
            crate::settings::ThemePref::System => "시스템".to_string(),
            crate::settings::ThemePref::Dark => "어둡게".to_string(),
            crate::settings::ThemePref::Light => "밝게".to_string(),
        },
        &mut y,
        &mut rows,
    );
    add(
        ROW_PATH,
        "경로",
        if cfg.codex_home.is_empty() {
            "기본".to_string()
        } else {
            "지정한 경로 · 누르면 기본".to_string()
        },
        &mut y,
        &mut rows,
    );
    add(
        ROW_DONE,
        "화면",
        "사용량으로".to_string(),
        &mut y,
        &mut rows,
    );
    y += s(8);
    let rule_h = s(1).max(2);
    let bottom_rule = Some(Spot {
        x: content_x,
        y,
        w: content_w,
        h: rule_h,
    });
    y += rule_h + s(8);
    let quit = Spot {
        x: content_x,
        y,
        w: content_w,
        h: s(36),
    };
    y += quit.h + s(16);
    Card {
        width,
        height: y,
        close,
        gear,
        refresh: Spot {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        },
        quit,
        brand_box: Spot {
            x: pad,
            y: s(22),
            w: s(150),
            h: s(16),
        },
        plan_box: Spot {
            x: pad + s(150),
            y: s(22),
            w: (gear.x - pad - s(150) - s(8)).max(s(40)),
            h: s(16),
        },
        hero_box,
        scope_box: None,
        rail: None,
        mood_box: None,
        bar: None,
        charge_box: None,
        charge_bar: None,
        top_rule: None,
        bottom_rule,
        refill_box: None,
        note_box: Some(note_box),
        trust_box: None,
        ledger_rule: None,
        facts: Vec::new(),
        days_rule: None,
        days_label: None,
        days_open_label: None,
        days_close_label: None,
        days: Vec::new(),
        story_box: None,
        story: String::new(),
        pace_caption_box: None,
        pace_box: None,
        pace: String::new(),
        cycle_box: None,
        cycle: String::new(),
        tiles: Vec::new(),
        rows,
        status_box: None,
        status: String::new(),
        hero: "설정".to_string(),
        scope: String::new(),
        refill: String::new(),
        charge: String::new(),
        charge_fill: 0.0,
        note,
        trust: String::new(),
        plan: String::new(),
        hero_big: false,
        remaining: 0,
    }
}

fn wrapped_line_count(text: &str, width: i32, font_px: i32) -> i32 {
    let per = (width / font_px.max(1)).max(8) as usize;
    let mut lines = 0i32;
    for part in text.split('\n') {
        let chars = part.chars().count().max(1);
        lines += ((chars + per - 1) / per) as i32;
    }
    lines.max(1)
}

unsafe fn work_area() -> RECT {
    let mut work = RECT::default();
    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0);
    work
}

unsafe fn tray_anchor(hwnd: HWND) -> Option<RECT> {
    let id = NOTIFYICONIDENTIFIER {
        cbSize: std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32,
        hWnd: hwnd,
        uID: ICON_ID,
        guidItem: windows_sys::core::GUID::from_u128(0),
    };
    let mut rect = RECT::default();
    if Shell_NotifyIconGetRect(&id, &mut rect) != 0 {
        return None;
    }
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return None;
    }
    Some(rect)
}

unsafe fn place(hwnd: HWND) {
    let dpi = window_dpi(hwnd);
    let Some(view) = VIEW.lock().ok().map(|view| view.clone()) else {
        return;
    };
    let card = card_for(&view, dpi);
    let margin = scale(8, dpi);
    let (mut x, mut y, work) = if let Some(anchor) = tray_anchor(hwnd) {
        let monitor = MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        let work = if !monitor.is_null() && GetMonitorInfoW(monitor, &mut info) != 0 {
            info.rcWork
        } else {
            work_area()
        };
        let x = anchor.right - card.width;
        let mut y = anchor.top - card.height - margin + motion_offset();
        if y < work.top + margin {
            y = anchor.bottom + margin;
        }
        (x, y, work)
    } else {
        let work = work_area();
        let x = work.right - card.width - scale(16, dpi);
        let y = work.bottom - card.height - margin + motion_offset();
        (x, y, work)
    };
    if x + card.width > work.right - margin {
        x = work.right - card.width - margin;
    }
    if x < work.left + margin {
        x = work.left + margin;
    }
    if y + card.height > work.bottom - margin {
        y = work.bottom - card.height - margin;
    }
    if y < work.top + margin {
        y = work.top + margin;
    }
    SetWindowPos(
        hwnd,
        std::ptr::null_mut(),
        x,
        y,
        card.width,
        card.height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    let radius = scale(26, dpi);
    let region = CreateRoundRectRgn(0, 0, card.width + 1, card.height + 1, radius, radius);
    if !region.is_null() {
        SetWindowRgn(hwnd, region, TRUE);
    }
}

fn motion_reduced() -> bool {
    let mut enabled = 1i32;
    let ok =
        unsafe { SystemParametersInfoW(0x1042, 0, &mut enabled as *mut i32 as *mut c_void, 0) };
    ok != 0 && enabled == 0
}

unsafe fn show_panel(hwnd: HWND) {
    let reduced = motion_reduced();
    if let Ok(mut motion) = MOTION.lock() {
        motion.tick = if reduced { MOTION_FRAMES } else { 0 };
        motion.live = !reduced;
    }
    begin_open_count();
    if reduced {
        if let Ok(mut count) = COUNT.lock() {
            count.tick = COUNT_FRAMES;
        }
    }
    place(hwnd);
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
    if !reduced {
        SetTimer(hwnd, 1, 16, None);
    }
    SetTimer(hwnd, 2, 30_000, None);
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    match msg {
        TRAY_MSG => {
            let event = lparam as u32;
            let low = event & 0xffff;
            if event == WM_RBUTTONUP
                || event == WM_LBUTTONUP
                || event == WM_LBUTTONDBLCLK
                || low == WM_RBUTTONUP
                || low == WM_LBUTTONUP
                || low == WM_LBUTTONDBLCLK
            {
                show_panel(hwnd);
            }
            0
        }
        SHOW_MSG => {
            show_panel(hwnd);
            0
        }
        REFRESH_MSG => {
            apply_view();
            0
        }
        WM_TIMER if wparam == 2 => {
            InvalidateRect(hwnd, std::ptr::null(), FALSE);
            0
        }
        WM_TIMER => {
            let motion_done = if let Ok(mut motion) = MOTION.lock() {
                if motion.tick < MOTION_FRAMES {
                    motion.tick += 1;
                }
                motion.tick >= MOTION_FRAMES
            } else {
                true
            };
            let count_done = if let Ok(mut count) = COUNT.lock() {
                if count.to >= 0 && count.tick < COUNT_FRAMES {
                    count.tick += 1;
                }
                count.to < 0 || count.tick >= COUNT_FRAMES
            } else {
                true
            };
            if !motion_done {
                place(hwnd);
            }
            InvalidateRect(hwnd, std::ptr::null(), FALSE);
            if motion_done && count_done {
                KillTimer(hwnd, 1);
            }
            0
        }
        WM_SETTINGCHANGE => {
            apply_view();
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_MOUSEMOVE => {
            let (x, y) = mouse_xy(lparam);
            let hit = hit_at(hwnd, x, y);
            if HOVER.swap(hit, Ordering::Relaxed) != hit {
                InvalidateRect(hwnd, std::ptr::null(), FALSE);
            }
            if !TRACKING.swap(true, Ordering::Relaxed) {
                let mut event = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut event);
            }
            0
        }
        WM_MOUSELEAVE => {
            TRACKING.store(false, Ordering::Relaxed);
            HOVER.store(0, Ordering::Relaxed);
            PRESSED.store(0, Ordering::Relaxed);
            InvalidateRect(hwnd, std::ptr::null(), FALSE);
            0
        }
        WM_LBUTTONDOWN => {
            let (x, y) = mouse_xy(lparam);
            PRESSED.store(hit_at(hwnd, x, y), Ordering::Relaxed);
            SetCapture(hwnd);
            0
        }
        WM_LBUTTONUP => {
            ReleaseCapture();
            let (x, y) = mouse_xy(lparam);
            let hit = hit_at(hwnd, x, y);
            let pressed = PRESSED.swap(0, Ordering::Relaxed);
            if hit != 0 && hit == pressed {
                activate(hwnd, hit);
            }
            0
        }
        WM_SETCURSOR => {
            if HOVER.load(Ordering::Relaxed) != 0 {
                SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_HAND));
                1
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
            hide_panel(hwnd);
            0
        }
        WM_ACTIVATE if (wparam as u32 & 0xffff) == WA_INACTIVE as u32 => {
            if IsWindowVisible(hwnd) != 0 {
                hide_panel(hwnd);
            }
            0
        }
        WM_CLOSE => {
            hide_panel(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        0x02E0 => {
            place(hwnd);
            InvalidateRect(hwnd, std::ptr::null(), FALSE);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn activate(hwnd: HWND, hit: u8) {
    match hit {
        HIT_CLOSE => unsafe {
            hide_panel(hwnd);
        },
        HIT_REFRESH => request_refresh(),
        HIT_SETTINGS => {
            SETTINGS_OPEN.store(true, Ordering::Relaxed);
            spawn_distro_reload();
            relayout(hwnd);
        }
        ROW_DONE => {
            SETTINGS_OPEN.store(false, Ordering::Relaxed);
            relayout(hwnd);
        }
        ROW_SOURCE => {
            update_config(|cfg| {
                cfg.source = match cfg.source {
                    crate::settings::Source::Windows => crate::settings::Source::Wsl,
                    crate::settings::Source::Wsl => crate::settings::Source::Windows,
                };
            });
            if config().source == crate::settings::Source::Wsl {
                spawn_distro_reload();
            }
            drop_cached_number();
            request_refresh();
            relayout(hwnd);
        }
        ROW_DISTRO => {
            let names = distro_names();
            if !names.is_empty() {
                update_config(|cfg| {
                    let index = names
                        .iter()
                        .position(|name| name == &cfg.distro)
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    cfg.distro = names[index % names.len()].clone();
                });
                drop_cached_number();
                request_refresh();
            }
            relayout(hwnd);
        }
        ROW_FOCUS => {
            update_config(|cfg| {
                cfg.focus = match cfg.focus {
                    crate::settings::FocusPref::Auto => crate::settings::FocusPref::Session,
                    crate::settings::FocusPref::Session => crate::settings::FocusPref::Week,
                    crate::settings::FocusPref::Week => crate::settings::FocusPref::Auto,
                };
            });
            unsafe {
                apply_view();
            }
        }
        ROW_START => {
            update_config(|cfg| {
                cfg.autostart = match cfg.autostart {
                    crate::settings::AutostartPref::Off => crate::settings::AutostartPref::On,
                    _ => crate::settings::AutostartPref::Off,
                };
            });
            apply_autostart(config().autostart);
            relayout(hwnd);
        }
        ROW_THEME => {
            update_config(|cfg| {
                cfg.theme = match cfg.theme {
                    crate::settings::ThemePref::System => crate::settings::ThemePref::Dark,
                    crate::settings::ThemePref::Dark => crate::settings::ThemePref::Light,
                    crate::settings::ThemePref::Light => crate::settings::ThemePref::System,
                };
            });
            unsafe {
                apply_view();
            }
        }
        ROW_PATH => {
            if !config().codex_home.is_empty() {
                update_config(|cfg| cfg.codex_home.clear());
                drop_cached_number();
                request_refresh();
            }
            relayout(hwnd);
        }
        HIT_QUIT => unsafe {
            mark_trust_seen();
            QUIT_FLAG.store(true, Ordering::Relaxed);
            if let Some(tx) = WAKE.get() {
                let _ = tx.send(());
            }
            remove_tray(hwnd);
            PostQuitMessage(0);
        },
        _ => {}
    }
}

fn relayout(hwnd: HWND) {
    unsafe {
        place(hwnd);
        InvalidateRect(hwnd, std::ptr::null(), FALSE);
    }
}

fn hit_at(hwnd: HWND, x: i32, y: i32) -> u8 {
    let Ok(view) = VIEW.lock() else {
        return 0;
    };
    let card = card_for(&view, window_dpi(hwnd));
    if card.close.contains(x, y) {
        return HIT_CLOSE;
    }
    if !SETTINGS_OPEN.load(Ordering::Relaxed) && card.gear.w > 0 && card.gear.contains(x, y) {
        return HIT_SETTINGS;
    }
    for row in &card.rows {
        if row.spot.contains(x, y) {
            return row.id;
        }
    }
    if card.refresh.w > 0 && card.refresh.contains(x, y) {
        HIT_REFRESH
    } else if card.quit.contains(x, y) {
        HIT_QUIT
    } else {
        0
    }
}

unsafe fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let view = VIEW
        .lock()
        .ok()
        .map(|view| view.clone())
        .unwrap_or(View::Loading);
    let card = card_for(&view, window_dpi(hwnd));
    let fonts = fonts_for(window_dpi(hwnd));
    let palette = palette_now();
    blend_round(
        hdc,
        Spot {
            x: 0,
            y: 0,
            w: card.width,
            h: card.height,
        },
        px(&card, 26),
        palette.plate,
        true,
    );
    paint_card(hdc, &card, &fonts, &palette);
    EndPaint(hwnd, &ps);
}

unsafe fn paint_card(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    card: &Card,
    fonts: &Fonts,
    palette: &Palette,
) {
    let hover = HOVER.load(Ordering::Relaxed);
    let left = DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX;
    let shown = animated_remaining(card.remaining);
    let accent = rgb_color(usage::mood_rgb_on(Some(shown), palette.light));
    if hover == HIT_CLOSE {
        fill_round(hdc, card.close, card.close.h / 2, palette.soft);
    }
    SetTextCharacterExtra(hdc, px(card, 2));
    text_in(
        hdc,
        fonts.brand,
        palette.ink,
        card.brand_box.rect(),
        "JUSAGE",
        left,
    );
    SetTextCharacterExtra(hdc, 0);
    text_in(
        hdc,
        fonts.quiet,
        palette.muted,
        rect(
            card.brand_box.x,
            card.brand_box.y + card.brand_box.h + px(card, 1),
            px(card, 88),
            px(card, 16),
        ),
        "Codex",
        left,
    );
    if let Some(spot) = card.trust_box {
        text_in(
            hdc,
            fonts.body,
            palette.muted,
            spot.rect(),
            &card.trust,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
    if !card.plan.is_empty() {
        text_in(
            hdc,
            fonts.quiet,
            palette.muted,
            card.plan_box.rect(),
            &card.plan,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }
    text_in(
        hdc,
        fonts.brand,
        palette.muted,
        card.close.rect(),
        "×",
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    if card.rows.is_empty() && card.gear.w > 0 {
        if hover == HIT_SETTINGS {
            fill_round(hdc, card.gear, card.gear.h / 2, palette.soft);
        }
        text_in(
            hdc,
            fonts.label,
            palette.ink,
            card.gear.rect(),
            "설정",
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    if let Some(rule) = card.top_rule {
        blend_round(hdc, rule, 0, palette.line, false);
    }
    if let Some(scope) = card.scope_box {
        text_in(
            hdc,
            fonts.quiet,
            palette.muted,
            scope.rect(),
            &card.scope,
            left,
        );
    }
    if let Some(rail) = card.rail {
        let [red, green, blue] = color_rgb(accent);
        blend_round(hdc, rail, rail.w, [red, green, blue, 255], false);
    }
    if card.hero_big {
        paint_meter(hdc, card, fonts, accent);
    } else {
        text_in(
            hdc,
            fonts.lead,
            palette.ink,
            card.hero_box.rect(),
            &card.hero,
            left,
        );
    }
    if let Some(mood) = card.mood_box {
        text_in(
            hdc,
            fonts.label,
            accent,
            mood.rect(),
            usage::mood_word(shown),
            DT_RIGHT | DT_BOTTOM | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    if let Some(spot) = card.bar {
        paint_capsule(
            hdc,
            spot,
            shown as f32 / 100.0,
            color_rgb(accent),
            palette.track,
        );
    }
    if let Some(spot) = card.note_box {
        text_in(
            hdc,
            fonts.body,
            palette.muted,
            spot.rect(),
            &card.note,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
    if let Some(spot) = card.charge_box {
        text_in(
            hdc,
            fonts.lead,
            palette.ink,
            spot.rect(),
            &card.charge,
            left,
        );
    }
    if let Some(spot) = card.refill_box {
        text_in(
            hdc,
            fonts.quiet,
            palette.muted,
            spot.rect(),
            &card.refill,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }
    if let Some(spot) = card.charge_bar {
        let gray = if palette.light {
            [140, 138, 132]
        } else {
            [206, 206, 202]
        };
        paint_capsule(hdc, spot, card.charge_fill, gray, palette.track);
    }

    for tile in &card.tiles {
        let radius = px(card, 14);
        let border = if tile.hot {
            let [r, g, b] = color_rgb(tile.accent);
            [r, g, b, 170]
        } else {
            palette.line
        };
        stroke_panel(
            hdc,
            tile.spot,
            radius,
            palette.tile,
            border,
            px(card, 1).max(2),
        );
        let inner_x = tile.spot.x + px(card, 14);
        let inner_w = tile.spot.w - px(card, 28);
        text_in(
            hdc,
            fonts.label,
            palette.muted,
            rect(inner_x, tile.spot.y + px(card, 12), inner_w, px(card, 22)),
            &tile.label,
            left,
        );
        text_in(
            hdc,
            fonts.figure,
            palette.ink,
            rect(inner_x, tile.spot.y + px(card, 34), inner_w, px(card, 30)),
            &tile.value,
            left,
        );
        text_in(
            hdc,
            fonts.body,
            tile.accent,
            rect(inner_x, tile.spot.y + px(card, 66), inner_w, px(card, 22)),
            &tile.mood,
            left,
        );
        if !tile.clock.is_empty() {
            text_in(
                hdc,
                fonts.quiet,
                palette.muted,
                rect(inner_x, tile.spot.y + px(card, 88), inner_w, px(card, 18)),
                &tile.clock,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
        }
        let percent = tile.value.trim_end_matches('%').parse::<u32>().unwrap_or(0);
        paint_capsule(
            hdc,
            Spot {
                x: inner_x,
                y: tile.spot.y + tile.spot.h - px(card, 18),
                w: inner_w,
                h: px(card, 6),
            },
            percent as f32 / 100.0,
            color_rgb(tile.accent),
            palette.track,
        );
    }
    if let Some(spot) = card.pace_caption_box {
        text_in(
            hdc,
            fonts.quiet,
            palette.muted,
            spot.rect(),
            "이 속도",
            left,
        );
    }
    if let Some(spot) = card.pace_box {
        text_in(
            hdc,
            fonts.body,
            palette.ink,
            spot.rect(),
            &card.pace,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
    if let Some(spot) = card.cycle_box {
        text_in(
            hdc,
            fonts.body,
            palette.ink,
            spot.rect(),
            &card.cycle,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
    if let Some(rule) = card.ledger_rule {
        blend_round(hdc, rule, 0, palette.line, false);
    }
    for fact in &card.facts {
        let label_flags = if fact.wrap {
            DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX
        } else {
            left
        };
        let value_flags = if fact.wrap {
            DT_RIGHT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX
        } else {
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX
        };
        text_in(
            hdc,
            fonts.label,
            palette.muted,
            fact.label.rect(),
            &fact.name,
            label_flags,
        );
        text_in(
            hdc,
            fonts.lead,
            palette.ink,
            fact.value.rect(),
            &fact.text,
            value_flags,
        );
        if let Some(rule) = fact.rule {
            blend_round(hdc, rule, 0, palette.line, false);
        }
    }
    if let Some(rule) = card.days_rule {
        blend_round(hdc, rule, 0, palette.line, false);
    }
    if let Some(spot) = card.days_label {
        text_in(
            hdc,
            fonts.quiet,
            palette.muted,
            spot.rect(),
            "최근 7일",
            left,
        );
    }
    if let Some(spot) = card.days_open_label {
        text_in(hdc, fonts.quiet, palette.muted, spot.rect(), "처음", left);
    }
    if let Some(spot) = card.days_close_label {
        text_in(hdc, fonts.quiet, palette.muted, spot.rect(), "나중", left);
    }
    let center = DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX;
    for col in &card.days {
        let name_color = if col.today { accent } else { palette.muted };
        text_in(
            hdc,
            fonts.label,
            name_color,
            col.name_box.rect(),
            &col.name,
            center,
        );
        if let Some(tick) = col.tick {
            let [red, green, blue] = color_rgb(accent);
            blend_round(hdc, tick, tick.h, [red, green, blue, 255], false);
        }
        text_in(
            hdc,
            fonts.digit,
            palette.muted,
            col.open_box.rect(),
            &col.open,
            center,
        );
        let close_color = if col.close == "·" {
            palette.muted
        } else if col.today {
            accent
        } else {
            palette.ink
        };
        text_in(
            hdc,
            fonts.digit,
            close_color,
            col.close_box.rect(),
            &col.close,
            center,
        );
    }
    if let Some(spot) = card.story_box {
        text_in(
            hdc,
            fonts.body,
            palette.ink,
            spot.rect(),
            &card.story,
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
    if let Some(spot) = card.status_box {
        text_in(
            hdc,
            fonts.quiet,
            palette.muted,
            spot.rect(),
            &card.status,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }
    for row in &card.rows {
        let hot = hover == row.id;
        let fill = if hot {
            if palette.light {
                [23, 23, 23, 16]
            } else {
                [255, 255, 255, 24]
            }
        } else {
            palette.tile
        };
        let line = if palette.light {
            [23, 23, 23, 48]
        } else {
            palette.line
        };
        stroke_panel(hdc, row.spot, px(card, 12), fill, line, px(card, 1).max(1));
        let label_w = row.spot.w * 42 / 100;
        text_in(
            hdc,
            fonts.label,
            palette.muted,
            rect(row.spot.x + px(card, 14), row.spot.y, label_w, row.spot.h),
            &row.label,
            left,
        );
        text_in(
            hdc,
            fonts.lead,
            palette.ink,
            rect(
                row.spot.x + label_w,
                row.spot.y,
                row.spot.w - label_w - px(card, 14),
                row.spot.h,
            ),
            &row.value,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }
    if let Some(rule) = card.bottom_rule {
        blend_round(hdc, rule, 0, palette.line, false);
    }
    if card.refresh.w > 0 {
        let refresh_fill = if hover == HIT_REFRESH {
            if palette.light {
                [23, 23, 23, 18]
            } else {
                [255, 255, 255, 28]
            }
        } else if palette.light {
            [255, 255, 255, 28]
        } else {
            [255, 255, 255, 10]
        };
        let refresh_line = if palette.light {
            [23, 23, 23, 72]
        } else {
            [236, 236, 232, 80]
        };
        stroke_panel(
            hdc,
            card.refresh,
            card.refresh.h / 2,
            refresh_fill,
            refresh_line,
            px(card, 1).max(1),
        );
        text_in(
            hdc,
            fonts.label,
            palette.ink,
            card.refresh.rect(),
            "다시 확인",
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    text_in(
        hdc,
        fonts.quiet,
        if hover == HIT_QUIT {
            palette.ink
        } else {
            palette.muted
        },
        card.quit.rect(),
        "끝내기",
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
}

unsafe fn paint_meter(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    card: &Card,
    fonts: &Fonts,
    accent: u32,
) {
    let shown = animated_remaining(card.remaining);
    SHOWN.store(shown as i32, Ordering::Relaxed);
    let digits = format!("{shown}");
    let width = text_size(hdc, fonts.hero, &digits).cx;
    text_in(
        hdc,
        fonts.hero,
        accent,
        card.hero_box.rect(),
        &digits,
        DT_LEFT | DT_BOTTOM | DT_SINGLELINE | DT_NOPREFIX,
    );
    let unit_h = px(card, 28);
    text_in(
        hdc,
        fonts.unit,
        accent,
        rect(
            card.hero_box.x + width + px(card, 2),
            card.hero_box.y + card.hero_box.h - unit_h,
            px(card, 28),
            unit_h,
        ),
        "%",
        DT_LEFT | DT_BOTTOM | DT_SINGLELINE | DT_NOPREFIX,
    );
}

unsafe fn paint_capsule(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    spot: Spot,
    fraction: f32,
    fill: [u8; 3],
    track: [u8; 4],
) {
    if spot.w <= 2 || spot.h <= 2 {
        return;
    }
    let width = spot.w as u32;
    let height = spot.h as u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let radius = height as f32 / 2.0;
    let fill_w = (width as f32) * fraction.clamp(0.0, 1.0);
    for y in 0..height {
        for x in 0..width {
            let cover = round_coverage(x as f32, y as f32, width as f32, height as f32, radius);
            if cover <= 0.0 {
                continue;
            }
            let (red, green, blue, alpha) = if (x as f32 + 0.5) < fill_w {
                let lift = if y * 3 < height { 32 } else { 0 };
                (
                    fill[0].saturating_add(lift),
                    fill[1].saturating_add(lift),
                    fill[2].saturating_add(lift),
                    255u8,
                )
            } else {
                (track[0], track[1], track[2], track[3])
            };
            let alpha = ((alpha as f32) * cover.min(1.0)) as u8;
            let index = ((y * width + x) * 4) as usize;
            pixels[index] = premul(blue, alpha);
            pixels[index + 1] = premul(green, alpha);
            pixels[index + 2] = premul(red, alpha);
            pixels[index + 3] = alpha;
        }
    }
    blend_pixels(hdc, spot, &pixels);
}

unsafe fn stroke_panel(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    spot: Spot,
    radius: i32,
    fill: [u8; 4],
    line: [u8; 4],
    inset: i32,
) {
    blend_round(hdc, spot, radius, line, false);
    if spot.w <= inset * 2 + 4 || spot.h <= inset * 2 + 4 {
        return;
    }
    blend_round(
        hdc,
        Spot {
            x: spot.x + inset,
            y: spot.y + inset,
            w: spot.w - inset * 2,
            h: spot.h - inset * 2,
        },
        (radius - inset).max(0),
        fill,
        false,
    );
}

fn assign_window_icon(hwnd: HWND, icon: HICON) {
    if hwnd.is_null() || icon.is_null() {
        return;
    }
    unsafe {
        SendMessageW(hwnd, WM_SETICON, 0, icon as LPARAM);
        SendMessageW(hwnd, WM_SETICON, 1, icon as LPARAM);
    }
}

unsafe fn hide_panel(hwnd: HWND) {
    mark_trust_seen();
    ShowWindow(hwnd, SW_HIDE);
    KillTimer(hwnd, 1);
    KillTimer(hwnd, 2);
}

fn px(card: &Card, logical: i32) -> i32 {
    if card.width <= 0 {
        logical
    } else {
        logical * card.width / CARD_W
    }
}

unsafe fn fonts_for(dpi: i32) -> Fonts {
    let Ok(mut slot) = FONTS.lock() else {
        return make_fonts(dpi);
    };
    if let Some(fonts) = slot.as_ref() {
        if fonts.dpi == dpi {
            return Fonts {
                dpi: fonts.dpi,
                brand: fonts.brand,
                hero: fonts.hero,
                unit: fonts.unit,
                figure: fonts.figure,
                lead: fonts.lead,
                label: fonts.label,
                body: fonts.body,
                quiet: fonts.quiet,
                digit: fonts.digit,
            };
        }
        DeleteObject(fonts.brand as HGDIOBJ);
        DeleteObject(fonts.hero as HGDIOBJ);
        DeleteObject(fonts.unit as HGDIOBJ);
        DeleteObject(fonts.figure as HGDIOBJ);
        DeleteObject(fonts.lead as HGDIOBJ);
        DeleteObject(fonts.label as HGDIOBJ);
        DeleteObject(fonts.body as HGDIOBJ);
        DeleteObject(fonts.quiet as HGDIOBJ);
        DeleteObject(fonts.digit as HGDIOBJ);
    }
    let fonts = make_fonts(dpi);
    *slot = Some(Fonts {
        dpi: fonts.dpi,
        brand: fonts.brand,
        hero: fonts.hero,
        unit: fonts.unit,
        figure: fonts.figure,
        lead: fonts.lead,
        label: fonts.label,
        body: fonts.body,
        quiet: fonts.quiet,
        digit: fonts.digit,
    });
    fonts
}

unsafe fn make_fonts(dpi: i32) -> Fonts {
    Fonts {
        dpi,
        brand: font(scale(12, dpi), FW_SEMIBOLD, "Cascadia Mono"),
        hero: font(scale(64, dpi), FW_SEMIBOLD, "Cascadia Mono"),
        unit: font(scale(20, dpi), FW_SEMIBOLD, "Cascadia Mono"),
        figure: font(scale(22, dpi), FW_SEMIBOLD, "Cascadia Mono"),
        lead: font(scale(16, dpi), FW_SEMIBOLD, "Segoe UI"),
        label: font(scale(14, dpi), FW_SEMIBOLD, "Segoe UI"),
        body: font(scale(15, dpi), FW_NORMAL, "Segoe UI"),
        quiet: font(scale(13, dpi), FW_NORMAL, "Segoe UI"),
        digit: font(scale(15, dpi), FW_SEMIBOLD, "Cascadia Mono"),
    }
}

unsafe fn font(height: i32, weight: u32, face_name: &str) -> HFONT {
    let face = wide(face_name);
    CreateFontW(
        -height,
        0,
        0,
        0,
        weight as i32,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        face.as_ptr(),
    )
}

unsafe fn text_size(hdc: windows_sys::Win32::Graphics::Gdi::HDC, font: HFONT, value: &str) -> SIZE {
    let old = SelectObject(hdc, font as HGDIOBJ);
    let text = wide(value);
    let mut size = SIZE { cx: 0, cy: 0 };
    let chars = text.len().saturating_sub(1) as i32;
    if chars > 0 {
        GetTextExtentPoint32W(hdc, text.as_ptr(), chars, &mut size);
    }
    SelectObject(hdc, old);
    size
}

unsafe fn text_in(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    font: HFONT,
    color: u32,
    mut area: RECT,
    value: &str,
    format: u32,
) {
    let old = SelectObject(hdc, font as HGDIOBJ);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color as COLORREF);
    let text = wide(value);
    DrawTextW(hdc, text.as_ptr(), -1, &mut area, format);
    SelectObject(hdc, old);
}

unsafe fn fill_round(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    spot: Spot,
    radius: i32,
    color: u32,
) {
    let brush = CreateSolidBrush(color as COLORREF);
    let pen = CreatePen(PS_SOLID, 1, color as COLORREF);
    let old_brush = SelectObject(hdc, brush as HGDIOBJ);
    let old_pen = SelectObject(hdc, pen as HGDIOBJ);
    RoundRect(
        hdc,
        spot.x,
        spot.y,
        spot.x + spot.w,
        spot.y + spot.h,
        radius,
        radius,
    );
    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(brush as HGDIOBJ);
    DeleteObject(pen as HGDIOBJ);
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}

fn mouse_xy(lparam: LPARAM) -> (i32, i32) {
    ((lparam as i16) as i32, ((lparam >> 16) as i16) as i32)
}

fn window_dpi(hwnd: HWND) -> i32 {
    let dpi = unsafe { GetDpiForWindow(hwnd) } as i32;
    if dpi > 0 {
        dpi
    } else {
        unsafe { screen_dpi() }
    }
}

unsafe fn add_tray(hwnd: HWND, icon: HICON, tip: &str) {
    let data = tray_data(hwnd, icon, tip);
    Shell_NotifyIconW(NIM_ADD, &data);
}

unsafe fn update_tray(hwnd: HWND, icon: HICON, tip: &str) {
    let data = tray_data(hwnd, icon, tip);
    Shell_NotifyIconW(NIM_MODIFY, &data);
}

unsafe fn remove_tray(hwnd: HWND) {
    let mut data = NOTIFYICONDATAW::default();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = ICON_ID;
    Shell_NotifyIconW(NIM_DELETE, &data);
}

unsafe fn tray_data(hwnd: HWND, icon: HICON, tip: &str) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW::default();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = ICON_ID;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = TRAY_MSG;
    data.hIcon = icon;
    let text = wide(tip);
    let count = text.len().min(data.szTip.len());
    data.szTip[..count].copy_from_slice(&text[..count]);
    data
}

unsafe fn make_icon(size: u32, remaining: u32, color: [u8; 3], show_number: bool) -> HICON {
    let mut pixels = remaining_icon(size, remaining, color, false);
    if show_number && !draw_font_number(&mut pixels, size, remaining) {
        pixels = remaining_icon(size, remaining, color, true);
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            biHeight: -(size as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };
    let hdc = GetDC(std::ptr::null_mut());
    let mut bits = std::ptr::null_mut();
    let color = CreateDIBSection(
        hdc,
        &bmi,
        DIB_RGB_COLORS,
        &mut bits,
        std::ptr::null_mut(),
        0,
    );
    ReleaseDC(std::ptr::null_mut(), hdc);
    if color.is_null() || bits.is_null() {
        return std::ptr::null_mut();
    }
    let dest = std::slice::from_raw_parts_mut(bits as *mut u8, pixels.len());
    for index in (0..pixels.len()).step_by(4) {
        dest[index] = pixels[index + 2];
        dest[index + 1] = pixels[index + 1];
        dest[index + 2] = pixels[index];
        dest[index + 3] = pixels[index + 3];
    }
    let mask = CreateBitmap(size as i32, size as i32, 1, 1, std::ptr::null());
    let info = ICONINFO {
        fIcon: TRUE,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask,
        hbmColor: color,
    };
    let icon = CreateIconIndirect(&info);
    let _ = DeleteObject(color as HGDIOBJ);
    if !mask.is_null() {
        let _ = DeleteObject(mask as HGDIOBJ);
    }
    icon
}

unsafe fn draw_font_number(pixels: &mut [u8], size: u32, number: u32) -> bool {
    let text = number.to_string();
    let wide_text = wide(&text);
    let chars = wide_text.len().saturating_sub(1) as i32;
    if chars <= 0 {
        return false;
    }
    let center = (size as f32 - 1.0) / 2.0;
    let hole = (((center - 0.15) * 0.80) * 2.0).round() as i32;
    let max_w = (hole as f32 * 0.82) as i32;
    let max_h = (hole as f32 * 0.72) as i32;
    if max_w < 8 || max_h < 8 {
        return false;
    }
    let screen = GetDC(std::ptr::null_mut());
    if screen.is_null() {
        return false;
    }
    let mut height = max_h;
    let mut chosen = std::ptr::null_mut();
    while height >= 8 {
        let face = wide("Cascadia Mono");
        let font = CreateFontW(
            -height,
            0,
            0,
            0,
            FW_SEMIBOLD as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            4,
            0,
            face.as_ptr(),
        );
        if font.is_null() {
            height -= 1;
            continue;
        }
        let old = SelectObject(screen, font as HGDIOBJ);
        let mut extent = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(screen, wide_text.as_ptr(), chars, &mut extent);
        SelectObject(screen, old);
        if extent.cx > 0 && extent.cx <= max_w && extent.cy <= max_h {
            chosen = font;
            break;
        }
        let _ = DeleteObject(font as HGDIOBJ);
        height -= 1;
    }
    ReleaseDC(std::ptr::null_mut(), screen);
    if chosen.is_null() {
        return false;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            biHeight: -(size as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };
    let screen = GetDC(std::ptr::null_mut());
    let mut bits = std::ptr::null_mut();
    let dib = CreateDIBSection(
        screen,
        &bmi,
        DIB_RGB_COLORS,
        &mut bits,
        std::ptr::null_mut(),
        0,
    );
    let mem = CreateCompatibleDC(screen);
    ReleaseDC(std::ptr::null_mut(), screen);
    if dib.is_null() || bits.is_null() || mem.is_null() {
        let _ = DeleteObject(chosen as HGDIOBJ);
        if !dib.is_null() {
            let _ = DeleteObject(dib as HGDIOBJ);
        }
        if !mem.is_null() {
            DeleteDC(mem);
        }
        return false;
    }
    std::ptr::write_bytes(bits, 0, (size * size * 4) as usize);
    let old_bmp = SelectObject(mem, dib as HGDIOBJ);
    let old_font = SelectObject(mem, chosen as HGDIOBJ);
    SetBkMode(mem, TRANSPARENT as i32);
    SetTextColor(mem, 0x00FF_FFFF);
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: size as i32,
        bottom: size as i32,
    };
    DrawTextW(
        mem,
        wide_text.as_ptr(),
        chars,
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    let count = (size * size) as usize;
    let raw = std::slice::from_raw_parts(bits as *const u8, count * 4);
    let mut cover = vec![0u8; count];
    for index in 0..count {
        let blue = raw[index * 4] as u16;
        let green = raw[index * 4 + 1] as u16;
        let red = raw[index * 4 + 2] as u16;
        cover[index] = ((red + green + blue) / 3).min(255) as u8;
    }
    SelectObject(mem, old_font);
    SelectObject(mem, old_bmp);
    let _ = DeleteObject(chosen as HGDIOBJ);
    let _ = DeleteObject(dib as HGDIOBJ);
    DeleteDC(mem);
    if cover.iter().all(|value| *value == 0) {
        return false;
    }
    let side = size as i32;
    let mut halo = vec![0u8; count];
    for y in 0..side {
        for x in 0..side {
            let mut best = 0u8;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let xx = x + dx;
                    let yy = y + dy;
                    if xx < 0 || yy < 0 || xx >= side || yy >= side {
                        continue;
                    }
                    best = best.max(cover[(yy * side + xx) as usize]);
                }
            }
            halo[(y * side + x) as usize] = best;
        }
    }
    for y in 0..side {
        for x in 0..side {
            let alpha = halo[(y * side + x) as usize];
            if alpha > 0 {
                over_pixel(pixels, size, x as u32, y as u32, [28, 32, 34], alpha);
            }
        }
    }
    for y in 0..side {
        for x in 0..side {
            let alpha = cover[(y * side + x) as usize];
            if alpha > 0 {
                over_pixel(pixels, size, x as u32, y as u32, [248, 248, 246], alpha);
            }
        }
    }
    true
}

fn over_pixel(pixels: &mut [u8], size: u32, x: u32, y: u32, color: [u8; 3], alpha: u8) {
    if alpha == 0 || x >= size || y >= size {
        return;
    }
    let index = ((y * size + x) * 4) as usize;
    let alpha = alpha as u16;
    let inverse = 255 - alpha;
    for channel in 0..3 {
        let ink = color[channel] as u16 * alpha / 255;
        let under = pixels[index + channel] as u16;
        pixels[index + channel] = (ink + under * inverse / 255) as u8;
    }
    let under = pixels[index + 3] as u16;
    pixels[index + 3] = (alpha + under * inverse / 255) as u8;
}

fn scale(px: i32, dpi: i32) -> i32 {
    px * dpi / 96
}

unsafe fn screen_dpi() -> i32 {
    let hdc = GetDC(std::ptr::null_mut());
    let dpi = GetDeviceCaps(hdc, LOGPIXELSX as i32);
    ReleaseDC(std::ptr::null_mut(), hdc);
    if dpi <= 0 {
        96
    } else {
        dpi
    }
}

fn write_state(
    kind: &str,
    left: Option<u32>,
    plan: &str,
    fail_code: Option<u16>,
    focus: &str,
    week_left: Option<u32>,
) {
    let plan: String = plan
        .chars()
        .filter(|ch| !ch.is_control())
        .take(40)
        .collect();
    let focus: String = focus
        .chars()
        .filter(|ch| !ch.is_control())
        .take(20)
        .collect();
    let body = if kind == "ready" {
        let week = week_left
            .map(|value| format!("week={value}\n"))
            .unwrap_or_default();
        format!(
            "state=ready\nleft={}\nplan={plan}\nfocus={focus}\n{week}",
            left.unwrap_or(0)
        )
    } else if let Some(code) = fail_code {
        format!("state={kind}\ncode={code}\n")
    } else {
        format!("state={kind}\n")
    };
    let Some(path) = data_file("state.txt") else {
        return;
    };
    let _ = std::fs::write(path, body);
}

fn legacy_beside(name: &str) -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(name))
}

/// User data lives under LocalAppData. An older file beside the exe is copied once and left in place.
fn data_file(name: &str) -> Option<std::path::PathBuf> {
    let preferred = std::env::var("LOCALAPPDATA").ok().and_then(|local| {
        let dir = crate::settings::data_dir(std::path::Path::new(&local));
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir.join(name))
    });
    if let Some(path) = preferred {
        if !path.exists() {
            if let Some(old) = legacy_beside(name) {
                if old.is_file() && old != path {
                    let _ = std::fs::copy(&old, &path);
                }
            }
        }
        return Some(path);
    }
    legacy_beside(name)
}

fn settle_home() {
    let Ok(current) = std::env::current_exe() else {
        return;
    };
    let Ok(local) = std::env::var("LOCALAPPDATA") else {
        return;
    };
    let home = usage::home_exe(std::path::Path::new(&local));
    if usage::already_home(&current, &home) {
        return;
    }
    if let Some(parent) = home.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    if std::fs::copy(&current, &home).is_err() {
        return;
    }
    if std::process::Command::new(&home).spawn().is_ok() {
        std::process::exit(0);
    }
}

fn trust_path() -> Option<std::path::PathBuf> {
    data_file("trust.txt")
}

fn trust_seen() -> bool {
    if TRUST_LOADED.load(Ordering::Relaxed) {
        return TRUST_SEEN.load(Ordering::Relaxed);
    }
    let seen = trust_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .is_some_and(|text| text.lines().any(|line| line.trim() == "seen=1"));
    TRUST_SEEN.store(seen, Ordering::Relaxed);
    TRUST_LOADED.store(true, Ordering::Relaxed);
    seen
}

fn mark_trust_seen() {
    TRUST_SEEN.store(true, Ordering::Relaxed);
    TRUST_LOADED.store(true, Ordering::Relaxed);
    let Some(path) = trust_path() else {
        return;
    };
    let _ = std::fs::write(path, "seen=1\n");
}

fn run_key_sub() -> Vec<u16> {
    wide(r"Software\Microsoft\Windows\CurrentVersion\Run")
}

fn forget_tray_left() {
    let sub = run_key_sub();
    let old_name = wide("TrayLeft");
    unsafe {
        let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, sub.as_ptr(), old_name.as_ptr());
    }
}

fn run_key_text() -> Option<String> {
    let sub = run_key_sub();
    let name = wide("Jusage");
    let mut bytes = [0u16; 520];
    let mut size = (bytes.len() * 2) as u32;
    let mut kind = 0u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            &mut kind,
            bytes.as_mut_ptr() as *mut c_void,
            &mut size,
        )
    };
    if status != 0 {
        return None;
    }
    let chars = (size as usize / 2).saturating_sub(1).min(bytes.len());
    let text = String::from_utf16_lossy(&bytes[..chars]);
    let text = text
        .trim_matches('"')
        .trim()
        .trim_end_matches('\u{0}')
        .to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn write_run_key() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let path = exe.to_string_lossy().to_string();
    if run_key_text().is_some_and(|have| have.eq_ignore_ascii_case(&path)) {
        return;
    }
    let stored = if path.contains(' ') {
        format!("\"{path}\"")
    } else {
        path
    };
    let sub = run_key_sub();
    let name = wide("Jusage");
    let value = wide(&stored);
    unsafe {
        let _ = RegSetKeyValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name.as_ptr(),
            REG_SZ,
            value.as_ptr() as *const c_void,
            (value.len() * 2) as u32,
        );
    }
}

fn delete_run_key() {
    let sub = run_key_sub();
    let name = wide("Jusage");
    unsafe {
        let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, sub.as_ptr(), name.as_ptr());
    }
}

fn settings_file_path() -> Option<std::path::PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    Some(crate::settings::settings_path(std::path::Path::new(&local)))
}

fn read_settings_file() -> Option<crate::settings::Settings> {
    let text = std::fs::read_to_string(settings_file_path()?).ok()?;
    Some(crate::settings::parse_settings(&text))
}

fn save_settings_file(settings: &crate::settings::Settings) -> bool {
    let Some(path) = settings_file_path() else {
        return false;
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    std::fs::write(path, crate::settings::format_settings(settings)).is_ok()
}

/// The first settings file only records whether startup is already on. It does not create the run key.
fn ensure_config() -> bool {
    if CONFIG.lock().ok().is_some_and(|slot| slot.is_some()) {
        return false;
    }
    let (settings, created) = if let Some(settings) = read_settings_file() {
        (settings, false)
    } else {
        let mut settings = crate::settings::Settings::default();
        settings.autostart = crate::settings::initial_autostart(run_key_text().is_some());
        let created = save_settings_file(&settings);
        (settings, created)
    };
    if let Ok(mut slot) = CONFIG.lock() {
        if slot.is_some() {
            return false;
        }
        *slot = Some(settings);
    }
    created
}

fn boot_autostart() {
    forget_tray_left();
    let created = ensure_config();
    if created {
        return;
    }
    match crate::settings::autostart_action(config().autostart) {
        crate::settings::AutostartAction::Leave => {}
        crate::settings::AutostartAction::Disable => delete_run_key(),
        crate::settings::AutostartAction::Enable => {
            if run_key_text().is_none() {
                write_run_key();
            }
        }
    }
}

fn apply_autostart(pref: crate::settings::AutostartPref) {
    match crate::settings::autostart_action(pref) {
        crate::settings::AutostartAction::Leave => {}
        crate::settings::AutostartAction::Enable => write_run_key(),
        crate::settings::AutostartAction::Disable => delete_run_key(),
    }
}

fn config() -> crate::settings::Settings {
    let _ = ensure_config();
    CONFIG
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_default()
}

fn update_config(mut change: impl FnMut(&mut crate::settings::Settings)) {
    let _ = ensure_config();
    let next = {
        let Ok(mut slot) = CONFIG.lock() else {
            return;
        };
        let settings = slot.get_or_insert_with(crate::settings::Settings::default);
        change(settings);
        settings.clone()
    };
    let _ = save_settings_file(&next);
}

fn source_signature() -> String {
    let cfg = config();
    let source = match cfg.source {
        crate::settings::Source::Windows => "windows",
        crate::settings::Source::Wsl => "wsl",
    };
    format!("{source}|{}|{}", cfg.distro, cfg.codex_home)
}

fn shown_focus(snapshot: &Snapshot) -> Reading {
    let choice = match config().focus {
        crate::settings::FocusPref::Session => "session",
        crate::settings::FocusPref::Week => "week",
        crate::settings::FocusPref::Auto => "auto",
    };
    usage::focus_choice(snapshot, choice)
}

fn theme_is_light() -> bool {
    match config().theme {
        crate::settings::ThemePref::Light => true,
        crate::settings::ThemePref::Dark => false,
        crate::settings::ThemePref::System => system_is_light(),
    }
}

fn drop_cached_number() {
    if let Ok(mut last) = LAST.lock() {
        *last = None;
    }
    if let Ok(mut sig) = LAST_SIG.lock() {
        sig.clear();
    }
    FRESH_AT.store(0, Ordering::Relaxed);
    clear_notice();
}

fn distro_names() -> Vec<String> {
    DISTROS
        .lock()
        .ok()
        .map(|slot| slot.clone())
        .unwrap_or_default()
}

fn spawn_distro_reload() {
    std::thread::spawn(|| {
        reload_distros();
        let hwnd = HWND_SLOT.load(Ordering::SeqCst) as HWND;
        if !hwnd.is_null() {
            unsafe {
                let _ = PostMessageW(hwnd, REFRESH_MSG, 0, 0);
            }
        }
    });
}

fn reload_distros() {
    let names = list_wsl_distros();
    if let Ok(mut slot) = DISTROS.lock() {
        *slot = names;
    }
}

fn list_wsl_distros() -> Vec<String> {
    crate::settings::parse_wsl_list(&command_bytes(&["-l", "-q"]))
}

fn command_bytes(args: &[&str]) -> Vec<u8> {
    let mut child = match std::process::Command::new("wsl.exe")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Vec::new(),
    };
    let mut stdout = child.stdout.take();
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let mut buf = Vec::new();
                if let Some(pipe) = stdout.as_mut() {
                    use std::io::Read;
                    let _ = pipe.read_to_end(&mut buf);
                }
                return buf;
            }
            Ok(None) if start.elapsed() < Duration::from_secs(8) => {
                std::thread::sleep(Duration::from_millis(40));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Vec::new();
            }
        }
    }
}

fn wake_distro(name: &str) {
    let Some(name) = crate::settings::safe_distro(name) else {
        return;
    };
    let _ = command_bytes(&["-d", &name, "--exec", "/bin/true"]);
}

enum WslProbe {
    Found(std::path::PathBuf),
    Reachable,
    Missing,
}

fn wsl_codex_in(distro: &str) -> WslProbe {
    let Some(distro) = crate::settings::safe_distro(distro) else {
        return WslProbe::Missing;
    };
    let root = std::path::PathBuf::from(format!(r"\\wsl$\{distro}"));
    if !root.is_dir() {
        return WslProbe::Missing;
    }
    let direct = root.join("root").join(".codex");
    if direct.join("auth.json").is_file() {
        return WslProbe::Found(direct);
    }
    let home = root.join("home");
    if let Ok(entries) = std::fs::read_dir(&home) {
        for entry in entries.flatten() {
            let candidate = entry.path().join(".codex");
            if candidate.join("auth.json").is_file() {
                return WslProbe::Found(candidate);
            }
        }
    }
    WslProbe::Reachable
}

fn find_wsl_codex(preferred: &str) -> Result<std::path::PathBuf, &'static str> {
    let names = if let Some(name) = crate::settings::safe_distro(preferred) {
        vec![name]
    } else {
        let listed = list_wsl_distros();
        if let Ok(mut slot) = DISTROS.lock() {
            *slot = listed.clone();
        }
        if listed.is_empty() {
            return Err("WSL 배포판을 찾지 못했어요. 설정에서 배포판을 골라 주세요.");
        }
        listed
    };
    let mut reachable = false;
    for name in &names {
        match wsl_codex_in(name) {
            WslProbe::Found(path) => {
                if preferred.is_empty() {
                    let chosen = name.clone();
                    update_config(|cfg| {
                        if cfg.distro.is_empty() {
                            cfg.distro = chosen.clone();
                        }
                    });
                }
                return Ok(path);
            }
            WslProbe::Reachable => reachable = true,
            WslProbe::Missing => {}
        }
    }
    if !reachable {
        if let Some(name) = names.first() {
            wake_distro(name);
            match wsl_codex_in(name) {
                WslProbe::Found(path) => return Ok(path),
                WslProbe::Reachable => return Err("WSL에서 Codex 로그인을 찾지 못했어요."),
                WslProbe::Missing => {}
            }
        }
        return Err("WSL에 접근하지 못했어요. WSL을 켠 뒤 다시 확인해 주세요.");
    }
    Err("WSL에서 Codex 로그인을 찾지 못했어요.")
}

fn current_home() -> Result<std::path::PathBuf, &'static str> {
    let cfg = config();
    if !cfg.codex_home.is_empty() {
        if cfg.codex_home.starts_with('/') {
            let distro = crate::settings::safe_distro(&cfg.distro)
                .ok_or("WSL 배포판을 찾지 못했어요. 설정에서 배포판을 골라 주세요.")?;
            let unc = crate::settings::linux_to_unc(&distro, &cfg.codex_home)
                .ok_or("경로를 읽지 못했어요.")?;
            let path = std::path::PathBuf::from(unc);
            if path.is_dir() {
                return Ok(path);
            }
            return Err("경로를 읽지 못했어요.");
        }
        let path = std::path::PathBuf::from(&cfg.codex_home);
        if path.is_dir() {
            return Ok(path);
        }
        return Err("경로를 읽지 못했어요.");
    }
    match cfg.source {
        crate::settings::Source::Windows => Ok(usage::codex_home()),
        crate::settings::Source::Wsl => find_wsl_codex(&cfg.distro),
    }
}

fn promote_loop() {
    std::thread::sleep(Duration::from_secs(2));
    for _ in 0..8 {
        if QUIT_FLAG.load(Ordering::Relaxed) {
            return;
        }
        promote_icon();
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn promote_icon() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let needle = exe
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if needle.is_empty() {
        return;
    }
    unsafe {
        let path = wide(r"Control Panel\NotifyIconSettings");
        let mut root: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            KEY_READ | KEY_ENUMERATE_SUB_KEYS,
            &mut root,
        ) != 0
        {
            return;
        }
        let mut index = 0u32;
        loop {
            let mut name = [0u16; 256];
            let mut name_len = name.len() as u32;
            let status = RegEnumKeyExW(
                root,
                index,
                name.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if status != 0 {
                break;
            }
            index += 1;
            let mut sub: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(
                root,
                name.as_ptr(),
                0,
                KEY_QUERY_VALUE | KEY_SET_VALUE,
                &mut sub,
            ) != 0
            {
                continue;
            }
            if exe_matches(sub, &needle) {
                let one = 1u32;
                let value_name = wide("IsPromoted");
                let _ = RegSetValueExW(
                    sub,
                    value_name.as_ptr(),
                    0,
                    REG_DWORD,
                    &one as *const u32 as *const u8,
                    4,
                );
            }
            RegCloseKey(sub);
        }
        RegCloseKey(root);
    }
}

unsafe fn exe_matches(key: HKEY, needle: &str) -> bool {
    let value_name = wide("ExecutablePath");
    let mut bytes = [0u16; 520];
    let mut size = (bytes.len() * 2) as u32;
    let mut kind = 0u32;
    if RegGetValueW(
        key,
        std::ptr::null(),
        value_name.as_ptr(),
        RRF_RT_REG_SZ,
        &mut kind,
        bytes.as_mut_ptr() as *mut c_void,
        &mut size,
    ) != 0
    {
        return false;
    }
    String::from_utf16_lossy(&bytes)
        .to_ascii_lowercase()
        .contains(needle)
}

fn motion_offset() -> i32 {
    let Ok(motion) = MOTION.lock() else {
        return 0;
    };
    if !motion.live || motion.tick >= MOTION_FRAMES {
        return 0;
    }
    let left = (MOTION_FRAMES - motion.tick) as i32;
    left * left * 18 / (MOTION_FRAMES as i32 * MOTION_FRAMES as i32)
}

fn icon_px() -> u32 {
    let small = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16) as u32;
    (small.saturating_mul(2)).clamp(40, 64)
}

fn palette_now() -> Palette {
    if theme_is_light() {
        Palette {
            light: true,
            ink: 0x00171717,
            muted: 0x005A5A5A,
            soft: 0x00E8E6E2,
            plate: [252, 250, 246, 196],
            tile: [255, 255, 255, 210],
            track: [222, 218, 210, 255],
            line: [40, 38, 34, 48],
        }
    } else {
        Palette {
            light: false,
            ink: 0x00F0F2F2,
            muted: 0x00A8A8A0,
            soft: 0x00323A42,
            plate: [14, 16, 20, 184],
            tile: [255, 255, 255, 16],
            track: [255, 255, 255, 36],
            line: [236, 236, 232, 48],
        }
    }
}

fn system_is_light() -> bool {
    let sub = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let name = wide("SystemUsesLightTheme");
    let mut value = 1u32;
    let mut size = 4u32;
    let mut kind = 0u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            &mut kind,
            &mut value as *mut u32 as *mut c_void,
            &mut size,
        )
    };
    status != 0 || value != 0
}

fn focus_left(view: &View) -> Option<i32> {
    match view {
        View::Ready(snapshot) => Some(shown_focus(snapshot).remaining as i32),
        _ => None,
    }
}

fn animated_remaining(target: u32) -> u32 {
    let Ok(count) = COUNT.lock() else {
        return target;
    };
    if count.to < 0 {
        return target;
    }
    usage::ease_count(count.from, count.to, count.tick, COUNT_FRAMES) as u32
}

fn arm_count(visible: bool) -> bool {
    let Some(view) = VIEW.lock().ok().map(|view| view.clone()) else {
        return false;
    };
    let Some(target) = focus_left(&view) else {
        if let Ok(mut count) = COUNT.lock() {
            count.to = -1;
            count.tick = COUNT_FRAMES;
        }
        return false;
    };
    if !visible {
        if let Ok(mut count) = COUNT.lock() {
            count.from = target;
            count.to = target;
            count.tick = COUNT_FRAMES;
        }
        return false;
    }
    let shown = SHOWN.load(Ordering::Relaxed);
    let current = COUNT
        .lock()
        .map(|count| {
            if count.to >= 0 {
                usage::ease_count(count.from, count.to, count.tick, COUNT_FRAMES)
            } else {
                shown
            }
        })
        .unwrap_or(shown);
    if current < 0 || current == target {
        if let Ok(mut count) = COUNT.lock() {
            count.from = target;
            count.to = target;
            count.tick = COUNT_FRAMES;
        }
        return false;
    }
    if let Ok(mut count) = COUNT.lock() {
        count.from = current;
        count.to = target;
        count.tick = 0;
    }
    true
}

fn begin_open_count() {
    let Some(view) = VIEW.lock().ok().map(|view| view.clone()) else {
        return;
    };
    let Some(target) = focus_left(&view) else {
        return;
    };
    let shown = SHOWN.load(Ordering::Relaxed);
    if let Ok(mut count) = COUNT.lock() {
        if shown >= 0 && shown != target {
            count.from = shown;
            count.to = target;
            count.tick = 0;
        } else {
            count.from = target;
            count.to = target;
            count.tick = COUNT_FRAMES;
        }
    }
}

fn color_rgb(color: u32) -> [u8; 3] {
    [
        (color & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        ((color >> 16) & 0xff) as u8,
    ]
}

fn clear_delta() {
    if let Ok(mut line) = DELTA.lock() {
        line.clear();
    }
}

fn note_change(snapshot: &Snapshot) {
    let focus = shown_focus(snapshot);
    let key = match focus.pane {
        Pane::Session => "session",
        Pane::Week => "week",
    };
    let now = now_unix();
    let today = local_civil(now)
        .map(|time| format!("{:04}-{:02}-{:02}", time.year, time.month, time.day))
        .unwrap_or_default();
    let previous = read_memory();
    let recent = previous.as_ref().and_then(|previous| {
        if previous.key != key {
            return None;
        }
        usage::change_line(
            now.saturating_sub(previous.at),
            previous.left,
            focus.remaining,
        )
    });
    let recent_base = previous.and_then(|previous| {
        if previous.key != key {
            None
        } else {
            Some(previous.left)
        }
    });
    write_memory(key, focus.remaining, now);
    let open = read_day()
        .and_then(|mark| {
            if mark.day == today && mark.pane == key {
                Some(mark.open)
            } else {
                None
            }
        })
        .unwrap_or(focus.remaining);
    if !today.is_empty() {
        write_day(&today, key, open);
        remember_days(key, &today, open, focus.remaining);
    }
    remember_pace(key, now, focus.remaining);
    remember_cycle(key, focus.remaining);
    let (_day, recent) = if today.is_empty() {
        (String::new(), recent.unwrap_or_default())
    } else {
        usage::account_moves(open, focus.remaining, recent_base, recent)
    };
    if let Ok(mut slot) = DELTA.lock() {
        *slot = recent;
    }
}

struct DayMark {
    day: String,
    pane: String,
    open: u32,
}

fn day_path() -> Option<std::path::PathBuf> {
    data_file("day.txt")
}

fn read_day() -> Option<DayMark> {
    let text = std::fs::read_to_string(day_path()?).ok()?;
    let mut day = String::new();
    let mut pane = String::new();
    let mut open = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("day=") {
            day = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("pane=") {
            pane = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("open=") {
            open = value.trim().parse().ok();
        }
    }
    if day.is_empty() || pane.is_empty() {
        return None;
    }
    Some(DayMark {
        day,
        pane,
        open: open?,
    })
}

fn days_path() -> Option<std::path::PathBuf> {
    data_file("days.txt")
}

fn read_days() -> String {
    days_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default()
}

fn remember_days(pane: &str, date: &str, open: u32, close: u32) {
    let Some(path) = days_path() else {
        return;
    };
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let Some(next) = usage::day_record(&existing, pane, date, open, close) else {
        return;
    };
    let _ = std::fs::write(path, next);
}

fn write_day(day: &str, pane: &str, open: u32) {
    let Some(path) = day_path() else {
        return;
    };
    let _ = std::fs::write(path, format!("day={day}\npane={pane}\nopen={open}\n"));
}

struct SavedNumber {
    key: String,
    left: u32,
    at: i64,
}

fn memory_path() -> Option<std::path::PathBuf> {
    data_file("last.txt")
}

fn read_memory() -> Option<SavedNumber> {
    let text = std::fs::read_to_string(memory_path()?).ok()?;
    let mut key = String::new();
    let mut left = None;
    let mut at = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("pane=") {
            key = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("left=") {
            left = value.trim().parse().ok();
        } else if let Some(value) = line.strip_prefix("at=") {
            at = value.trim().parse().ok();
        }
    }
    Some(SavedNumber {
        key,
        left: left?,
        at: at?,
    })
}

fn write_memory(key: &str, left: u32, at: i64) {
    let Some(path) = memory_path() else {
        return;
    };
    let _ = std::fs::write(path, format!("pane={key}\nleft={left}\nat={at}\n"));
}

fn pace_path() -> Option<std::path::PathBuf> {
    data_file("pace.txt")
}

fn remember_pace(pane: &str, now: i64, left: u32) {
    let Some(path) = pace_path() else {
        return;
    };
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let Some(next) = usage::pace_record(&existing, pane, now, left) else {
        return;
    };
    let _ = std::fs::write(path, next);
}

fn cycle_path() -> Option<std::path::PathBuf> {
    data_file("cycle.txt")
}

fn read_cycle() -> String {
    cycle_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default()
}

fn remember_cycle(pane: &str, remaining: u32) {
    let Some(path) = cycle_path() else {
        return;
    };
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let Some(next) = usage::cycle_update(&existing, pane, remaining) else {
        return;
    };
    let _ = std::fs::write(path, next);
}

fn pace_for(pane: &str) -> Vec<(i64, u32)> {
    let Some(path) = pace_path() else {
        return Vec::new();
    };
    let text = std::fs::read_to_string(path).unwrap_or_default();
    usage::pace_rows(&text, pane)
}

fn enable_glass(hwnd: HWND) {
    let backdrop: i32 = 3;
    let border: u32 = 0xFFFFFFFE;
    let margins = MARGINS {
        cxLeftWidth: -1,
        cxRightWidth: -1,
        cyTopHeight: -1,
        cyBottomHeight: -1,
    };
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, 38, &backdrop as *const i32 as *const c_void, 4);
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        let _ = DwmSetWindowAttribute(hwnd, 34, &border as *const u32 as *const c_void, 4);
    }
}

unsafe fn blend_round(
    dest: windows_sys::Win32::Graphics::Gdi::HDC,
    spot: Spot,
    radius: i32,
    rgba: [u8; 4],
    sheen: bool,
) {
    if spot.w <= 1 || spot.h <= 1 {
        return;
    }
    let width = spot.w as u32;
    let height = spot.h as u32;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let radius = radius.max(0) as f32;
    for y in 0..height {
        for x in 0..width {
            let cover = round_coverage(x as f32, y as f32, width as f32, height as f32, radius);
            if cover <= 0.0 {
                continue;
            }
            let (mut red, mut green, mut blue, mut alpha) = (rgba[0], rgba[1], rgba[2], rgba[3]);
            if sheen && y < 22 {
                let lift = ((22 - y) as u16 * 3).min(70) as u8;
                red = red.saturating_add(lift);
                green = green.saturating_add(lift);
                blue = blue.saturating_add(lift);
                alpha = alpha.saturating_add(lift / 3);
            }
            if cover < 0.9 {
                red = 255;
                green = 255;
                blue = 255;
                alpha = (200.0 * cover) as u8;
            }
            let alpha = (alpha as f32 * cover.min(1.0)) as u8;
            let index = ((y * width + x) * 4) as usize;
            pixels[index] = premul(blue, alpha);
            pixels[index + 1] = premul(green, alpha);
            pixels[index + 2] = premul(red, alpha);
            pixels[index + 3] = alpha;
        }
    }
    blend_pixels(dest, spot, &pixels);
}

unsafe fn blend_pixels(dest: windows_sys::Win32::Graphics::Gdi::HDC, spot: Spot, pixels: &[u8]) {
    if spot.w <= 1 || spot.h <= 1 || pixels.len() < (spot.w as usize) * (spot.h as usize) * 4 {
        return;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: spot.w,
            biHeight: -spot.h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };
    let mut bits = std::ptr::null_mut();
    let bitmap = CreateDIBSection(
        dest,
        &bmi,
        DIB_RGB_COLORS,
        &mut bits,
        std::ptr::null_mut(),
        0,
    );
    if bitmap.is_null() || bits.is_null() {
        return;
    }
    std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, pixels.len());
    let source = CreateCompatibleDC(dest);
    let old = SelectObject(source, bitmap as HGDIOBJ);
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    AlphaBlend(
        dest, spot.x, spot.y, spot.w, spot.h, source, 0, 0, spot.w, spot.h, blend,
    );
    SelectObject(source, old);
    DeleteObject(bitmap as HGDIOBJ);
    DeleteDC(source);
}

fn premul(channel: u8, alpha: u8) -> u8 {
    ((channel as u16 * alpha as u16) / 255) as u8
}

fn round_coverage(x: f32, y: f32, width: f32, height: f32, radius: f32) -> f32 {
    let px = x + 0.5;
    let py = y + 0.5;
    let half_w = width / 2.0;
    let half_h = height / 2.0;
    let radius = radius.min(half_w).min(half_h);
    let qx = (px - half_w).abs() - (half_w - radius);
    let qy = (py - half_h).abs() - (half_h - radius);
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    let inside = qx.min(0.0).max(qy.min(0.0));
    (0.6 - (outside + inside - radius)).clamp(0.0, 1.0)
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn set_process_dpi() -> i32 {
    windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
        windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    )
}
