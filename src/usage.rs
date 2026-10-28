use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub plan: String,
    pub session_used: f64,
    pub weekly_used: Option<f64>,
    pub session_reset_at: Option<i64>,
    pub session_reset_after: Option<i64>,
    pub session_window_seconds: Option<i64>,
    pub weekly_window_seconds: Option<i64>,
    pub weekly_reset_at: Option<i64>,
    pub weekly_reset_after: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Session,
    Week,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    pub pane: Pane,
    pub name: &'static str,
    pub remaining: u32,
    pub reset_at: Option<i64>,
    pub reset_after: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchFail {
    NoLogin,
    Http(u16),
    Transport,
    Parse,
}

impl FetchFail {
    pub fn code(self) -> u16 {
        match self {
            FetchFail::NoLogin => 0,
            FetchFail::Transport => 1,
            FetchFail::Parse => 2,
            FetchFail::Http(status) => status,
        }
    }
}

pub fn plan_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.eq_ignore_ascii_case("prolite") {
        return "Pro 100".to_string();
    }
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub fn remaining_percent(used: f64) -> u32 {
    let used = used.clamp(0.0, 100.0);
    (100.0 - used).round().clamp(0.0, 100.0) as u32
}

pub fn mood_word(remaining: u32) -> &'static str {
    if remaining > 20 {
        "여유"
    } else if remaining > 5 {
        "주의"
    } else {
        "거의 없음"
    }
}

/// Light-palette wrapper kept for tests. The window asks `mood_rgb_on` so a dark taskbar can use brighter colors.
#[allow(dead_code)]
pub fn mood_rgb(remaining: Option<u32>) -> [u8; 3] {
    mood_rgb_on(remaining, true)
}

/// Status ink only. Comfort is teal, caution is amber, and the last sliver is red.
/// Unknown stays in the slate family so a missing number does not look like a warning.
pub fn mood_rgb_on(remaining: Option<u32>, light: bool) -> [u8; 3] {
    if light {
        match remaining {
            Some(value) if value > 20 => [15, 118, 110],
            Some(value) if value > 5 => [180, 83, 9],
            Some(_) => [176, 42, 55],
            None => [62, 77, 104],
        }
    } else {
        match remaining {
            Some(value) if value > 20 => [45, 212, 191],
            Some(value) if value > 5 => [251, 191, 36],
            Some(_) => [226, 104, 116],
            None => [152, 169, 196],
        }
    }
}

/// Ease-out cubic from `from` to `to` across `frames` steps.
/// Tick 0 is the old number. The last tick lands on the new number.
pub fn ease_count(from: i32, to: i32, tick: u8, frames: u8) -> i32 {
    if frames == 0 || tick >= frames {
        return to.clamp(0, 100);
    }
    let t = tick as f32 / frames as f32;
    let eased = 1.0 - (1.0 - t).powi(3);
    ((from as f32 + (to - from) as f32 * eased).round() as i32).clamp(0, 100)
}

/// Name a usage window from the length Codex sent.
/// The plan name is not used: Plus carries a 5-hour window, and Pro 100 does not.
pub fn window_name(seconds: Option<i64>, weekly_fallback: bool) -> &'static str {
    match seconds {
        Some(value) if (14_400..=21_600).contains(&value) => "5시간",
        Some(value) if (82_800..=90_000).contains(&value) => "하루",
        Some(value) if (518_400..=691_200).contains(&value) => "이번 주",
        None if weekly_fallback => "이번 주",
        _ => "이번 창",
    }
}

pub fn readings(snapshot: &Snapshot) -> (Reading, Option<Reading>) {
    let session_name = window_name(snapshot.session_window_seconds, false);
    let session = Reading {
        pane: Pane::Session,
        name: session_name,
        remaining: remaining_percent(snapshot.session_used),
        reset_at: snapshot.session_reset_at,
        reset_after: snapshot.session_reset_after,
    };
    let week = snapshot.weekly_used.map(|used| {
        let mut name = window_name(snapshot.weekly_window_seconds, true);
        if name == session_name {
            name = "다른 창";
        }
        Reading {
            pane: Pane::Week,
            name,
            remaining: remaining_percent(used),
            reset_at: snapshot.weekly_reset_at,
            reset_after: snapshot.weekly_reset_after,
        }
    });
    (session, week)
}

pub fn focus_reading(snapshot: &Snapshot) -> Reading {
    let (session, week) = readings(snapshot);
    match week {
        Some(week) if week.remaining < session.remaining => week,
        _ => session,
    }
}

/// `auto` keeps the tighter window. A missing week falls back to the window that exists.
pub fn focus_choice(snapshot: &Snapshot, choice: &str) -> Reading {
    let (session, week) = readings(snapshot);
    match choice {
        "session" => session,
        "week" => week.unwrap_or(session),
        _ => focus_reading(snapshot),
    }
}

pub fn hover_tip(snapshot: &Snapshot) -> String {
    let (session, week) = readings(snapshot);
    match week {
        Some(week) => format!(
            "Jusage · {} {}% · {} {}%",
            session.name, session.remaining, week.name, week.remaining
        ),
        None => format!("Jusage · {} {}% 남음", session.name, session.remaining),
    }
}

