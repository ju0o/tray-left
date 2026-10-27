use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, FALSE, COLORREF, FILETIME, HWND, LPARAM, RECT, SIZE,
    SYSTEMTIME, TRUE, WPARAM,
};
use windows_sys::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows_sys::Win32::Graphics::Gdi::{
    AlphaBlend, BeginPaint, CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW,
    CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
    GetDC, GetDeviceCaps, GetTextExtentPoint32W, InvalidateRect, ReleaseDC, RoundRect,
    SelectObject, SetBkMode, SetTextCharacterExtra, SetTextColor, SetWindowRgn, AC_SRC_ALPHA,
    AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, DIB_RGB_COLORS, DT_CENTER,
    DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER,
    FW_NORMAL, FW_SEMIBOLD, HFONT, HGDIOBJ, LOGPIXELSX, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
};
use windows_sys::Win32::Storage::FileSystem::FileTimeToLocalFileTime;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Time::FileTimeToSystemTime;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteKeyValueW, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, RegSetKeyValueW,
    RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_ENUMERATE_SUB_KEYS, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE,
    REG_DWORD, REG_SZ, RRF_RT_REG_SZ,
};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Controls::{MARGINS, WM_MOUSELEAVE};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, CreateWindowExW, DefWindowProcW, DestroyIcon, DispatchMessageW,
    FindWindowW, GetMessageW, GetSystemMetrics, IsWindowVisible, KillTimer,
    LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassW, SetCursor, SetForegroundWindow,
    SetTimer, SetWindowPos, ShowWindow, SystemParametersInfoW, TranslateMessage, CS_DROPSHADOW,
    CS_HREDRAW, CS_VREDRAW, HICON, ICONINFO, IDC_ARROW, IDC_HAND, SM_CXSMICON, SPI_GETWORKAREA,
    SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOW, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND,
    WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONUP,
    WM_SETCURSOR, WM_TIMER, WNDCLASSW, WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_POPUP,
};

use crate::icon::remaining_icon;
use crate::usage::{self, Civil, FetchFail, Pane, Reading, Snapshot};

const CARD_W: i32 = 440;
const MOTION_FRAMES: u8 = 16;
const TRAY_MSG: u32 = 0x8000 + 1;
const REFRESH_MSG: u32 = 0x8000 + 2;
const SHOW_MSG: u32 = 0x8000 + 3;
const ICON_ID: u32 = 1;
const HIT_CLOSE: u8 = 1;
const HIT_REFRESH: u8 = 2;
const HIT_QUIT: u8 = 3;

const INK: u32 = 0x00171717;
const MUTED: u32 = 0x005A5A5A;
const WHITE: u32 = 0x00FFFFFF;
const BUTTON: u32 = 0x00171717;
const BUTTON_HOVER: u32 = 0x00323232;
const SOFT: u32 = 0x00E8E6E2;

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
    figure: HFONT,
    lead: HFONT,
    body: HFONT,
    quiet: HFONT,
}

unsafe impl Send for Fonts {}

