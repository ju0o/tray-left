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
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection,
    CreateFontW, CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, EndPaint, FillRect, GetDC, GetDeviceCaps, GetTextExtentPoint32W,
    InvalidateRect, ReleaseDC, RoundRect,
    SelectObject, SetBkMode, SetTextColor, SetWindowRgn, BITMAPINFO, BITMAPINFOHEADER,
    DIB_RGB_COLORS, DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER,
    DT_WORDBREAK, FW_NORMAL, FW_SEMIBOLD, HFONT, HGDIOBJ, LOGPIXELSX, PAINTSTRUCT,
    PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows_sys::Win32::Storage::FileSystem::FileTimeToLocalFileTime;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Time::FileTimeToSystemTime;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, RegSetKeyValueW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_ENUMERATE_SUB_KEYS, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE,
    REG_DWORD, REG_SZ, RRF_RT_REG_SZ,
};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
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
    FindWindowW, GetClientRect, GetMessageW, IsWindowVisible, LoadCursorW,
    PostMessageW, PostQuitMessage, RegisterClassW, SetCursor, SetForegroundWindow, SetWindowPos,
    ShowWindow, SystemParametersInfoW, TranslateMessage, CS_DROPSHADOW, CS_HREDRAW, CS_VREDRAW,
    HICON, ICONINFO, IDC_ARROW, IDC_HAND, SPI_GETWORKAREA, SWP_NOACTIVATE, SWP_NOZORDER,
    SW_HIDE, SW_SHOW, WM_CLOSE, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDBLCLK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONUP, WM_SETCURSOR, WNDCLASSW,
    WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_POPUP,
};

use crate::icon::remaining_icon;
use crate::usage::{self, Civil, FetchFail, Snapshot};

const CARD_W: i32 = 320;
const TRAY_MSG: u32 = 0x8000 + 1;
const REFRESH_MSG: u32 = 0x8000 + 2;
const SHOW_MSG: u32 = 0x8000 + 3;
const ICON_ID: u32 = 1;
const HIT_CLOSE: u8 = 1;
const HIT_REFRESH: u8 = 2;
const HIT_QUIT: u8 = 3;

const INK: u32 = 0x00171717;
const MUTED: u32 = 0x005A5A5A;
const PAPER: u32 = 0x00F4F6F7;
const LINE: u32 = 0x00D8D6D2;
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
    title: HFONT,
    hero: HFONT,
    lead: HFONT,
    body: HFONT,
}

unsafe impl Send for Fonts {}