pub fn named_refill(name: &str, phrase: &str) -> String {
    let topic = format!("{name}{}", particle(name));
    if let Some(when) = phrase.strip_suffix("쯤 다시 채워져요") {
        if when.is_empty() {
            format!("{topic} 곧 다시 채워져요")
        } else {
            format!("{topic} {when}쯤 다시 채워져요")
        }
    } else if phrase == "곧 다시 채워져요" {
        format!("{topic} 곧 다시 채워져요")
    } else {
        format!("{topic} {phrase}")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Charge {
    pub seconds_left: i64,
    pub fraction: Option<f32>,
    pub phrase: String,
}

/// Time left until the window refills.
/// `fraction` is 1 just after a refill and 0 at the refill moment.
/// It needs the window length Codex sent. The phrase still works without it.
pub fn charge_state(now: i64, reset_at: i64, window_secs: Option<i64>) -> Option<Charge> {
    if reset_at < 1_000_000_000 {
        return None;
    }
    let seconds_left = (reset_at - now).max(0);
    let fraction = window_secs
        .filter(|secs| *secs > 0)
        .map(|secs| (seconds_left as f32 / secs as f32).clamp(0.0, 1.0));
    Some(Charge {
        seconds_left,
        fraction,
        phrase: charge_phrase(seconds_left),
    })
}

/// Earliest refill time Codex already told us, for either window.
pub fn soonest_reset(snapshot: &Snapshot, now: i64) -> Option<i64> {
    let session = reset_stamp(snapshot.session_reset_at, snapshot.session_reset_after, now);
    let week = reset_stamp(snapshot.weekly_reset_at, snapshot.weekly_reset_after, now);
    session.into_iter().chain(week).min()
}

fn reset_stamp(at: Option<i64>, after: Option<i64>, now: i64) -> Option<i64> {
    if let Some(at) = at.filter(|value| *value > 1_000_000_000) {
        return Some(at);
    }
    after
        .filter(|seconds| *seconds > 0)
        .map(|seconds| now.saturating_add(seconds))
}

/// When to ask again after a refill time.
/// A future refill is one check at that time.
/// A refill that already passed, while the number did not rise, is checked again soon, then stopped.
pub fn next_reset_watch(
    now: i64,
    reset_at: Option<i64>,
    remaining: u32,
    previous: Option<u32>,
    late_tries: u8,
) -> (Option<i64>, u8) {
    if reset_at.is_some_and(|at| at > now) {
        return (reset_at, 0);
    }
    let rose = previous.is_some_and(|before| remaining > before);
    if rose || reset_at.is_none() {
        return (None, 0);
    }
    let late_tries = late_tries.saturating_add(1);
    if late_tries >= 5 {
        (None, 0)
    } else {
        (Some(now.saturating_add(15)), late_tries)
    }
}

pub fn charge_phrase(seconds: i64) -> String {
    if seconds <= 0 {
        return "지금 채워져요".to_string();
    }
    let total_minutes = ((seconds + 59) / 60).max(1);
    if total_minutes < 60 {
        return format!("충전까지 {total_minutes}분");
    }
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours < 48 {
        return if minutes == 0 {
            format!("충전까지 {hours}시간")
        } else {
            format!("충전까지 {hours}시간 {minutes}분")
        };
    }
    let days = hours / 24;
    let hours = hours % 24;
    if hours == 0 {
        format!("충전까지 {days}일")
    } else {
        format!("충전까지 {days}일 {hours}시간")
    }
}

/// Movement of the Codex account since the first number seen today.
/// `recent_base` is the previous saved account number for the same window.
/// The same step is not repeated in both sentences.
pub fn account_move_value(open: u32, now: u32) -> String {
    if now < open {
        format!("{}% 줄었어요", open - now)
    } else if now > open {
        format!("{}% 채워졌어요", now - open)
    } else {
        "그대로예요".to_string()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    Steady,
    RefillsFirst,
    EmptyIn(i64),
}

pub const TRUST_LINE: &str = "이 PC에 이미 로그인된 Codex만 읽고, 비밀번호는 받지 않습니다.";
pub const PACE_GATHERING: &str = "모으는 중이에요";

struct Stretch {
    elapsed: i64,
    watched: i64,
    opened_left: u32,
    remaining: u32,
}

/// Samples since the last rise, with the current number appended.
fn pace_series(samples: &[(i64, u32)], now: i64, remaining: u32) -> Vec<(i64, u32)> {
    let mut start = 0usize;
    for index in 1..samples.len() {
        if samples[index].1 > samples[index - 1].1 {
            start = index;
        }
    }
    let mut series: Vec<(i64, u32)> = samples[start..]
        .iter()
        .copied()
        .filter(|(time, _)| *time <= now)
        .collect();
    if series.last().map(|(time, _)| *time) == Some(now) {
        if let Some(last) = series.last_mut() {
            last.1 = remaining;
        }
    } else {
        series.push((now, remaining));
    }
    series
}

/// One falling-or-flat stretch. A rise starts the stretch over.
/// Needs two samples and at least two minutes of watching.
fn stretch(samples: &[(i64, u32)], now: i64, remaining: u32) -> Option<Stretch> {
    let series = pace_series(samples, now, remaining);
    let (opened, opened_left) = *series.first()?;
    let watched = now.saturating_sub(opened);
    if series.len() < 2 || watched < 2 * 60 {
        return None;
    }
    // Idle time after the last real change must not stretch the pace.
    // The open panel redraws every 30 seconds, and that was moving the sentence
    // even while the remaining number stayed still.
    let mut end = opened;
    let mut previous = opened_left;
    for (time, left) in series.iter().skip(1) {
        if *left != previous {
            end = *time;
            previous = *left;
        }
    }
    let elapsed = end.saturating_sub(opened);
    Some(Stretch {
        elapsed,
        watched,
        opened_left,
        remaining,
    })
}

/// How fast the account number is falling.
/// Samples are `(unix, remaining)` for one window. A rise starts a new stretch.
/// Needs at least two minutes. `EmptyIn` is seconds from `now` until zero.
pub fn pace_outlook(
    samples: &[(i64, u32)],
    now: i64,
    remaining: u32,
    reset_at: Option<i64>,
) -> Option<Pace> {
    let stretch = stretch(samples, now, remaining)?;
    if stretch.opened_left <= stretch.remaining {
        return Some(Pace::Steady);
    }
    if stretch.elapsed < 2 * 60 {
        return None;
    }
    let drop = (stretch.opened_left - stretch.remaining) as i64;
    let seconds = (stretch.remaining as i64).saturating_mul(stretch.elapsed) / drop;
    if reset_at.is_some_and(|reset| now.saturating_add(seconds) >= reset) {
        return Some(Pace::RefillsFirst);
    }
    Some(Pace::EmptyIn(seconds))
}

/// What percent would still be left when this window refills, at the current pace.
/// Hidden while the pace is still gathering, and hidden when the number would hit zero first.
pub fn refill_guess(
    samples: &[(i64, u32)],
    now: i64,
    remaining: u32,
    reset_at: Option<i64>,
) -> Option<String> {
    let reset = reset_at.filter(|at| *at > now)?;
    let stretch = stretch(samples, now, remaining)?;
    if stretch.opened_left <= stretch.remaining {
        return Some(format!("충전 즈음 {}% 남을 듯", stretch.remaining));
    }
    if stretch.elapsed < 2 * 60 {
        return None;
    }
    let drop = (stretch.opened_left - stretch.remaining) as i64;
    let until = reset.saturating_sub(now);
    let lose = drop.saturating_mul(until) / stretch.elapsed;
    if lose >= stretch.remaining as i64 {
        return None;
    }
    let left = stretch.remaining - lose as u32;
    let shown = ((left + 2) / 5) * 5;
    if shown == 0 || shown > 100 {
        return None;
    }
    Some(format!("충전 즈음 {shown}% 남을 듯"))
}

/// Clock of this window against what was used. `seconds_left` is until the refill.
pub fn window_balance(window_seconds: i64, seconds_left: i64, remaining: u32) -> Option<String> {
    if window_seconds <= 0 || remaining > 100 {
        return None;
    }
    let used = 100 - remaining;
    let used_bit = if used == 0 {
        "아직 안 썼어요".to_string()
    } else {
        format!("{used}%를 썼어요")
    };
    if seconds_left <= 0 {
        return Some(format!("채워질 때가 됐고, {used_bit}"));
    }
    let left = seconds_left.min(window_seconds);
    let elapsed = window_seconds - left;
    if elapsed <= 0 {
        return Some(if used == 0 {
            "이 창은 방금 채워졌어요".to_string()
        } else {
            format!("이 창은 방금 채워졌고, {used_bit}")
        });
    }
    let passed =
        ((elapsed.saturating_mul(100) + window_seconds / 2) / window_seconds).clamp(0, 100);
    Some(format!("시간은 {passed}% 지났고, {used_bit}"))
}

/// How far the number has climbed back above today's lowest sample.
pub fn lifted_since_low(low: u32, now: u32) -> Option<String> {
    if now > low && now <= 100 {
        Some(format!("{}% 올라왔어요", now - low))
    } else {
        None
    }
}

/// What the usage check returned. Login loss is separate from a later network miss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchKind {
    Ok,
    NoLogin,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterFetch {
    Ready,
    KeepLast,
    NeedLogin,
    FirstFail,
}

/// A later failure keeps the last number. Login loss does not.
/// The first failure, before any number arrived, is still a full failure.
pub fn after_fetch(had_number: bool, kind: FetchKind) -> AfterFetch {
    match kind {
        FetchKind::Ok => AfterFetch::Ready,
        FetchKind::NoLogin => AfterFetch::NeedLogin,
        FetchKind::Failed if had_number => AfterFetch::KeepLast,
        FetchKind::Failed => AfterFetch::FirstFail,
    }
}

pub fn manual_refresh_keeps_number(had_number: bool) -> bool {
    had_number
}

pub const KEEP_LAST_NOTE: &str = "방금 확인은 못 했어요. 잠시 뒤 다시 확인해 주세요.";
pub const CHECKING_NOTE: &str = "확인 중";

/// One pane's cycle. `risen` is 1 after the number has climbed back to its start.
/// A smaller bounce keeps the low. Line is `session|week risen open low close`.
pub fn cycle_update(text: &str, pane: &str, remaining: u32) -> Option<String> {
    if (pane != "session" && pane != "week") || remaining > 100 {
        return None;
    }
    let mut rows = cycle_rows(text);
    let before = format_cycles(&rows);
    if let Some(row) = rows.iter_mut().find(|row| row.pane == pane) {
        if cycle_restart(row, remaining) {
            row.risen = true;
            row.open = remaining;
            row.low = remaining;
            row.close = remaining;
        } else if remaining > row.close {
            row.close = remaining;
        } else {
            row.close = remaining;
            row.low = row.low.min(remaining);
        }
    } else {
        rows.push(CycleRow {
            pane: pane.to_string(),
            risen: false,
            open: remaining,
            low: remaining,
            close: remaining,
        });
    }
    let body = format_cycles(&rows);
    if body == before {
        None
    } else {
        Some(body)
    }
}

pub fn cycle_restarted(text: &str, pane: &str, remaining: u32) -> bool {
    if (pane != "session" && pane != "week") || remaining > 100 {
        return false;
    }
    cycle_rows(text)
        .into_iter()
        .find(|row| row.pane == pane)
        .is_some_and(|row| cycle_restart(&row, remaining))
}

pub fn cycle_line(text: &str, pane: &str, remaining: u32) -> String {
    let Some(row) = cycle_rows(text)
        .into_iter()
        .rev()
        .find(|row| row.pane == pane)
    else {
        return String::new();
    };
    let head = if row.risen {
        "채워진 뒤"
    } else {
        "기록을 시작한 뒤"
    };
    let mut line = if remaining < row.open {
        format!("{head} {}% 썼어요", row.open - remaining)
    } else if remaining > row.open {
        format!("{head} {}% 채워졌어요", remaining - row.open)
    } else {
        format!("{head} 그대로예요")
    };
    if row.low < remaining {
        line.push_str(&format!(". 한때는 {}%까지 내려갔어요", row.low));
    }
    line
}

struct CycleRow {
    pane: String,
    risen: bool,
    open: u32,
    low: u32,
    close: u32,
}

fn cycle_restart(row: &CycleRow, remaining: u32) -> bool {
    remaining > row.close && remaining >= row.open
}

fn cycle_rows(text: &str) -> Vec<CycleRow> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(pane) = parts.next() else { continue };
        if pane != "session" && pane != "week" {
            continue;
        }
        let Some(flag) = parts.next() else { continue };
        let risen = match flag {
            "1" => true,
            "0" => false,
            _ => continue,
        };
        let (Some(open), Some(low), Some(close)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (Ok(open), Ok(low), Ok(close)) = (
            open.parse::<u32>(),
            low.parse::<u32>(),
            close.parse::<u32>(),
        ) else {
            continue;
        };
        if open > 100 || low > 100 || close > 100 {
            continue;
        }
        rows.push(CycleRow {
            pane: pane.to_string(),
            risen,
            open,
            low: low.min(open).min(close),
            close,
        });
    }
    rows
}

fn format_cycles(rows: &[CycleRow]) -> String {
    let mut body = String::new();
    for row in rows {
        let flag = if row.risen { 1 } else { 0 };
        body.push_str(&format!(
            "{} {flag} {} {} {}\n",
            row.pane, row.open, row.low, row.close
        ));
    }
    body
}

/// A second window can carry its own short pace. One window already says it in the body.
pub fn tile_brief(two_windows: bool, brief: &str) -> String {
    if two_windows && !brief.is_empty() {
        brief.to_string()
    } else {
        String::new()
    }
}

/// Average move on finished days in this strip. Today is still open, so it is left out.
/// Needs two finished days. Negative means the number fell.
pub fn average_day_value(slots: &[DaySlot]) -> Option<String> {
    let done: Vec<i32> = slots
        .iter()
        .filter(|slot| !slot.today)
        .filter_map(|slot| Some(slot.close? as i32 - slot.open? as i32))
        .collect();
    if done.len() < 2 {
        return None;
    }
    let sum: i32 = done.iter().sum();
    let count = done.len() as i32;
    let avg = if sum >= 0 {
        (sum + count / 2) / count
    } else {
        -((sum.abs() + count / 2) / count)
    };
    if avg < 0 {
        Some(format!("{}% 줄었어요", -avg))
    } else if avg > 0 {
        Some(format!("{}% 채워졌어요", avg))
    } else {
        Some("그대로예요".to_string())
    }
}

pub fn pace_line(
    samples: &[(i64, u32)],
    now: i64,
    remaining: u32,
    reset_at: Option<i64>,
) -> String {
    let Some(stretch) = stretch(samples, now, remaining) else {
        return PACE_GATHERING.to_string();
    };
    let Some(pace) = pace_outlook(samples, now, remaining, reset_at) else {
        return PACE_GATHERING.to_string();
    };
    let drop = stretch.opened_left.saturating_sub(stretch.remaining);
    let note = if stretch.elapsed < 60 * 60 || drop <= 1 {
        " (짧은 기록에 따른 단순 추정)"
    } else {
        ""
    };
    match pace {
        Pace::Steady => flat_phrase(stretch.watched),
        Pace::RefillsFirst => format!(
            "{} 동안 {drop}% 줄었고, 채워지기 전에 괜찮아요{note}",
            span_phrase(stretch.elapsed),
        ),
        Pace::EmptyIn(seconds) => format!(
            "{} 동안 {drop}% 줄어서, {} 바닥나요{note}",
            span_phrase(stretch.elapsed),
            pace_ahead(seconds)
        ),
    }
}

/// Short reason for the tray tooltip. Empty while the pace is still gathering.
pub fn pace_brief(samples: &[(i64, u32)], now: i64, remaining: u32) -> String {
    let Some(stretch) = stretch(samples, now, remaining) else {
        return String::new();
    };
    if stretch.opened_left <= stretch.remaining {
        return "그대로".to_string();
    }
    if stretch.elapsed < 2 * 60 {
        return String::new();
    }
    let drop = stretch.opened_left - stretch.remaining;
    format!("{}에 {drop}%", span_phrase(stretch.elapsed))
}

pub fn tray_tip(windows: &str, brief: &str) -> String {
    if brief.is_empty() {
        windows.to_string()
    } else {
        format!("{windows} · {brief}")
    }
}

/// The later half of this stretch is clearly faster or slower than the earlier half.
/// Close halves stay quiet so the row does not flicker.
pub fn pace_shift(samples: &[(i64, u32)], now: i64, remaining: u32) -> Option<String> {
    let series = pace_series(samples, now, remaining);
    if series.len() < 3 {
        return None;
    }
    let (opened, opened_left) = series[0];
    let mut end = opened;
    let mut previous = opened_left;
    for (time, left) in series.iter().skip(1) {
        if *left != previous {
            end = *time;
            previous = *left;
        }
    }
    if end <= opened {
        return None;
    }
    let mid = opened + (end - opened) / 2;
    let mid_point = series
        .iter()
        .take_while(|(time, _)| *time <= mid)
        .last()
        .copied()?;
    let early_elapsed = mid_point.0.saturating_sub(opened);
    let late_elapsed = end.saturating_sub(mid_point.0);
    if early_elapsed < 2 * 60 || late_elapsed < 2 * 60 {
        return None;
    }
    let early_drop = opened_left as i64 - mid_point.1 as i64;
    let late_drop = mid_point.1 as i64 - series.last()?.1 as i64;
    if early_drop <= 0 || late_drop <= 0 {
        return None;
    }
    let early_score = early_drop * late_elapsed;
    let late_score = late_drop * early_elapsed;
    if late_score * 2 > early_score * 3 {
        Some("아까보다 빨라졌어요".to_string())
    } else if early_score * 2 > late_score * 3 {
        Some("아까보다 느려졌어요".to_string())
    } else {
        None
    }
}

fn span_phrase(seconds: i64) -> String {
    let minutes = (seconds / 60).max(1);
    if minutes < 60 {
        return format!("{minutes}분");
    }
    let hours = minutes / 60;
    let rest = minutes % 60;
    if rest == 0 {
        format!("{hours}시간")
    } else {
        format!("{hours}시간 {rest}분")
    }
}

fn flat_phrase(watched: i64) -> String {
    if watched < 10 * 60 {
        return "지금은 안 줄어요".to_string();
    }
    if watched < 60 * 60 {
        let minutes = ((watched + 5 * 60) / (10 * 60)) * 10;
        if minutes >= 60 {
            return "약 1시간째 그대로예요".to_string();
        }
        return format!("약 {minutes}분째 그대로예요");
    }
    let hours = ((watched + 1800) / 3600).max(1);
    format!("약 {hours}시간째 그대로예요")
}

pub fn home_exe(local_app_data: &std::path::Path) -> PathBuf {
    local_app_data.join("Jusage").join("jusage.exe")
}

pub fn already_home(current: &std::path::Path, home: &std::path::Path) -> bool {
    norm_path(current) == norm_path(home)
}

fn norm_path(path: &std::path::Path) -> String {
    let text = path.to_string_lossy().replace('/', "\\");
    let text = text.trim_start_matches("\\\\?\\").trim_end_matches('\\');
    text.to_ascii_lowercase()
}

pub fn pace_value(pace: Pace) -> String {
    match pace {
        Pace::Steady => "지금은 안 줄어요".to_string(),
        Pace::RefillsFirst => "채워지기 전에 괜찮아요".to_string(),
        Pace::EmptyIn(seconds) => format!("{} 바닥나요", pace_ahead(seconds)),
    }
}

/// One window's saved samples. Lines are `session|week unix remaining`.
pub fn pace_rows(text: &str, pane: &str) -> Vec<(i64, u32)> {
    parse_pace(text)
        .into_iter()
        .filter(|(name, _, _)| name == pane)
        .map(|(_, at, left)| (at, left))
        .collect()
}

/// Append one sample. Same window and same number inside 30 seconds is skipped.
/// Keeps the newest 48 lines. Returns nothing when the file should stay as it is.
pub fn pace_record(text: &str, pane: &str, now: i64, left: u32) -> Option<String> {
    if (pane != "session" && pane != "week") || left > 100 {
        return None;
    }
    let mut rows = parse_pace(text);
    if let Some((_, at, value)) = rows.iter().rev().find(|(name, _, _)| name == pane) {
        if *value == left && now >= *at && now.saturating_sub(*at) < 30 {
            return None;
        }
    }
    rows.push((pane.to_string(), now, left));
    if rows.len() > 48 {
        let extra = rows.len() - 48;
        rows.drain(0..extra);
    }
    let mut body = String::new();
    for (name, at, value) in rows {
        body.push_str(&format!("{name} {at} {value}\n"));
    }
    Some(body)
}

fn parse_pace(text: &str) -> Vec<(String, i64, u32)> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(pane) = parts.next() else {
            continue;
        };
        if pane != "session" && pane != "week" {
            continue;
        }
        let (Some(at), Some(left)) = (parts.next(), parts.next()) else {
            continue;
        };
        let (Ok(at), Ok(left)) = (at.parse::<i64>(), left.parse::<u32>()) else {
            continue;
        };
        if left > 100 {
            continue;
        }
        rows.push((pane.to_string(), at, left));
    }
    rows
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DaySlot {
    pub name: &'static str,
    pub open: Option<u32>,
    pub close: Option<u32>,
    pub low: Option<u32>,
    pub today: bool,
}

/// Seven local days ending today. Missing days stay empty. Today is last.
pub fn recent_days(text: &str, pane: &str, year: i32, month: u8, day: u8) -> [DaySlot; 7] {
    let saved = day_rows(text);
    let mut cursor = (year, month, day);
    let mut dates = Vec::with_capacity(7);
    for _ in 0..7 {
        dates.push(cursor);
        cursor = prev_date(cursor.0, cursor.1, cursor.2);
    }
    dates.reverse();
    let mut slots = [DaySlot {
        name: "월",
        open: None,
        close: None,
        low: None,
        today: false,
    }; 7];
    for (index, (year, month, day)) in dates.into_iter().enumerate() {
        let key = format!("{year:04}-{month:02}-{day:02}");
        let found = saved
            .iter()
            .rev()
            .find(|(name, date, _, _, _)| name == pane && date == &key);
        slots[index] = DaySlot {
            name: weekday_name(year, month, day),
            open: found.map(|row| row.2),
            close: found.map(|row| row.3),
            low: found.map(|row| row.4),
            today: index == 6,
        };
    }
    slots
}

/// One line is `session|week YYYY-MM-DD open close low`.
/// `low` is the smallest number in the current cycle for that day.
/// Rows older than 14 days are dropped. Returns nothing when the file is unchanged.
pub fn day_record(
    text: &str,
    pane: &str,
    date: &str,
    open: u32,
    close: u32,
    restarted: bool,
) -> Option<String> {
    if (pane != "session" && pane != "week") || open > 100 || close > 100 {
        return None;
    }
    let Some((year, month, day)) = parse_ymd(date) else {
        return None;
    };
    let cutoff = julian_day(year, month, day) - 13;
    let mut rows = day_rows(text);
    let before = format_days(&rows);
    let low = open.min(close);
    if let Some(row) = rows
        .iter_mut()
        .find(|(name, saved, _, _, _)| name == pane && saved == date)
    {
        row.2 = open;
        row.3 = close;
        row.4 = if restarted { low } else { row.4.min(low) };
    } else {
        rows.push((pane.to_string(), date.to_string(), open, close, low));
    }
    rows.retain(|(name, saved, _, _, _)| {
        (name == "session" || name == "week")
            && parse_ymd(saved).is_some_and(|(y, m, d)| julian_day(y, m, d) >= cutoff)
    });
    let mut kept: Vec<(String, String, u32, u32, u32)> = Vec::new();
    for pane_name in ["session", "week"] {
        let mut one: Vec<_> = rows
            .iter()
            .filter(|(name, _, _, _, _)| name == pane_name)
            .cloned()
            .collect();
        one.sort_by(|a, b| a.1.cmp(&b.1));
        if one.len() > 14 {
            one.drain(0..one.len() - 14);
        }
        kept.extend(one);
    }
    let body = format_days(&kept);
    if body == before {
        None
    } else {
        Some(body)
    }
}

fn day_rows(text: &str) -> Vec<(String, String, u32, u32, u32)> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let Some(pane) = parts.next() else { continue };
        if pane != "session" && pane != "week" {
            continue;
        }
        let Some(date) = parts.next() else { continue };
        if parse_ymd(date).is_none() {
            continue;
        }
        let (Some(open), Some(close)) = (parts.next(), parts.next()) else {
            continue;
        };
        let (Ok(open), Ok(close)) = (open.parse::<u32>(), close.parse::<u32>()) else {
            continue;
        };
        if open > 100 || close > 100 {
            continue;
        }
        let low = match parts.next() {
            Some(raw) => match raw.parse::<u32>() {
                Ok(value) if value <= 100 => value.min(open).min(close),
                _ => continue,
            },
            None => open.min(close),
        };
        rows.push((pane.to_string(), date.to_string(), open, close, low));
    }
    rows
}

