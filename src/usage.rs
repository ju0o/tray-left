use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub plan: String,
    pub session_used: f64,
    pub weekly_used: Option<f64>,
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
    Some(Snapshot {
        plan: json
            .get("plan_type")
            .and_then(|value| value.as_str())
            .map(plan_label)
            .unwrap_or_default(),
        session_used: session?.clamp(0.0, 100.0),
        weekly_used: weekly.map(|value| value.clamp(0.0, 100.0)),
    })
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
}