struct Row {
    label: String,
    value: String,
    value_color: u32,
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
    hero_box: Spot,
    scope_box: Option<Spot>,
    bar: Option<Spot>,
    refill_box: Option<Spot>,
    note_box: Option<Spot>,
    rows_top: i32,
    row_h: i32,
    rows: Vec<Row>,
    hero: String,
    word: String,
    scope: String,
    refill: String,
    note: String,
    hero_big: bool,
    accent: u32,
    remaining: u32,
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

pub fn run() {
    unsafe {
        let _ = set_process_dpi();
        let class_name = wide("TrayLeftWindow");
        let title = wide("Codex");
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
        let mutex_name = wide("Local\\TrayLeftSingle");
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
        let icon = make_icon(32, 0, usage::mood_rgb(None));
        ICON_SLOT.store(icon as usize, Ordering::SeqCst);
        add_tray(hwnd, icon, "Codex 확인 중");
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
        Ok(snapshot) => View::Ready(snapshot),
        Err(FetchFail::NoLogin) => View::NeedLogin,
        Err(fail) => View::Failed(fail.code()),
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
    let icon = make_icon(32, remaining, rgb);
    if !hwnd.is_null() {
        update_tray(hwnd, icon, &tip);
    }
    let old = ICON_SLOT.swap(icon as usize, Ordering::SeqCst) as HICON;
    if !old.is_null() && old != icon {
        DestroyIcon(old);
    }
    write_state(kind, left, &plan, code);
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
            "Codex 확인 중".into(),
            0,
            usage::mood_rgb(None),
            "loading",
            None,
            String::new(),
            None,
        ),
        View::NeedLogin => (
            "Codex 로그인 필요".into(),
            0,
            usage::mood_rgb(None),
            "login",
            None,
            String::new(),
            None,
        ),
        View::Failed(code) => (
            "Codex 확인 실패".into(),
            0,
            usage::mood_rgb(None),
            "failed",
            None,
            String::new(),
            Some(*code),
        ),
        View::Ready(snapshot) => {
            let left = usage::remaining_percent(snapshot.session_used);
            (
                format!("Codex {left}% 남음"),
                left,
                usage::mood_rgb(Some(left)),
                "ready",
                Some(left),
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
    let mut rows = Vec::new();
    let hero;
    let mut word = String::new();
    let mut scope = String::new();
    let mut refill = String::new();
    let mut note = String::new();
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
            let left = usage::remaining_percent(snapshot.session_used);
            remaining = left;
            hero = format!("{left}%");
            word = usage::mood_word(left).to_string();
            hero_big = true;
            accent = rgb_color(usage::mood_rgb(Some(left)));
            show_bar = true;
            scope = usage::window_name(snapshot.session_window_seconds, false).to_string();
            let now = now_unix();
            if let Some(at) = reset_unix(snapshot.session_reset_at, snapshot.session_reset_after, now) {
                if let (Some(today), Some(moment)) = (local_civil(now), local_civil(at)) {
                    refill = usage::refill_phrase(today, moment);
                }
            }
            if let Some(used) = snapshot.weekly_used {
                let week_left = usage::remaining_percent(used);
                let mut label = usage::window_name(snapshot.weekly_window_seconds, true).to_string();
                if label == scope {
                    label = "다른 창".to_string();
                }
                let value_color = if week_left > 20 {
                    INK
                } else {
                    rgb_color(usage::mood_rgb(Some(week_left)))
                };
                rows.push(Row {
                    label,
                    value: format!("{week_left}% 남음"),
                    value_color,
                });
            }
            if !snapshot.plan.is_empty() {
                rows.push(Row {
                    label: "요금제".to_string(),
                    value: snapshot.plan.clone(),
                    value_color: INK,
                });
            }
        }
    }
    let width = s(CARD_W);
    let pad = s(20);
    let close_size = s(44);
    let close = Spot {
        x: width - close_size - s(6),
        y: s(4),
        w: close_size,
        h: close_size,
    };
    let mut y = s(56);
    let hero_box = Spot {
        x: pad,
        y,
        w: width - pad * 2,
        h: if hero_big { s(40) } else { s(32) },
    };
    y += hero_box.h;
    let scope_box = if scope.is_empty() {
        None
    } else {
        let spot = Spot { x: pad, y, w: width - pad * 2, h: s(24) };
        y += spot.h;
        Some(spot)
    };
    let bar = if show_bar {
        y += s(10);
        let spot = Spot {
            x: pad,
            y,
            w: width - pad * 2,
            h: s(6).max(4),
        };
        y += spot.h;
        Some(spot)
    } else {
        None
    };
    let refill_box = if refill.is_empty() {
        None
    } else {
        y += s(8);
        let spot = Spot { x: pad, y, w: width - pad * 2, h: s(44) };
        y += spot.h;
        Some(spot)
    };
    let note_box = if note.is_empty() {
        None
    } else {
        y += s(6);
        let spot = Spot { x: s(28), y, w: width - s(56), h: s(52) };
        y += spot.h;
        Some(spot)
    };
    let rows_top = if rows.is_empty() {
        y
    } else {
        y += s(8);
        let top = y;
        y += s(32) * rows.len() as i32;
        top
    };
    let row_h = if rows.is_empty() { 0 } else { s(32) };
    y += s(12);
    let button_w = s(168).min(width - pad * 2);
    let refresh = Spot {
        x: (width - button_w) / 2,
        y,
        w: button_w,
        h: s(44),
    };
    y += refresh.h;
    let quit = Spot {
        x: pad,
        y,
        w: width - pad * 2,
        h: s(44),
    };
    y += quit.h + s(8);
    Card {
        width,
        height: y,
        close,
        refresh,
        quit,
        hero_box,
        scope_box,
        bar,
        refill_box,
        note_box,
        rows_top,
        row_h,
        rows,
        hero,
        word,
        scope,
        refill,
        note,
        hero_big,
        accent,
        remaining,
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
    let y = work.bottom - card.height - scale(16, dpi);
    SetWindowPos(
        hwnd,
        std::ptr::null_mut(),
        x,
        y,
        card.width,
        card.height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    let radius = scale(18, dpi);
    let region = CreateRoundRectRgn(0, 0, card.width + 1, card.height + 1, radius, radius);
    if !region.is_null() {
        SetWindowRgn(hwnd, region, TRUE);
    }
}

unsafe fn show_panel(hwnd: HWND) {
    place(hwnd);
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
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
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client);
    let width = client.right.max(1);
    let height = client.bottom.max(1);
    let mem = CreateCompatibleDC(hdc);
    let bitmap = CreateCompatibleBitmap(hdc, width, height);
    let old = SelectObject(mem, bitmap as HGDIOBJ);
    let view = VIEW.lock().ok().map(|view| view.clone()).unwrap_or(View::Loading);
    let card = card_for(&view, window_dpi(hwnd));
    let fonts = fonts_for(window_dpi(hwnd));
    paint_card(mem, &card, &fonts);
    BitBlt(hdc, 0, 0, width, height, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old);
    DeleteObject(bitmap as HGDIOBJ);
    DeleteDC(mem);
    EndPaint(hwnd, &ps);
}

unsafe fn paint_card(hdc: HGDIOBJ, card: &Card, fonts: &Fonts) {
    let hdc = hdc as windows_sys::Win32::Graphics::Gdi::HDC;
    let hover = HOVER.load(Ordering::Relaxed);
    let pad = px(card, 20);
    fill_rect(hdc, 0, 0, card.width, card.height, PAPER);
    text_in(
        hdc,
        fonts.title,
        INK,
        rect(pad, card.close.y, card.close.x - pad - px(card, 8), card.close.h),
        "Codex",
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    if hover == HIT_CLOSE {
        fill_round(hdc, card.close, px(card, 16), SOFT);
    }
    text_in(
        hdc,
        fonts.title,
        MUTED,
        card.close.rect(),
        "×",
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    if card.hero_big {
        paint_meter(hdc, card, fonts);
    } else {
        text_in(
            hdc,
            fonts.lead,
            INK,
            card.hero_box.rect(),
            &card.hero,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    if let Some(scope) = card.scope_box {
        text_in(
            hdc,
            fonts.body,
            MUTED,
            scope.rect(),
            &card.scope,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
    if let Some(bar) = card.bar {
        fill_round(hdc, bar, bar.h, LINE);
        let filled = (bar.w as u32 * card.remaining / 100) as i32;
        if filled > 0 {
            fill_round(hdc, Spot { w: filled.max(bar.h), ..bar }, bar.h, card.accent);
        }
    }
    if let Some(spot) = card.refill_box {
        text_in(
            hdc,
            fonts.body,
            MUTED,
            spot.rect(),
            &card.refill,
            DT_CENTER | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
    if let Some(spot) = card.note_box {
        text_in(
            hdc,
            fonts.body,
            MUTED,
            spot.rect(),
            &card.note,
            DT_CENTER | DT_WORDBREAK | DT_NOPREFIX,
        );
    }
    for (index, row) in card.rows.iter().enumerate() {
        let y = card.rows_top + card.row_h * index as i32;
        let label = rect(pad, y, card.width / 2 - pad, card.row_h);
        let value = rect(card.width / 2, y, card.width / 2 - pad, card.row_h);
        text_in(hdc, fonts.body, MUTED, label, &row.label, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        text_in(
            hdc,
            fonts.body,
            row.value_color,
            value,
            &row.value,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
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
    let word = text_size(hdc, fonts.title, &card.word);
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
        fonts.title,
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
                title: fonts.title,
                hero: fonts.hero,
                lead: fonts.lead,
                body: fonts.body,
            };
        }
        DeleteObject(fonts.title as HGDIOBJ);
        DeleteObject(fonts.hero as HGDIOBJ);
        DeleteObject(fonts.lead as HGDIOBJ);
        DeleteObject(fonts.body as HGDIOBJ);
    }
    let fonts = make_fonts(dpi);
    *slot = Some(Fonts {
        dpi: fonts.dpi,
        title: fonts.title,
        hero: fonts.hero,
        lead: fonts.lead,
        body: fonts.body,
    });
    fonts
}

unsafe fn make_fonts(dpi: i32) -> Fonts {
    Fonts {
        dpi,
        title: font(scale(16, dpi), FW_SEMIBOLD),
        hero: font(scale(32, dpi), FW_SEMIBOLD),
        lead: font(scale(20, dpi), FW_SEMIBOLD),
        body: font(scale(16, dpi), FW_NORMAL),
    }
}

unsafe fn font(height: i32, weight: u32) -> HFONT {
    let face = wide("Segoe UI");
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

unsafe fn fill_rect(hdc: windows_sys::Win32::Graphics::Gdi::HDC, x: i32, y: i32, w: i32, h: i32, color: u32) {
    let brush = CreateSolidBrush(color as COLORREF);
    let area = RECT { left: x, top: y, right: x + w, bottom: y + h };
    FillRect(hdc, &area, brush);
    DeleteObject(brush as HGDIOBJ);
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

unsafe fn make_icon(size: u32, remaining: u32, color: [u8; 3]) -> HICON {
    let pixels = remaining_icon(size, remaining, color);
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

fn write_state(kind: &str, left: Option<u32>, plan: &str, fail_code: Option<u16>) {
    let plan: String = plan.chars().filter(|ch| !ch.is_control()).take(40).collect();
    let body = if kind == "ready" {
        format!("state=ready\nleft={}\nplan={plan}\n", left.unwrap_or(0))
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
    let name = wide("TrayLeft");
    let value = wide(&path);
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

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn set_process_dpi() -> i32 {
    windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
        windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    )
}