fn format_days(rows: &[(String, String, u32, u32, u32)]) -> String {
    let mut body = String::new();
    for (pane, date, open, close, low) in rows {
        body.push_str(&format!("{pane} {date} {open} {close} {low}\n"));
    }
    body
}

pub fn week_story(slots: &[DaySlot]) -> String {
    let known: Vec<&DaySlot> = slots.iter().filter(|slot| slot.open.is_some()).collect();
    if known.is_empty() {
        return "기록이 쌓이면 최근 7일을 보여드려요".to_string();
    }
    if known.len() == 1 && known[0].today {
        return "오늘은 여기부터 기록해요".to_string();
    }
    let open = known[0].open.unwrap_or(0);
    let close = known[known.len() - 1].close.unwrap_or(open);
    let mut net = if close < open {
        format!("이 기록에서 {}% 줄었어요", open - close)
    } else if close > open {
        format!("이 기록에서 {}% 채워졌어요", close - open)
    } else {
        "이 기록은 그대로예요".to_string()
    };
    let mut best: Option<(&str, u32)> = None;
    for slot in &known {
        let (Some(opened), Some(closed)) = (slot.open, slot.close) else {
            continue;
        };
        if opened > closed {
            let drop = opened - closed;
            if best.map(|(_, was)| drop > was).unwrap_or(true) {
                best = Some((slot.name, drop));
            }
        }
    }
    if let Some((name, _)) = best {
        net.push_str(&format!(". {name}요일이 가장 많이 줄었어요"));
    }
    net
}

