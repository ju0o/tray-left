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

pub fn mood_rgb(remaining: Option<u32>) -> [u8; 3] {
    match remaining {
        Some(value) if value > 20 => [15, 118, 110],
        Some(_) => [180, 83, 9],
        None => [140, 140, 140],
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Civil {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
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
        _ => format!("{}월 {}일 {}", at.month, at.day, clock_korean(at.hour, at.minute)),
    };
    format!("{when}쯤 다시 채워져요")
}

fn civil_minutes(time: Civil) -> i64 {
    julian_day(time.year, time.month, time.day) * 24 * 60 + time.hour as i64 * 60 + time.minute as i64
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
        weekly_window_seconds: weekly_window.and_then(|value| unix_field(value, "limit_window_seconds")),
    })
}

fn unix_field(window: &serde_json::Value, name: &str) -> Option<i64> {
    let value = window.get(name)?;
    let number = value.as_f64().or_else(|| value.as_i64().map(|item| item as f64))?;
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
        .set("User-Agent", "tray-left/0.1");
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
        .filter_map(|name| response.header(&name).map(|value| (name, value.to_string())))
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
    let root = if sessions.is_dir() { sessions } else { home.to_path_buf() };
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
        let now = Civil { year: 2026, month: 10, day: 27, hour: 15, minute: 0 };
        assert_eq!(
            refill_phrase(now, Civil { hour: 21, minute: 0, ..now }),
            "오늘 오후 9시쯤 다시 채워져요"
        );
        assert_eq!(
            refill_phrase(now, Civil { day: 28, hour: 0, minute: 10, ..now }),
            "내일 오전 12시 10분쯤 다시 채워져요"
        );
        assert_eq!(
            refill_phrase(now, Civil { day: 30, hour: 16, minute: 0, ..now }),
            "10월 30일 오후 4시쯤 다시 채워져요"
        );
        assert_eq!(
            refill_phrase(now, Civil { hour: 14, minute: 0, ..now }),
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
        assert_eq!(mood_rgb(None), [140, 140, 140]);
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
        assert_eq!(window_name(snapshot.session_window_seconds, false), "이번 주");
    }

    #[test]
    fn missing_window_length_stays_generic() {
        assert_eq!(window_name(None, false), "이번 창");
        assert_eq!(window_name(None, true), "이번 주");
        assert_eq!(window_name(Some(3_600), false), "이번 창");
    }
}