struct Tile {
    spot: Spot,
    label: String,
    value: String,
    mood: String,
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

struct Card {
    width: i32,
    height: i32,
    close: Spot,
    refresh: Spot,
    quit: Spot,
    brand_box: Spot,
    plan_box: Spot,
    hero_box: Spot,
    scope_box: Option<Spot>,
    bar: Option<Spot>,
    refill_box: Option<Spot>,
    note_box: Option<Spot>,
    delta_box: Option<Spot>,
    tiles: Vec<Tile>,
    hero: String,
    word: String,
    scope: String,
    refill: String,
    note: String,
    delta: String,
    plan: String,
    hero_big: bool,
    accent: u32,
    remaining: u32,
}

struct Motion {
    tick: u8,
    live: bool,
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
static MOTION: Mutex<Motion> = Mutex::new(Motion { tick: MOTION_FRAMES, live: false });

pub fn run() {
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
        let icon = make_icon(icon_px(), 0, usage::mood_rgb(None), false);
        ICON_SLOT.store(icon as usize, Ordering::SeqCst);
        add_tray(hwnd, icon, "Jusage 확인 중");
        apply_view();
        remember_login();
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

fn worker(rx: mpsc::Receiver<()>) {
    let home = usage::codex_home();
    publish(home.as_path());
    let mut seen = usage::newest_activity(&home);
    let mut due: Option<Instant> = None;
    loop {
        if QUIT_FLAG.load(Ordering::Relaxed) {
            break;
        }
        let wait = due
            .map(|moment| moment.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_secs(20));
        let woke = rx.recv_timeout(wait).is_ok();
        if woke {
            due = None;
            publish(home.as_path());
            seen = usage::newest_activity(&home);
            continue;
        }
        let current = usage::newest_activity(&home);
        if current > seen {
            seen = current;
            due = Some(Instant::now() + Duration::from_secs(90));
        }
        if due.is_some_and(|moment| Instant::now() >= moment) {
            due = None;
            publish(home.as_path());
        }
    }
}

fn publish(home: &std::path::Path) {
    let next = match usage::fetch_usage(home) {
        Ok(snapshot) => {
            note_change(&snapshot);
            View::Ready(snapshot)
        }
        Err(FetchFail::NoLogin) => {
            clear_delta();
            View::NeedLogin
        }
        Err(fail) => {
            clear_delta();
            View::Failed(fail.code())
        }
    };
    if let Ok(mut view) = VIEW.lock() {
        *view = next;
    }
    let hwnd = HWND_SLOT.load(Ordering::SeqCst) as HWND;
    if !hwnd.is_null() {
        unsafe {
            let _ = PostMessageW(hwnd, REFRESH_MSG, 0, 0);
        }
    }
}

fn request_refresh() {
    let Ok(mut view) = VIEW.lock() else {
        return;
    };
    if matches!(*view, View::Loading) {
        return;
    }
    *view = View::Loading;
    drop(view);
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
    }
    let old = ICON_SLOT.swap(icon as usize, Ordering::SeqCst) as HICON;
    if !old.is_null() && old != icon {
        DestroyIcon(old);
    }
    let focus_name = match &view {
        View::Ready(snapshot) => usage::focus_reading(snapshot).name.to_string(),
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
    if IsWindowVisible(hwnd) != 0 {
        place(hwnd);
    }
    InvalidateRect(hwnd, std::ptr::null(), FALSE);
}

fn describe(view: &View) -> (String, u32, [u8; 3], &'static str, Option<u32>, String, Option<u16>) {
    match view {
        View::Loading => (
            "Jusage 확인 중".into(),
            0,
            usage::mood_rgb(None),
            "loading",
            None,
            String::new(),
            None,
        ),
        View::NeedLogin => (
            "Jusage 로그인 필요".into(),
            0,
            usage::mood_rgb(None),
            "login",
            None,
            String::new(),
            None,
        ),
        View::Failed(code) => (
            "Jusage 확인 실패".into(),
            0,
            usage::mood_rgb(None),
            "failed",
            None,
            String::new(),
            Some(*code),
        ),
        View::Ready(snapshot) => {
            let focus = usage::focus_reading(snapshot);
            (
                usage::hover_tip(snapshot),
                focus.remaining,
                usage::mood_rgb(Some(focus.remaining)),
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
    let s = |value: i32| scale(value, dpi);
    let mut tiles = Vec::new();
    let hero;
    let mut word = String::new();
    let mut scope = String::new();
    let mut refill = String::new();
    let mut note = String::new();
    let mut delta = String::new();
    let mut plan = String::new();
    let mut hero_big = false;
    let mut accent = INK;
    let mut remaining = 0u32;
    let mut show_bar = false;
    match view {
        View::Loading => hero = "확인 중".to_string(),
        View::NeedLogin => {
            hero = "로그인이 필요해요".to_string();
            note = "Codex에 로그인한 뒤 다시 확인해 주세요.".to_string();
        }
        View::Failed(_) => {
            hero = "확인하지 못했어요".to_string();
            note = "잠시 뒤 다시 확인해 주세요.".to_string();
        }
        View::Ready(snapshot) => {
            let (session, week) = usage::readings(snapshot);
            let focus = usage::focus_reading(snapshot);
            remaining = focus.remaining;
            hero = format!("{}%", focus.remaining);
            word = usage::mood_word(focus.remaining).to_string();
            hero_big = true;
            accent = rgb_color(usage::mood_rgb(Some(focus.remaining)));
            show_bar = true;
            scope = focus.name.to_string();
            plan = snapshot.plan.clone();
            let now = now_unix();
            if let Some(at) = reset_unix(focus.reset_at, focus.reset_after, now) {
                if let (Some(today), Some(moment)) = (local_civil(now), local_civil(at)) {
                    refill = usage::named_refill(focus.name, &usage::refill_phrase(today, moment));
                }
            }
            if let Some(line) = DELTA.lock().ok() {
                delta = line.clone();
            }
            if let Some(week) = week {
                let mut session_tile = tile_from(session);
                let mut week_tile = tile_from(week);
                session_tile.hot = focus.pane == Pane::Session;
                week_tile.hot = focus.pane == Pane::Week;
                tiles.push(session_tile);
                tiles.push(week_tile);
            }
        }
    }
    let width = s(CARD_W);
    let pad = s(22);
    let close_size = s(40);
    let close = Spot {
        x: width - close_size - s(8),
        y: s(8),
        w: close_size,
        h: close_size,
    };
    let brand_box = Spot { x: pad, y: s(8), w: s(210), h: close_size };
    let plan_box = Spot {
        x: brand_box.x + brand_box.w,
        y: s(8),
        w: close.x - brand_box.x - brand_box.w - s(8),
        h: close_size,
    };
    let mut y = s(58);
    let hero_box = Spot {
        x: pad,
        y,
        w: width - pad * 2,
        h: if hero_big { s(58) } else { s(36) },
    };
    y += hero_box.h;
    let scope_box = if scope.is_empty() {
        None
    } else {
        let spot = Spot { x: pad, y, w: width - pad * 2, h: s(20) };
        y += spot.h;
        Some(spot)
    };
    let bar = if show_bar {
        y += s(12);
        let spot = Spot { x: pad, y, w: width - pad * 2, h: s(8).max(6) };
        y += spot.h;
        Some(spot)
    } else {
        None
    };
    let refill_box = if refill.is_empty() {
        None
    } else {
        y += s(10);
        let spot = Spot { x: pad, y, w: width - pad * 2, h: s(22) };
        y += spot.h;
        Some(spot)
    };
    let note_box = if note.is_empty() {
        None
    } else {
        y += s(8);
        let spot = Spot { x: pad, y, w: width - pad * 2, h: s(22) };
        y += spot.h;
        Some(spot)
    };
    if !tiles.is_empty() {
        y += s(16);
        let gap = s(10);
        let tile_w = (width - pad * 2 - gap) / tiles.len() as i32;
        let tile_h = s(86);
        for (index, tile) in tiles.iter_mut().enumerate() {
            tile.spot = Spot {
                x: pad + (tile_w + gap) * index as i32,
                y,
                w: tile_w,
                h: tile_h,
            };
        }
        y += tile_h;
    }
    let delta_box = if delta.is_empty() {
        None
    } else {
        y += s(12);
        let spot = Spot { x: pad, y, w: width - pad * 2, h: s(20) };
        y += spot.h;
        Some(spot)
    };
    y += s(18);
    let button_w = s(176).min(width - pad * 2);
    let refresh = Spot {
        x: (width - button_w) / 2,
        y,
        w: button_w,
        h: s(42),
    };
    y += refresh.h;
    let quit = Spot { x: pad, y, w: width - pad * 2, h: s(36) };
    y += quit.h + s(12);
    Card {
        width,
        height: y,
        close,
        refresh,
        quit,
        brand_box,
        plan_box,
        hero_box,
        scope_box,
        bar,
        refill_box,
        note_box,
        delta_box,
        tiles,
        hero,
        word,
        scope,
        refill,
        note,
        delta,
        plan,
        hero_big,
        accent,
        remaining,
    }
}

fn tile_from(reading: Reading) -> Tile {
    Tile {
        spot: Spot { x: 0, y: 0, w: 0, h: 0 },
        label: reading.name.to_string(),
        value: format!("{}%", reading.remaining),
        mood: usage::mood_word(reading.remaining).to_string(),
        accent: rgb_color(usage::mood_rgb(Some(reading.remaining))),
        hot: false,
    }
}

unsafe fn place(hwnd: HWND) {
    let dpi = window_dpi(hwnd);
    let Some(view) = VIEW.lock().ok().map(|view| view.clone()) else {
        return;
    };
    let card = card_for(&view, dpi);
    let mut work = RECT::default();
    SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0);
    let x = work.right - card.width - scale(16, dpi);
    let y = work.bottom - card.height - scale(16, dpi) + motion_offset();
    SetWindowPos(
        hwnd,
        std::ptr::null_mut(),
        x,
        y,
        card.width,
        card.height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    let radius = scale(22, dpi);
    let region = CreateRoundRectRgn(0, 0, card.width + 1, card.height + 1, radius, radius);
    if !region.is_null() {
        SetWindowRgn(hwnd, region, TRUE);
    }
}

unsafe fn show_panel(hwnd: HWND) {
    if let Ok(mut motion) = MOTION.lock() {
        motion.tick = 0;
        motion.live = true;
    }
    place(hwnd);
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
    SetTimer(hwnd, 1, 16, None);
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
        WM_TIMER => {
            let done = if let Ok(mut motion) = MOTION.lock() {
                if motion.tick < MOTION_FRAMES {
                    motion.tick += 1;
                }
                motion.tick >= MOTION_FRAMES
            } else {
                true
            };
            place(hwnd);
            InvalidateRect(hwnd, std::ptr::null(), FALSE);
            if done {
                KillTimer(hwnd, 1);
            }
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
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
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
            ShowWindow(hwnd, SW_HIDE);
        },
        HIT_REFRESH => request_refresh(),
        HIT_QUIT => unsafe {
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

fn hit_at(hwnd: HWND, x: i32, y: i32) -> u8 {
    let Ok(view) = VIEW.lock() else {
        return 0;
    };
    let card = card_for(&view, window_dpi(hwnd));
    if card.close.contains(x, y) {
        HIT_CLOSE
    } else if card.refresh.contains(x, y) {
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
    let view = VIEW.lock().ok().map(|view| view.clone()).unwrap_or(View::Loading);
    let card = card_for(&view, window_dpi(hwnd));
    let fonts = fonts_for(window_dpi(hwnd));
    blend_round(hdc, Spot { x: 0, y: 0, w: card.width, h: card.height }, px(&card, 22), [255, 255, 255, 188], true);
    paint_card(hdc, &card, &fonts);
    EndPaint(hwnd, &ps);
}

unsafe fn paint_card(hdc: windows_sys::Win32::Graphics::Gdi::HDC, card: &Card, fonts: &Fonts) {
    let hover = HOVER.load(Ordering::Relaxed);
    let one = DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX;
    if hover == HIT_CLOSE {
        fill_round(hdc, card.close, card.close.h / 2, SOFT);
    }
    SetTextCharacterExtra(hdc, px(card, 1).max(1));
    text_in(
        hdc,
        fonts.brand,
        INK,
        card.brand_box.rect(),
        "JUSAGE",
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    let brand_w = text_size(hdc, fonts.brand, "JUSAGE").cx + px(card, 18);
    SetTextCharacterExtra(hdc, 0);
    text_in(
        hdc,
        fonts.quiet,
        MUTED,
        rect(card.brand_box.x + brand_w, card.brand_box.y, px(card, 70), card.brand_box.h),
        "CODEX",
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    if !card.plan.is_empty() {
        text_in(
            hdc,
            fonts.quiet,
            MUTED,
            card.plan_box.rect(),
            &card.plan,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }
    text_in(hdc, fonts.brand, MUTED, card.close.rect(), "×", DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    if card.hero_big {
        paint_meter(hdc, card, fonts);
    } else {
        text_in(
            hdc,
            fonts.lead,
            INK,
            card.hero_box.rect(),
            &card.hero,
            one,
        );
    }
    if let Some(scope) = card.scope_box {
        text_in(hdc, fonts.quiet, MUTED, scope.rect(), &card.scope, one);
    }
    if let Some(bar) = card.bar {
        blend_round(hdc, bar, bar.h, [24, 24, 24, 28], false);
        let eased = (card.remaining as f32 * motion_ease()) as u32;
        let filled = (bar.w as u32 * eased / 100) as i32;
        if filled > 0 {
            let [r, g, b] = color_rgb(card.accent);
            blend_round(hdc, Spot { w: filled.max(bar.h), ..bar }, bar.h, [r, g, b, 255], false);
        }
    }
    if let Some(spot) = card.refill_box {
        text_in(hdc, fonts.body, MUTED, spot.rect(), &card.refill, one);
    }
    if let Some(spot) = card.note_box {
        text_in(hdc, fonts.body, MUTED, spot.rect(), &card.note, one);
    }
    for tile in &card.tiles {
        let radius = px(card, 16);
        if tile.hot {
            let [r, g, b] = color_rgb(tile.accent);
            blend_round(hdc, tile.spot, radius, [r, g, b, 70], false);
            let inset = Spot {
                x: tile.spot.x + 1,
                y: tile.spot.y + 1,
                w: (tile.spot.w - 2).max(1),
                h: (tile.spot.h - 2).max(1),
            };
            blend_round(hdc, inset, radius - 1, [255, 255, 255, 168], false);
        } else {
            blend_round(hdc, tile.spot, radius, [255, 255, 255, 130], false);
        }
        let inner_x = tile.spot.x + px(card, 14);
        let inner_w = tile.spot.w - px(card, 28);
        text_in(
            hdc,
            fonts.quiet,
            MUTED,
            rect(inner_x, tile.spot.y + px(card, 10), inner_w, px(card, 18)),
            &tile.label,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        text_in(
            hdc,
            fonts.figure,
            INK,
            rect(inner_x, tile.spot.y + px(card, 28), inner_w, px(card, 28)),
            &tile.value,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        text_in(
            hdc,
            fonts.quiet,
            tile.accent,
            rect(inner_x, tile.spot.y + px(card, 56), inner_w, px(card, 18)),
            &tile.mood,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
    }
    if let Some(spot) = card.delta_box {
        text_in(hdc, fonts.body, INK, spot.rect(), &card.delta, one);
    }
    let refresh_fill = if hover == HIT_REFRESH { BUTTON_HOVER } else { BUTTON };
    fill_round(hdc, card.refresh, card.refresh.h / 2, refresh_fill);
    text_in(
        hdc,
        fonts.body,
        WHITE,
        card.refresh.rect(),
        "다시 확인",
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    text_in(
        hdc,
        fonts.body,
        if hover == HIT_QUIT { INK } else { MUTED },
        card.quit.rect(),
        "끝내기",
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
}

unsafe fn paint_meter(hdc: windows_sys::Win32::Graphics::Gdi::HDC, card: &Card, fonts: &Fonts) {
    let percent = text_size(hdc, fonts.hero, &card.hero);
    let word = text_size(hdc, fonts.body, &card.word);
    let gap = px(card, 8);
    let total = percent.cx + if card.word.is_empty() { 0 } else { gap + word.cx };
    let x = card.hero_box.x + (card.hero_box.w - total).max(0) / 2;
    let percent_y = card.hero_box.y + (card.hero_box.h - percent.cy).max(0) / 2;
    text_in(
        hdc,
        fonts.hero,
        card.accent,
        rect(x, percent_y, percent.cx + px(card, 4), percent.cy + 4),
        &card.hero,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    if card.word.is_empty() {
        return;
    }
    let word_y = card.hero_box.y + (card.hero_box.h - word.cy).max(0) / 2;
    text_in(
        hdc,
        fonts.body,
        card.accent,
        rect(x + percent.cx + gap, word_y, word.cx + px(card, 8), word.cy + 4),
        &card.word,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
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
                figure: fonts.figure,
                lead: fonts.lead,
                body: fonts.body,
                quiet: fonts.quiet,
            };
        }
        DeleteObject(fonts.brand as HGDIOBJ);
        DeleteObject(fonts.hero as HGDIOBJ);
        DeleteObject(fonts.figure as HGDIOBJ);
        DeleteObject(fonts.lead as HGDIOBJ);
        DeleteObject(fonts.body as HGDIOBJ);
        DeleteObject(fonts.quiet as HGDIOBJ);
    }
    let fonts = make_fonts(dpi);
    *slot = Some(Fonts {
        dpi: fonts.dpi,
        brand: fonts.brand,
        hero: fonts.hero,
        figure: fonts.figure,
        lead: fonts.lead,
        body: fonts.body,
        quiet: fonts.quiet,
    });
    fonts
}

unsafe fn make_fonts(dpi: i32) -> Fonts {
    Fonts {
        dpi,
        brand: font(scale(12, dpi), FW_SEMIBOLD, "Cascadia Mono"),
        hero: font(scale(46, dpi), FW_SEMIBOLD, "Cascadia Mono"),
        figure: font(scale(22, dpi), FW_SEMIBOLD, "Cascadia Mono"),
        lead: font(scale(18, dpi), FW_SEMIBOLD, "Segoe UI"),
        body: font(scale(13, dpi), FW_NORMAL, "Segoe UI"),
        quiet: font(scale(12, dpi), FW_NORMAL, "Segoe UI"),
    }
}

unsafe fn font(height: i32, weight: u32, face_name: &str) -> HFONT {
    let face = wide(face_name);
    CreateFontW(-height, 0, 0, 0, weight as i32, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr())
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

unsafe fn text_in(hdc: windows_sys::Win32::Graphics::Gdi::HDC, font: HFONT, color: u32, mut area: RECT, value: &str, format: u32) {
    let old = SelectObject(hdc, font as HGDIOBJ);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color as COLORREF);
    let text = wide(value);
    DrawTextW(hdc, text.as_ptr(), -1, &mut area, format);
    SelectObject(hdc, old);
}

unsafe fn fill_round(hdc: windows_sys::Win32::Graphics::Gdi::HDC, spot: Spot, radius: i32, color: u32) {
    let brush = CreateSolidBrush(color as COLORREF);
    let pen = CreatePen(PS_SOLID, 1, color as COLORREF);
    let old_brush = SelectObject(hdc, brush as HGDIOBJ);
    let old_pen = SelectObject(hdc, pen as HGDIOBJ);
    RoundRect(hdc, spot.x, spot.y, spot.x + spot.w, spot.y + spot.h, radius, radius);
    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(brush as HGDIOBJ);
    DeleteObject(pen as HGDIOBJ);
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT { left: x, top: y, right: x + w, bottom: y + h }
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
    let pixels = remaining_icon(size, remaining, color, show_number);
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
    let color = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
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

fn scale(px: i32, dpi: i32) -> i32 {
    px * dpi / 96
}

unsafe fn screen_dpi() -> i32 {
    let hdc = GetDC(std::ptr::null_mut());
    let dpi = GetDeviceCaps(hdc, LOGPIXELSX as i32);
    ReleaseDC(std::ptr::null_mut(), hdc);
    if dpi <= 0 { 96 } else { dpi }
}

fn write_state(kind: &str, left: Option<u32>, plan: &str, fail_code: Option<u16>, focus: &str, week_left: Option<u32>) {
    let plan: String = plan.chars().filter(|ch| !ch.is_control()).take(40).collect();
    let focus: String = focus.chars().filter(|ch| !ch.is_control()).take(20).collect();
    let body = if kind == "ready" {
        let week = week_left.map(|value| format!("week={value}\n")).unwrap_or_default();
        format!("state=ready\nleft={}\nplan={plan}\nfocus={focus}\n{week}", left.unwrap_or(0))
    } else if let Some(code) = fail_code {
        format!("state={kind}\ncode={code}\n")
    } else {
        format!("state={kind}\n")
    };
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let _ = std::fs::write(dir.join("state.txt"), body);
}

fn remember_login() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let path = exe.to_string_lossy();
    let sub = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let name = wide("Jusage");
    let old_name = wide("TrayLeft");
    let value = wide(&path);
    unsafe {
        let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, sub.as_ptr(), old_name.as_ptr());
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
        if RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, KEY_READ | KEY_ENUMERATE_SUB_KEYS, &mut root) != 0 {
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
            if RegOpenKeyExW(root, name.as_ptr(), 0, KEY_QUERY_VALUE | KEY_SET_VALUE, &mut sub) != 0 {
                continue;
            }
            if exe_matches(sub, &needle) {
                let one = 1u32;
                let value_name = wide("IsPromoted");
                let _ = RegSetValueExW(sub, value_name.as_ptr(), 0, REG_DWORD, &one as *const u32 as *const u8, 4);
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
    String::from_utf16_lossy(&bytes).to_ascii_lowercase().contains(needle)
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

fn motion_ease() -> f32 {
    let tick = MOTION.lock().map(|motion| motion.tick).unwrap_or(MOTION_FRAMES);
    let t = (tick as f32 / MOTION_FRAMES as f32).clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn icon_px() -> u32 {
    let small = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16) as u32;
    (small.saturating_mul(2)).clamp(40, 64)
}

fn color_rgb(color: u32) -> [u8; 3] {
    [(color & 0xff) as u8, ((color >> 8) & 0xff) as u8, ((color >> 16) & 0xff) as u8]
}

fn clear_delta() {
    if let Ok(mut line) = DELTA.lock() {
        line.clear();
    }
}

fn note_change(snapshot: &Snapshot) {
    let focus = usage::focus_reading(snapshot);
    let key = match focus.pane {
        Pane::Session => "session",
        Pane::Week => "week",
    };
    let now = now_unix();
    let line = read_memory().and_then(|previous| {
        if previous.key != key {
            return None;
        }
        usage::change_line(now.saturating_sub(previous.at), previous.left, focus.remaining)
    });
    write_memory(key, focus.remaining, now);
    if let Ok(mut slot) = DELTA.lock() {
        *slot = line.unwrap_or_default();
    }
}

struct SavedNumber {
    key: String,
    left: u32,
    at: i64,
}

fn memory_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("last.txt"))
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
    Some(SavedNumber { key, left: left?, at: at? })
}

fn write_memory(key: &str, left: u32, at: i64) {
    let Some(path) = memory_path() else {
        return;
    };
    let _ = std::fs::write(path, format!("pane={key}\nleft={left}\nat={at}\n"));
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

unsafe fn blend_round(dest: windows_sys::Win32::Graphics::Gdi::HDC, spot: Spot, radius: i32, rgba: [u8; 4], sheen: bool) {
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
    let bitmap = CreateDIBSection(dest, &bmi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
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
    AlphaBlend(dest, spot.x, spot.y, spot.w, spot.h, source, 0, 0, spot.w, spot.h, blend);
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