pub fn yesterday_close(slots: &[DaySlot]) -> Option<u32> {
    let today = slots.iter().position(|slot| slot.today)?;
    if today == 0 {
        return None;
    }
    slots[today - 1].close
}

/// Lowest point on an earlier day, when it went under today's lowest.
pub fn earlier_low(slots: &[DaySlot]) -> Option<(&'static str, u32)> {
    let today_low = slots
        .iter()
        .find(|slot| slot.today)
        .and_then(|slot| slot.low);
    let mut best: Option<(&'static str, u32)> = None;
    for slot in slots.iter().filter(|slot| !slot.today) {
        let Some(low) = slot.low else { continue };
        if today_low.is_some_and(|today| low >= today) {
            continue;
        }
        if best.map(|(_, was)| low < was).unwrap_or(true) {
            best = Some((slot.name, low));
        }
    }
    best
}

fn parse_ymd(date: &str) -> Option<(i32, u8, u8)> {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return None;
    }
    let year = date[0..4].parse().ok()?;
    let month = date[5..7].parse().ok()?;
    let day = date[8..10].parse().ok()?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn prev_date(year: i32, month: u8, day: u8) -> (i32, u8, u8) {
    if day > 1 {
        return (year, month, day - 1);
    }
    if month > 1 {
        let month = month - 1;
        return (year, month, days_in_month(year, month));
    }
    (year - 1, 12, 31)
}

pub fn weekday_name(year: i32, month: u8, day: u8) -> &'static str {
    let names = ["월", "화", "수", "목", "금", "토", "일"];
    let delta = julian_day(year, month, day) - julian_day(1970, 1, 1);
    let index = (delta.rem_euclid(7) + 3).rem_euclid(7) as usize;
    names[index]
}

/// Rough pace. Exact minutes were redrawn every 30 seconds and kept drifting.
/// Under an hour, 10-minute steps. After that, whole hours.
fn pace_ahead(seconds: i64) -> String {
    if seconds < 10 * 60 {
        return "곧".to_string();
    }
    if seconds < 60 * 60 {
        let minutes = ((seconds + 5 * 60) / (10 * 60)) * 10;
        if minutes >= 60 {
            return "약 1시간 뒤".to_string();
        }
        return format!("약 {}분 뒤", minutes.max(10));
    }
    let hours = ((seconds + 1800) / 3600).max(1);
    if hours < 48 {
        return format!("약 {hours}시간 뒤");
    }
    let days = hours / 24;
    let rest = hours % 24;
    if rest == 0 {
        format!("약 {days}일 뒤")
    } else {
        format!("약 {days}일 {rest}시간 뒤")
    }
}

pub fn account_moves(
    open: u32,
    now: u32,
    recent_base: Option<u32>,
    recent: Option<String>,
) -> (String, String) {
    let moved = open != now;
    let mut day = if now < open {
        format!("오늘 이 계정에서 {}% 줄었어요", open - now)
    } else if now > open {
        format!("오늘 이 계정에서 {}% 채워졌어요", now - open)
    } else {
        String::new()
    };
    let recent = match recent {
        Some(line) if !(moved && recent_base == Some(open)) => line,
        _ => String::new(),
    };
    if day.is_empty() && recent.is_empty() {
        day = "오늘 이 계정은 그대로예요".to_string();
    }
    (day, recent)
}

pub fn change_line(age_secs: i64, before: u32, now: u32) -> Option<String> {
    if before == now {
        return None;
    }
    let age = age_secs.max(0);
    let when = if age < 60 {
        "방금".to_string()
    } else if age < 3_600 {
        format!("{}분 전", (age / 60).max(1))
    } else if age < 86_400 {
        format!("{}시간 전", (age / 3_600).max(1))
    } else if age < 86_400 * 2 {
        "어제".to_string()
    } else {
        format!("{}일 전", (age / 86_400).max(2))
    };
    let (amount, verb) = if now < before {
        (before - now, "줄었어요")
    } else {
        (now - before, "채워졌어요")
    };
    Some(format!("{when}보다 {amount}% {verb}"))
}

fn particle(name: &str) -> &'static str {
    let Some(ch) = name.chars().last() else {
        return "은";
    };
    let code = ch as u32;
    if (0xAC00..=0xD7A3).contains(&code) && (code - 0xAC00) % 28 == 0 {
        "는"
    } else {
        "은"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Civil {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

pub fn clock_when(now: Civil, at: Civil) -> String {
    if civil_minutes(at) <= civil_minutes(now) {
        return "곧".to_string();
    }
    let day = julian_day(at.year, at.month, at.day) - julian_day(now.year, now.month, now.day);
    match day {
        0 => format!("오늘 {}", clock_korean(at.hour, at.minute)),
        1 => format!("내일 {}", clock_korean(at.hour, at.minute)),
        _ => format!(
            "{}월 {}일 {}",
            at.month,
            at.day,
            clock_korean(at.hour, at.minute)
        ),
    }
}

pub fn refill_phrase(now: Civil, at: Civil) -> String {
    let now_min = civil_minutes(now);
    let at_min = civil_minutes(at);
    if at_min <= now_min {
        return "곧 다시 채워져요".to_string();
    }
    let day = julian_day(at.year, at.month, at.day) - julian_day(now.year, now.month, now.day);
    let when = match day {
        0 => format!("오늘 {}", clock_korean(at.hour, at.minute)),
        1 => format!("내일 {}", clock_korean(at.hour, at.minute)),
        _ => format!(
            "{}월 {}일 {}",
            at.month,
            at.day,
            clock_korean(at.hour, at.minute)
        ),
    };
    format!("{when}쯤 다시 채워져요")
}

fn civil_minutes(time: Civil) -> i64 {
    julian_day(time.year, time.month, time.day) * 24 * 60
        + time.hour as i64 * 60
        + time.minute as i64
}

fn clock_korean(hour: u8, minute: u8) -> String {
    let (half, hour) = match hour {
        0 => ("오전", 12),
        1..=11 => ("오전", hour),
        12 => ("오후", 12),
        _ => ("오후", hour - 12),
    };
    if minute == 0 {
        format!("{half} {hour}시")
    } else {
        format!("{half} {hour}시 {minute}분")
    }
}

fn julian_day(year: i32, month: u8, day: u8) -> i64 {
    let y = year as i64;
    let m = month as i64;
    let d = day as i64;
    let a = (14 - m) / 12;
    let y = y + 4800 - a;
    let m = m + 12 * a - 3;
    d + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045
}

pub fn parse_usage(headers: &[(&str, &str)], body: &str) -> Option<Snapshot> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let mut session = number_at(&json, "/rate_limit/primary_window/used_percent");
    let mut weekly = number_at(&json, "/rate_limit/secondary_window/used_percent");
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("x-codex-primary-used-percent") {
            if let Ok(number) = value.trim().parse::<f64>() {
                session = Some(number);
            }
        } else if name.eq_ignore_ascii_case("x-codex-secondary-used-percent") {
            if let Ok(number) = value.trim().parse::<f64>() {
                weekly = Some(number);
            }
        }
    }
    let window = json.pointer("/rate_limit/primary_window");
    let weekly_window = json.pointer("/rate_limit/secondary_window");
    Some(Snapshot {
        plan: json
            .get("plan_type")
            .and_then(|value| value.as_str())
            .map(plan_label)
            .unwrap_or_default(),
        session_used: session?.clamp(0.0, 100.0),
        weekly_used: weekly.map(|value| value.clamp(0.0, 100.0)),
        session_reset_at: window.and_then(|value| unix_field(value, "reset_at")),
        session_reset_after: window.and_then(|value| unix_field(value, "reset_after_seconds")),
        session_window_seconds: window.and_then(|value| unix_field(value, "limit_window_seconds")),
        weekly_window_seconds: weekly_window
            .and_then(|value| unix_field(value, "limit_window_seconds")),
        weekly_reset_at: weekly_window.and_then(|value| unix_field(value, "reset_at")),
        weekly_reset_after: weekly_window
            .and_then(|value| unix_field(value, "reset_after_seconds")),
    })
}

fn unix_field(window: &serde_json::Value, name: &str) -> Option<i64> {
    let value = window.get(name)?;
    let number = value
        .as_f64()
        .or_else(|| value.as_i64().map(|item| item as f64))?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    let whole = number.round() as i64;
    if name == "reset_at" && whole > 10_000_000_000 {
        Some(whole / 1000)
    } else {
        Some(whole)
    }
}

#[cfg(windows)]
fn build_agent() -> Result<ureq::Agent, FetchFail> {
    let tls = native_tls::TlsConnector::new().map_err(|_| FetchFail::Transport)?;
    Ok(ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .tls_connector(std::sync::Arc::new(tls))
        .build())
}

#[cfg(not(windows))]
fn build_agent() -> Result<ureq::Agent, FetchFail> {
    Ok(ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .build())
}

fn number_at(json: &serde_json::Value, pointer: &str) -> Option<f64> {
    let value = json.pointer(pointer)?;
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|number| number as f64))
}

pub fn codex_home() -> PathBuf {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        return PathBuf::from(home);
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(profile).join(".codex");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".codex");
    }
    PathBuf::from(".codex")
}

pub fn read_login(home: &Path) -> Result<(String, Option<String>), FetchFail> {
    let text = std::fs::read_to_string(home.join("auth.json")).map_err(|_| FetchFail::NoLogin)?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|_| FetchFail::NoLogin)?;
    let token = json
        .pointer("/tokens/access_token")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(FetchFail::NoLogin)?
        .to_string();
    let account = json
        .pointer("/tokens/account_id")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    Ok((token, account))
}

pub fn fetch_usage(home: &Path) -> Result<Snapshot, FetchFail> {
    let (token, account) = read_login(home)?;
    let agent = build_agent()?;
    let mut request = agent
        .get("https://chatgpt.com/backend-api/wham/usage")
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/json")
        .set("User-Agent", "jusage/0.3");
    if let Some(account) = account.as_deref() {
        request = request.set("ChatGPT-Account-Id", account);
    }
    let response = match request.call() {
        Ok(response) => response,
        Err(ureq::Error::Status(status, _)) if status == 401 || status == 403 => {
            return Err(FetchFail::NoLogin);
        }
        Err(ureq::Error::Status(status, _)) => return Err(FetchFail::Http(status)),
        Err(_) => return Err(FetchFail::Transport),
    };
    let headers: Vec<(String, String)> = response
        .headers_names()
        .into_iter()
        .filter_map(|name| {
            response
                .header(&name)
                .map(|value| (name, value.to_string()))
        })
        .collect();
    let body = response.into_string().map_err(|_| FetchFail::Transport)?;
    let header_refs: Vec<(&str, &str)> = headers
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    parse_usage(&header_refs, &body).ok_or(FetchFail::Parse)
}

pub fn newest_activity(home: &Path) -> Option<SystemTime> {
    let sessions = home.join("sessions");
    let root = if sessions.is_dir() {
        sessions
    } else {
        home.to_path_buf()
    };
    let mut newest = None;
    let mut seen = 0u32;
    walk_times(&root, 4, &mut newest, &mut seen);
    newest
}

fn walk_times(dir: &Path, depth: u8, newest: &mut Option<SystemTime>, seen: &mut u32) {
    if depth == 0 || *seen > 2_000 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        *seen += 1;
        if *seen > 2_000 {
            return;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            walk_times(&entry.path(), depth - 1, newest, seen);
            continue;
        }
        if let Ok(modified) = meta.modified() {
            *newest = Some(match *newest {
                Some(current) if current > modified => current,
                _ => modified,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_pro_100_for_the_internal_name() {
        let body = r#"{"plan_type":"prolite","rate_limit":{"primary_window":{"used_percent":39}}}"#;
        let snapshot = parse_usage(&[], body).unwrap();
        assert_eq!(snapshot.plan, "Pro 100");
        assert_eq!(remaining_percent(snapshot.session_used), 61);
        assert_eq!(snapshot.weekly_used, None);
    }

    #[test]
    fn header_percent_wins_over_body() {
        let body = r#"{"plan_type":"plus","rate_limit":{"primary_window":{"used_percent":10},"secondary_window":{"used_percent":4}}}"#;
        let snapshot = parse_usage(&[("x-codex-primary-used-percent", "25")], body).unwrap();
        assert_eq!(snapshot.plan, "Plus");
        assert_eq!(snapshot.session_used, 25.0);
        assert_eq!(snapshot.weekly_used, Some(4.0));
    }

    #[test]
    fn missing_session_is_not_a_snapshot() {
        assert!(parse_usage(&[], r#"{"plan_type":"pro"}"#).is_none());
    }

    #[test]
    fn other_plan_names_stay_plain() {
        assert_eq!(plan_label("pro"), "Pro");
        assert_eq!(plan_label("PROLITE"), "Pro 100");
        assert_eq!(plan_label("team"), "Team");
    }

    #[test]
    fn reads_the_session_reset_clock() {
        let body = r#"{"plan_type":"prolite","rate_limit":{"primary_window":{"used_percent":10,"reset_at":1738300000,"reset_after_seconds":90}}}"#;
        let snapshot = parse_usage(&[], body).unwrap();
        assert_eq!(snapshot.session_reset_at, Some(1_738_300_000));
        assert_eq!(snapshot.session_reset_after, Some(90));
    }

    #[test]
    fn refill_line_uses_today_tomorrow_or_a_date() {
        let now = Civil {
            year: 2026,
            month: 10,
            day: 27,
            hour: 15,
            minute: 0,
        };
        assert_eq!(
            refill_phrase(
                now,
                Civil {
                    hour: 21,
                    minute: 0,
                    ..now
                }
            ),
            "오늘 오후 9시쯤 다시 채워져요"
        );
        assert_eq!(
            refill_phrase(
                now,
                Civil {
                    day: 28,
                    hour: 0,
                    minute: 10,
                    ..now
                }
            ),
            "내일 오전 12시 10분쯤 다시 채워져요"
        );
        assert_eq!(
            refill_phrase(
                now,
                Civil {
                    day: 30,
                    hour: 16,
                    minute: 0,
                    ..now
                }
            ),
            "10월 30일 오후 4시쯤 다시 채워져요"
        );
        assert_eq!(
            refill_phrase(
                now,
                Civil {
                    hour: 14,
                    minute: 0,
                    ..now
                }
            ),
            "곧 다시 채워져요"
        );
    }

    #[test]
    fn mood_follows_how_much_is_left() {
        assert_eq!(mood_word(58), "여유");
        assert_eq!(mood_word(20), "주의");
        assert_eq!(mood_word(5), "거의 없음");
        assert_eq!(mood_rgb(Some(58)), [15, 118, 110]);
        assert_eq!(mood_rgb(Some(12)), [180, 83, 9]);
        assert_eq!(mood_rgb(Some(5)), [176, 42, 55]);
        assert_eq!(mood_rgb(None), [62, 77, 104]);
        assert_eq!(mood_rgb_on(Some(58), false), [45, 212, 191]);
        assert_eq!(mood_rgb_on(Some(12), false), [251, 191, 36]);
        assert_eq!(mood_rgb_on(Some(5), false), [226, 104, 116]);
        assert_eq!(mood_rgb_on(None, false), [152, 169, 196]);
    }

    #[test]
    fn the_count_starts_at_the_old_number_and_lands_on_the_new_one() {
        assert_eq!(ease_count(43, 28, 0, 20), 43);
        let mid = ease_count(43, 28, 10, 20);
        assert!(mid < 43 && mid > 28, "mid was {mid}");
        assert_eq!(ease_count(43, 28, 20, 20), 28);
        assert_eq!(ease_count(10, 40, 20, 20), 40);
    }

    #[test]
    fn plus_shape_names_the_five_hour_window_and_the_week() {
        let body = r#"{"plan_type":"plus","rate_limit":{"primary_window":{"used_percent":40,"limit_window_seconds":18000,"reset_at":1738300000},"secondary_window":{"used_percent":12,"limit_window_seconds":604800}}}"#;
        let snapshot = parse_usage(&[], body).unwrap();
        assert_eq!(snapshot.plan, "Plus");
        assert_eq!(snapshot.session_window_seconds, Some(18_000));
        assert_eq!(snapshot.weekly_window_seconds, Some(604_800));
        assert_eq!(window_name(snapshot.session_window_seconds, false), "5시간");
        assert_eq!(window_name(snapshot.weekly_window_seconds, true), "이번 주");
        assert_eq!(remaining_percent(snapshot.session_used), 60);
        assert_eq!(remaining_percent(snapshot.weekly_used.unwrap()), 88);
    }

    #[test]
    fn a_week_only_window_is_not_called_five_hours() {
        let body = r#"{"plan_type":"prolite","rate_limit":{"primary_window":{"used_percent":42,"limit_window_seconds":604800}}}"#;
        let snapshot = parse_usage(&[], body).unwrap();
        assert_eq!(snapshot.plan, "Pro 100");
        assert_eq!(snapshot.weekly_used, None);
        assert_eq!(
            window_name(snapshot.session_window_seconds, false),
            "이번 주"
        );
    }

    #[test]
    fn missing_window_length_stays_generic() {
        assert_eq!(window_name(None, false), "이번 창");
        assert_eq!(window_name(None, true), "이번 주");
        assert_eq!(window_name(Some(3_600), false), "이번 창");
    }

    #[test]
    fn the_lower_window_becomes_the_focus() {
        let plus = r#"{"plan_type":"plus","rate_limit":{"primary_window":{"used_percent":20,"limit_window_seconds":18000,"reset_at":2000000000},"secondary_window":{"used_percent":80,"limit_window_seconds":604800,"reset_at":2000600000}}}"#;
        let tight_week = parse_usage(&[], plus).unwrap();
        assert_eq!(focus_reading(&tight_week).pane, Pane::Week);
        assert_eq!(focus_reading(&tight_week).remaining, 20);
        assert_eq!(hover_tip(&tight_week), "Jusage · 5시간 80% · 이번 주 20%");
        let session_only = parse_usage(&[], r#"{"plan_type":"prolite","rate_limit":{"primary_window":{"used_percent":42,"limit_window_seconds":604800}}}"#).unwrap();
        assert_eq!(focus_reading(&session_only).name, "이번 주");
        assert_eq!(focus_reading(&session_only).remaining, 58);
        assert_eq!(focus_choice(&tight_week, "session").pane, Pane::Session);
        assert_eq!(focus_choice(&tight_week, "week").remaining, 20);
        assert_eq!(focus_choice(&session_only, "week").name, "이번 주");
        assert_eq!(focus_choice(&tight_week, "auto").pane, Pane::Week);
    }

    #[test]
    fn clock_when_is_the_short_local_time() {
        let now = Civil {
            year: 2026,
            month: 10,
            day: 28,
            hour: 15,
            minute: 0,
        };
        assert_eq!(
            clock_when(
                now,
                Civil {
                    hour: 21,
                    minute: 12,
                    ..now
                }
            ),
            "오늘 오후 9시 12분"
        );
        assert_eq!(
            clock_when(
                now,
                Civil {
                    day: 29,
                    hour: 9,
                    minute: 0,
                    ..now
                }
            ),
            "내일 오전 9시"
        );
        assert_eq!(
            clock_when(
                now,
                Civil {
                    hour: 9,
                    minute: 0,
                    ..now
                }
            ),
            "곧"
        );
    }

    #[test]
    fn refill_names_the_window_in_one_line() {
        let now = Civil {
            year: 2026,
            month: 10,
            day: 27,
            hour: 15,
            minute: 0,
        };
        let today = refill_phrase(
            now,
            Civil {
                hour: 21,
                minute: 0,
                ..now
            },
        );
        assert_eq!(
            named_refill("5시간", &today),
            "5시간은 오늘 오후 9시쯤 다시 채워져요"
        );
        assert_eq!(
            named_refill("이번 주", "곧 다시 채워져요"),
            "이번 주는 곧 다시 채워져요"
        );
    }

    #[test]
    fn charge_counts_down_to_the_refill() {
        assert_eq!(charge_phrase(0), "지금 채워져요");
        assert_eq!(charge_phrase(90), "충전까지 2분");
        assert_eq!(charge_phrase(3 * 3600), "충전까지 3시간");
        assert_eq!(charge_phrase(4 * 3600 + 12 * 60), "충전까지 4시간 12분");
        assert_eq!(charge_phrase(2 * 86_400 + 3 * 3600), "충전까지 2일 3시간");
        let now = 1_800_000_000i64;
        let half = charge_state(now, now + 9_000, Some(18_000)).unwrap();
        assert!((half.fraction.unwrap() - 0.5).abs() < 0.001);
        let empty = charge_state(now, now - 10, Some(18_000)).unwrap();
        assert_eq!(empty.fraction, Some(0.0));
        assert_eq!(empty.phrase, "지금 채워져요");
        let full = charge_state(now, now + 20_000, Some(18_000)).unwrap();
        assert_eq!(full.fraction, Some(1.0));
        let words_only = charge_state(now, now + 120, None).unwrap();
        assert_eq!(words_only.fraction, None);
        assert_eq!(words_only.phrase, "충전까지 2분");
    }

    #[test]
    fn a_passed_refill_is_checked_again_until_the_number_rises() {
        let now = 1_800_000_000i64;
        let week_sooner = parse_usage(&[], r#"{"plan_type":"plus","rate_limit":{"primary_window":{"used_percent":10,"reset_at":1800001000},"secondary_window":{"used_percent":40,"reset_at":1800000500}}}"#).unwrap();
        assert_eq!(soonest_reset(&week_sooner, now), Some(1_800_000_500));
        let (at, tries) = next_reset_watch(now, Some(now - 5), 18, Some(18), 0);
        assert_eq!(at, Some(now + 15));
        assert_eq!(tries, 1);
        let (at, tries) = next_reset_watch(now, Some(now - 5), 18, Some(18), 4);
        assert_eq!(at, None);
        assert_eq!(tries, 0);
        let (at, tries) = next_reset_watch(now, Some(now - 5), 100, Some(18), 2);
        assert_eq!(at, None);
        assert_eq!(tries, 0);
        let (at, tries) = next_reset_watch(now, Some(now + 3_600), 18, Some(21), 3);
        assert_eq!(at, Some(now + 3_600));
        assert_eq!(tries, 0);
        let (at, _) = next_reset_watch(now, None, 18, Some(18), 0);
        assert_eq!(at, None);
    }

    #[test]
    fn pace_says_when_the_account_would_run_out() {
        let now = 1_800_000_000i64;
        let hour = [(now - 3600, 95u32), (now, 90)];
        assert_eq!(
            pace_outlook(&hour, now, 90, None),
            Some(Pace::EmptyIn(18 * 3600))
        );
        assert_eq!(
            pace_value(Pace::EmptyIn(18 * 3600)),
            "약 18시간 뒤 바닥나요"
        );
        let held = [(now - 3600, 95u32), (now - 1800, 90)];
        assert_eq!(
            pace_outlook(&held, now - 1800, 90, None),
            pace_outlook(&held, now, 90, None)
        );
        assert_ne!(
            pace_outlook(&held, now, 90, None),
            pace_outlook(&held, now, 85, None)
        );
        assert_eq!(
            pace_outlook(&hour, now, 90, Some(now + 2 * 3600)),
            Some(Pace::RefillsFirst)
        );
        assert_eq!(
            pace_outlook(&[(now - 3600, 90), (now, 90)], now, 90, None),
            Some(Pace::Steady)
        );
        assert_eq!(pace_outlook(&[(now - 60, 95)], now, 90, None), None);
        assert_eq!(
            pace_outlook(&[(now - 180, 95)], now, 90, None),
            Some(Pace::EmptyIn(3_240))
        );
        let reset = [(now - 7200, 40u32), (now - 3600, 90), (now, 80)];
        assert_eq!(
            pace_outlook(&reset, now, 80, None),
            Some(Pace::EmptyIn(8 * 3600))
        );
        assert_eq!(account_move_value(99, 95), "4% 줄었어요");
        assert_eq!(account_move_value(95, 95), "그대로예요");
    }

    #[test]
    fn pace_file_keeps_one_window_and_skips_a_repeat() {
        let now = 1_800_000_000i64;
        let first = pace_record("", "session", now, 90).unwrap();
        assert_eq!(first, "session 1800000000 90\n");
        assert!(pace_record(&first, "session", now + 10, 90).is_none());
        let second = pace_record(&first, "session", now + 40, 88).unwrap();
        assert_eq!(
            pace_rows(&second, "session"),
            vec![(now, 90), (now + 40, 88)]
        );
        let mixed = pace_record(&second, "week", now + 50, 70).unwrap();
        assert_eq!(pace_rows(&mixed, "week"), vec![(now + 50, 70)]);
        assert_eq!(pace_rows(&mixed, "session").len(), 2);
        assert!(pace_record("token secret", "nope", now, 1).is_none());
        let mut text = String::new();
        for index in 0..50 {
            text = pace_record(&text, "session", now + index * 60, 80).unwrap();
        }
        let kept = pace_rows(&text, "session");
        assert_eq!(kept.len(), 48);
        assert_eq!(kept[0], (now + 2 * 60, 80));
        assert_eq!(*kept.last().unwrap(), (now + 49 * 60, 80));
    }

    #[test]
    fn pace_line_says_it_is_still_gathering() {
        let now = 1_800_000_000i64;
        assert_eq!(pace_line(&[], now, 90, None), "모으는 중이에요");
        assert_eq!(
            pace_line(&[(now - 60, 95)], now, 90, None),
            "모으는 중이에요"
        );
        assert_eq!(
            pace_line(&[(now - 180, 95)], now, 90, None),
            "3분 동안 5% 줄어서, 약 50분 뒤 바닥나요 (짧은 기록에 따른 단순 추정)"
        );
        assert_eq!(
            pace_line(&[(now - 45 * 60, 90)], now, 89, None),
            "45분 동안 1% 줄어서, 약 2일 19시간 뒤 바닥나요 (짧은 기록에 따른 단순 추정)"
        );
        let held = [(now - 3600, 95u32), (now - 1800, 90)];
        assert_eq!(
            pace_line(&held, now - 1800, 90, None),
            pace_line(&held, now, 90, None)
        );
        assert_eq!(
            pace_line(&[(now - 3600, 90), (now, 90)], now, 90, None),
            "약 1시간째 그대로예요"
        );
        assert_eq!(
            pace_line(&[(now - 3600, 100)], now, 90, Some(now + 2 * 3600)),
            "1시간 동안 10% 줄었고, 채워지기 전에 괜찮아요"
        );
        assert_eq!(pace_brief(&[(now - 180, 95)], now, 90), "3분에 5%");
        assert_eq!(pace_brief(&[(now - 60, 95)], now, 90), "");
        assert_eq!(
            tray_tip("Jusage · 이번 주 80% 남음", "46분에 8%"),
            "Jusage · 이번 주 80% 남음 · 46분에 8%"
        );
        assert_eq!(
            tray_tip("Jusage · 이번 주 80% 남음", ""),
            "Jusage · 이번 주 80% 남음"
        );
        let faster = [(now - 1200, 90u32), (now - 600, 88), (now, 80)];
        assert_eq!(
            pace_shift(&faster, now, 80),
            Some("아까보다 빨라졌어요".into())
        );
        let slower = [(now - 1200, 90u32), (now - 600, 82), (now, 80)];
        assert_eq!(
            pace_shift(&slower, now, 80),
            Some("아까보다 느려졌어요".into())
        );
        let even = [(now - 1200, 90u32), (now - 600, 85), (now, 80)];
        assert_eq!(pace_shift(&even, now, 80), None);
    }

    #[test]
    fn refill_guess_names_what_is_left_when_the_refill_wins() {
        let now = 1_800_000_000i64;
        let falling = [(now - 3600, 100u32)];
        assert_eq!(
            refill_guess(&falling, now, 90, Some(now + 7200)),
            Some("충전 즈음 70% 남을 듯".into())
        );
        assert_eq!(
            refill_guess(&[(now - 3600, 95)], now, 90, Some(now + 2 * 86_400)),
            None
        );
        assert_eq!(
            refill_guess(&[(now - 3600, 90)], now, 90, Some(now + 3600)),
            Some("충전 즈음 90% 남을 듯".into())
        );
        assert_eq!(
            refill_guess(&[(now - 60, 95)], now, 90, Some(now + 3600)),
            None
        );
        assert_eq!(refill_guess(&falling, now, 90, None), None);
        assert_eq!(refill_guess(&falling, now, 90, Some(now - 5)), None);
    }

    #[test]
    fn window_balance_compares_the_clock_with_what_was_used() {
        assert_eq!(
            window_balance(100, 43, 84),
            Some("시간은 57% 지났고, 16%를 썼어요".into())
        );
        assert_eq!(
            window_balance(100, 100, 100),
            Some("이 창은 방금 채워졌어요".into())
        );
        assert_eq!(
            window_balance(100, 50, 100),
            Some("시간은 50% 지났고, 아직 안 썼어요".into())
        );
        assert_eq!(
            window_balance(100, 0, 84),
            Some("채워질 때가 됐고, 16%를 썼어요".into())
        );
        assert_eq!(
            window_balance(100, -5, 100),
            Some("채워질 때가 됐고, 아직 안 썼어요".into())
        );
        assert_eq!(window_balance(0, 10, 84), None);
        assert_eq!(window_balance(100, 10, 101), None);
    }

    #[test]
    fn recovery_and_daily_average_wait_for_real_history() {
        assert_eq!(lifted_since_low(70, 84), Some("14% 올라왔어요".into()));
        assert_eq!(lifted_since_low(84, 84), None);
        assert_eq!(lifted_since_low(90, 84), None);
        assert_eq!(
            average_day_value(&recent_days("", "session", 2026, 10, 28)),
            None
        );
        let mut text = day_record("", "session", "2026-10-26", 80, 70, false).unwrap();
        text = day_record(&text, "session", "2026-10-28", 60, 55, false).unwrap();
        assert_eq!(
            average_day_value(&recent_days(&text, "session", 2026, 10, 28)),
            None
        );
        text = day_record(&text, "session", "2026-10-27", 70, 60, false).unwrap();
        assert_eq!(
            average_day_value(&recent_days(&text, "session", 2026, 10, 28)),
            Some("10% 줄었어요".into())
        );
        let filled = day_record("", "session", "2026-10-26", 40, 50, false).unwrap();
        let filled = day_record(&filled, "session", "2026-10-27", 50, 60, false).unwrap();
        assert_eq!(
            average_day_value(&recent_days(&filled, "session", 2026, 10, 28)),
            Some("10% 채워졌어요".into())
        );
        assert_eq!(
            week_story(&recent_days("", "session", 2026, 10, 28)),
            "기록이 쌓이면 최근 7일을 보여드려요"
        );
    }

    #[test]
    fn recent_days_keep_first_and_last_for_one_week() {
        assert_eq!(weekday_name(1970, 1, 1), "목");
        assert_eq!(weekday_name(2026, 10, 28), "수");
        let text = day_record("", "session", "2026-10-28", 90, 86, false).unwrap();
        let text = day_record(&text, "session", "2026-10-22", 70, 64, false).unwrap();
        assert!(day_record(&text, "session", "2026-10-28", 90, 86, false).is_none());
        let week = recent_days(&text, "session", 2026, 10, 28);
        assert_eq!(
            week.map(|slot| slot.name),
            ["목", "금", "토", "일", "월", "화", "수"]
        );
        assert!(week[6].today);
        assert_eq!(week[6].open, Some(90));
        assert_eq!(week[6].close, Some(86));
        assert_eq!(week[6].low, Some(86));
        assert_eq!(week[0].open, Some(70));
        assert_eq!(week[0].close, Some(64));
        assert_eq!(week[0].low, Some(64));
        assert_eq!(week[1].open, None);
        let other = recent_days(&text, "week", 2026, 10, 28);
        assert!(other.iter().all(|slot| slot.open.is_none()));
        let old = day_record(&text, "session", "2026-10-28", 90, 80, false).unwrap();
        assert!(!old.contains("2026-10-01"));
        let stale = format!("{text}session 2026-10-01 10 9\n");
        let trimmed = day_record(&stale, "session", "2026-10-28", 90, 86, false).unwrap();
        assert!(!trimmed.contains("2026-10-01"));
        let dipped = day_record(&text, "session", "2026-10-28", 90, 70, false).unwrap();
        let back = day_record(&dipped, "session", "2026-10-28", 90, 88, false).unwrap();
        let week = recent_days(&back, "session", 2026, 10, 28);
        assert_eq!(week[6].close, Some(88));
        assert_eq!(week[6].low, Some(70));
        assert_eq!(
            week_story(&week),
            "이 기록에서 18% 채워졌어요. 목요일이 가장 많이 줄었어요"
        );
        assert_eq!(yesterday_close(&week), None);
        assert_eq!(earlier_low(&week), Some(("목", 64)));
        let with_yesterday = day_record(&back, "session", "2026-10-27", 80, 75, false).unwrap();
        let week = recent_days(&with_yesterday, "session", 2026, 10, 28);
        assert_eq!(yesterday_close(&week), Some(75));
    }

    #[test]
    fn a_reset_starts_a_new_daily_low_and_preserves_previous_days() {
        let previous_cycle = "session 0 98 46 46\n";
        assert!(cycle_restarted(previous_cycle, "session", 98));
        assert_eq!(
            cycle_update(previous_cycle, "session", 98).as_deref(),
            Some("session 1 98 98 98\n")
        );
        assert!(!cycle_restarted(previous_cycle, "session", 97));
        assert!(!cycle_restarted(previous_cycle, "other", 98));

        let days = "session 2026-10-27 98 46 46\nsession 2026-10-28 98 46 46\n";
        let reset = day_record(days, "session", "2026-10-28", 98, 98, true).unwrap();
        let week = recent_days(&reset, "session", 2026, 10, 28);
        assert_eq!(week[5].low, Some(46));
        assert_eq!(week[6].open, Some(98));
        assert_eq!(week[6].close, Some(98));
        assert_eq!(week[6].low, Some(98));

        let after_use = day_record(&reset, "session", "2026-10-28", 98, 97, false).unwrap();
        assert_eq!(
            recent_days(&after_use, "session", 2026, 10, 28)[6].low,
            Some(97)
        );
    }

    #[test]
    fn home_copy_is_skipped_when_already_there() {
        let home = home_exe(std::path::Path::new(r"C:\Users\user\AppData\Local"));
        assert!(home.ends_with("jusage.exe"));
        assert!(already_home(
            &home,
            std::path::Path::new(r"C:\Users\user\AppData\Local\Jusage\jusage.exe"),
        ));
        assert!(already_home(
            std::path::Path::new(r"\\?\C:\Users\user\AppData\Local\Jusage\jusage.exe"),
            &home,
        ));
        assert!(!already_home(
            std::path::Path::new(r"C:\Users\user\Downloads\jusage.exe"),
            &home,
        ));
    }

    #[test]
    fn account_moves_follow_the_account_not_one_computer() {
        let (day, recent) =
            account_moves(40, 21, Some(40), Some("3시간 전보다 19% 줄었어요".into()));
        assert_eq!(day, "오늘 이 계정에서 19% 줄었어요");
        assert_eq!(recent, "");
        let (day, recent) =
            account_moves(40, 21, Some(30), Some("2시간 전보다 9% 줄었어요".into()));
        assert_eq!(day, "오늘 이 계정에서 19% 줄었어요");
        assert_eq!(recent, "2시간 전보다 9% 줄었어요");
        let (day, recent) = account_moves(21, 21, Some(40), Some("어제보다 19% 줄었어요".into()));
        assert_eq!(day, "");
        assert_eq!(recent, "어제보다 19% 줄었어요");
        let (day, recent) = account_moves(21, 21, None, None);
        assert_eq!(day, "오늘 이 계정은 그대로예요");
        assert_eq!(recent, "");
        let (day, _) = account_moves(18, 30, None, None);
        assert_eq!(day, "오늘 이 계정에서 12% 채워졌어요");
    }

    #[test]
    fn change_line_says_how_the_number_moved() {
        assert_eq!(
            change_line(3 * 3600, 58, 48),
            Some("3시간 전보다 10% 줄었어요".into())
        );
        assert_eq!(
            change_line(30, 40, 55),
            Some("방금보다 15% 채워졌어요".into())
        );
        assert_eq!(change_line(90, 40, 40), None);
        assert_eq!(
            change_line(86_400 + 10, 10, 30),
            Some("어제보다 20% 채워졌어요".into())
        );
    }

    #[test]
    fn later_failure_keeps_the_number_until_login_is_gone() {
        assert_eq!(after_fetch(true, FetchKind::Ok), AfterFetch::Ready);
        assert_eq!(after_fetch(false, FetchKind::Ok), AfterFetch::Ready);
        assert_eq!(after_fetch(true, FetchKind::NoLogin), AfterFetch::NeedLogin);
        assert_eq!(
            after_fetch(false, FetchKind::NoLogin),
            AfterFetch::NeedLogin
        );
        assert_eq!(after_fetch(true, FetchKind::Failed), AfterFetch::KeepLast);
        assert_eq!(after_fetch(false, FetchKind::Failed), AfterFetch::FirstFail);
        assert!(manual_refresh_keeps_number(true));
        assert!(!manual_refresh_keeps_number(false));
        assert_eq!(
            KEEP_LAST_NOTE,
            "방금 확인은 못 했어요. 잠시 뒤 다시 확인해 주세요."
        );
        assert_eq!(CHECKING_NOTE, "확인 중");
    }

    #[test]
    fn cycle_starts_from_the_first_sample_and_resets_when_it_rises() {
        let first = cycle_update("", "session", 90).unwrap();
        assert_eq!(first, "session 0 90 90 90\n");
        assert_eq!(
            cycle_line(&first, "session", 90),
            "기록을 시작한 뒤 그대로예요"
        );
        let dropped = cycle_update(&first, "session", 80).unwrap();
        assert_eq!(dropped, "session 0 90 80 80\n");
        assert_eq!(
            cycle_line(&dropped, "session", 80),
            "기록을 시작한 뒤 10% 썼어요"
        );
        let dipped = cycle_update(&dropped, "session", 70).unwrap();
        let bounced = cycle_update(&dipped, "session", 75).unwrap();
        assert_eq!(bounced, "session 0 90 70 75\n");
        assert_eq!(
            cycle_line(&bounced, "session", 75),
            "기록을 시작한 뒤 15% 썼어요. 한때는 70%까지 내려갔어요"
        );
        let rose = cycle_update(&bounced, "session", 100).unwrap();
        assert_eq!(rose, "session 1 100 100 100\n");
        assert_eq!(cycle_line(&rose, "session", 100), "채워진 뒤 그대로예요");
        let used = cycle_update(&rose, "session", 88).unwrap();
        assert_eq!(used, "session 1 100 88 88\n");
        assert_eq!(cycle_line(&used, "session", 88), "채워진 뒤 12% 썼어요");
        let eased = cycle_update(&used, "session", 90).unwrap();
        assert_eq!(eased, "session 1 100 88 90\n");
        assert_eq!(
            cycle_line(&eased, "session", 90),
            "채워진 뒤 10% 썼어요. 한때는 88%까지 내려갔어요"
        );
        assert!(cycle_update(&used, "session", 88).is_none());
        assert!(cycle_update(&used, "other", 80).is_none());
        assert!(cycle_update(&used, "session", 101).is_none());
        assert_eq!(cycle_line(&used, "week", 88), "");
        let bump = cycle_update(&dropped, "session", 81).unwrap();
        assert_eq!(bump, "session 0 90 80 81\n");
        assert_eq!(
            cycle_line(&bump, "session", 81),
            "기록을 시작한 뒤 9% 썼어요. 한때는 80%까지 내려갔어요"
        );
    }

    #[test]
    fn second_window_can_carry_its_own_short_pace() {
        assert_eq!(tile_brief(true, "3분에 5%"), "3분에 5%");
        assert_eq!(tile_brief(false, "3분에 5%"), "");
        assert_eq!(tile_brief(true, ""), "");
    }
}
