use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::model;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    New,
    Working,
    Idle,
    Input,
}

impl SessionStatus {
    pub fn label(&self) -> &str {
        match self {
            SessionStatus::New => "New",
            SessionStatus::Working => "Working",
            SessionStatus::Idle => "Idle",
            SessionStatus::Input => "Input",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: String,
    pub project_name: String,
    pub branch: Option<String>,
    pub cwd: String,
    pub relative_dir: Option<String>,
    pub model: Option<String>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub status: SessionStatus,
    pub pid: Option<i32>,
    pub effort: Option<String>,
    pub last_activity: Option<String>,
    pub started_at: u64,
    pub jsonl_path: PathBuf,
    pub last_file_size: u64,
}

impl Session {
    pub fn room_id(&self) -> String {
        match &self.relative_dir {
            Some(dir) => format!("{} \u{203A} {}", self.project_name, dir),
            None => self.project_name.clone(),
        }
    }

    pub fn token_display(&self) -> String {
        let used = self.total_input_tokens + self.total_output_tokens;
        let window = self
            .model
            .as_deref()
            .map(model::context_window)
            .unwrap_or(200_000);
        format!("{}k / {}", used / 1000, format_window(window))
    }

    pub fn token_ratio(&self) -> f64 {
        let used = self.total_input_tokens + self.total_output_tokens;
        let window = self
            .model
            .as_deref()
            .map(model::context_window)
            .unwrap_or(200_000);
        if window == 0 {
            return 0.0;
        }
        used as f64 / window as f64
    }

    pub fn model_display(&self) -> String {
        match &self.model {
            Some(m) => model::format_with_effort(m, self.effort.as_deref().unwrap_or("")),
            None => "\u{2014}".to_string(),
        }
    }
}

pub fn format_window(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{}M", tokens / 1_000_000)
    } else {
        format!("{}k", tokens / 1000)
    }
}

// ── Windows Process Detection ────────────────────────────────────────

struct LiveSessionInfo {
    pid: i32,
    started_at: u64,
}

fn build_live_session_map() -> HashMap<String, LiveSessionInfo> {
    let session_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("sessions"),
        None => return HashMap::new(),
    };

    let mut map = HashMap::new();

    let entries = match fs::read_dir(&session_dir) {
        Ok(e) => e,
        Err(_) => return map,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().map(|e| e == "json").unwrap_or(false) {
            continue;
        }

        let pid_str = match path.file_stem().map(|s| s.to_string_lossy().to_string()) {
            Some(s) => s,
            None => continue,
        };

        let pid: i32 = match pid_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        if !is_claude_cli_process(pid) {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        #[derive(Deserialize)]
        struct SessionFile {
            #[serde(rename = "sessionId")]
            session_id: String,
            #[serde(rename = "startedAt", default)]
            started_at: u64,
        }

        if let Ok(info) = serde_json::from_str::<SessionFile>(&content) {
            map.insert(
                info.session_id,
                LiveSessionInfo {
                    pid,
                    started_at: info.started_at,
                },
            );
        }
    }

    map
}

fn is_claude_cli_process(pid: i32) -> bool {
    let output = ProcessCommand::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Get-Process -Id {} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Path",
                pid
            ),
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let path = String::from_utf8_lossy(&o.stdout).trim().to_lowercase();
            path.contains(".local") && path.contains("claude")
        }
        _ => false,
    }
}

// ── JSONL Parsing ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct JsonlEntry {
    #[serde(default)]
    message: Option<MessageEntry>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Deserialize)]
struct MessageEntry {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<UsageEntry>,
}

#[derive(Deserialize)]
struct UsageEntry {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

#[derive(Debug)]
struct ParsedInfo {
    input_tokens: u64,
    output_tokens: u64,
    model: Option<String>,
    effort: Option<String>,
    cwd: Option<String>,
    last_activity: Option<String>,
    file_size: u64,
}

fn parse_jsonl(
    path: &Path,
    prev_file_size: u64,
    prev_input: u64,
    prev_output: u64,
    prev_model: Option<String>,
    prev_effort: Option<String>,
    prev_activity: Option<String>,
) -> ParsedInfo {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => {
            return ParsedInfo {
                input_tokens: prev_input,
                output_tokens: prev_output,
                model: prev_model,
                effort: prev_effort,
                cwd: None,
                last_activity: prev_activity,
                file_size: 0,
            }
        }
    };

    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);

    if file_size == prev_file_size && prev_file_size > 0 {
        return ParsedInfo {
            input_tokens: prev_input,
            output_tokens: prev_output,
            model: prev_model,
            effort: prev_effort,
            cwd: None,
            last_activity: prev_activity,
            file_size,
        };
    }

    let mut reader = BufReader::new(file);
    let mut total_input = prev_input;
    let mut total_output = prev_output;
    let mut model_val = prev_model;
    let mut effort = prev_effort;
    let mut last_activity = prev_activity;
    let mut cwd = None;

    if prev_file_size > 0 {
        let _ = reader.seek(SeekFrom::Start(prev_file_size));
    } else {
        total_input = 0;
        total_output = 0;
        model_val = None;
        effort = None;
        last_activity = None;
    }

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.contains("\"type\"") {
            continue;
        }

        if trimmed.contains("\"type\":\"assistant\"") {
            if trimmed.contains("\"<synthetic>\"") {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<JsonlEntry>(trimmed) {
                if let Some(ts) = entry.timestamp {
                    last_activity = Some(ts);
                }
                if entry.cwd.is_some() {
                    cwd = entry.cwd;
                }
                if let Some(msg) = entry.message {
                    if let Some(m) = msg.model {
                        model_val = Some(m);
                    }
                    if let Some(usage) = msg.usage {
                        total_input = usage.input_tokens
                            + usage.cache_creation_input_tokens
                            + usage.cache_read_input_tokens;
                        total_output = usage.output_tokens;
                    }
                }
            }
        } else if trimmed.contains("\"type\":\"user\"") || trimmed.contains("\"type\":\"system\"") {
            if let Ok(entry) = serde_json::from_str::<JsonlEntry>(trimmed) {
                if let Some(ts) = entry.timestamp {
                    last_activity = Some(ts);
                }
                if entry.cwd.is_some() {
                    cwd = entry.cwd;
                }
            }
            if trimmed.contains("<local-command-stdout>Set model to")
                && !trimmed.contains("toolUseResult")
                && !trimmed.contains("tool_result")
            {
                if let Some(stdout_pos) = trimmed.find("<local-command-stdout>Set model to") {
                    let tag_end = stdout_pos + "<local-command-stdout>Set model to".len();
                    let raw_remainder = &trimmed[tag_end..];
                    let raw_remainder = raw_remainder
                        .find("</local-command-stdout>")
                        .map_or(raw_remainder, |end| &raw_remainder[..end]);
                    let remainder = raw_remainder.trim();

                    let (model_part, new_effort) = if let Some(wp) = remainder.find("with ") {
                        let after_with = &remainder[wp + 5..];
                        let eff = after_with
                            .find(" effort")
                            .map(|end| after_with[..end].trim().to_string())
                            .filter(|s| !s.is_empty());
                        (&remainder[..wp], eff)
                    } else {
                        (&remainder[..], None)
                    };
                    if let Some(e) = new_effort {
                        effort = Some(e);
                    }

                    let model_name = model_part
                        .trim()
                        .trim_end_matches("(default)")
                        .trim()
                        .trim_end_matches("(1M context)")
                        .trim()
                        .trim_end_matches("(200k context)")
                        .trim();
                    if let Some(id) = model::id_from_display_name(model_name) {
                        model_val = Some(id.to_string());
                    }
                }
            }
        }
    }

    ParsedInfo {
        input_tokens: total_input,
        output_tokens: total_output,
        model: model_val,
        effort,
        cwd,
        last_activity,
        file_size,
    }
}

// ── Git Info ─────────────────────────────────────────────────────────

struct GitInfo {
    repo_name: String,
    relative_dir: Option<String>,
    branch: Option<String>,
    fetched_at: Instant,
}

static GIT_CACHE: Mutex<Option<HashMap<String, GitInfo>>> = Mutex::new(None);
const GIT_CACHE_TTL: Duration = Duration::from_secs(30);

fn git_project_info(cwd: &str) -> (String, Option<String>, Option<String>) {
    {
        let cache = GIT_CACHE.lock().unwrap();
        if let Some(info) = cache.as_ref().and_then(|c| c.get(cwd)) {
            if info.fetched_at.elapsed() < GIT_CACHE_TTL {
                return (info.repo_name.clone(), info.relative_dir.clone(), info.branch.clone());
            }
        }
    }

    let repo_name = fetch_git_repo_name(cwd);
    let relative_dir = fetch_relative_dir(cwd);
    let branch = fetch_git_branch(cwd);

    let mut cache = GIT_CACHE.lock().unwrap();
    if cache.is_none() {
        *cache = Some(HashMap::new());
    }
    cache.as_mut().unwrap().insert(
        cwd.to_string(),
        GitInfo {
            repo_name: repo_name.clone(),
            relative_dir: relative_dir.clone(),
            branch: branch.clone(),
            fetched_at: Instant::now(),
        },
    );
    (repo_name, relative_dir, branch)
}

fn fetch_git_repo_name(cwd: &str) -> String {
    let output = ProcessCommand::new("git")
        .args(["-C", cwd, "rev-parse", "--git-common-dir"])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let common = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let common_path = if Path::new(&common).is_absolute() {
                PathBuf::from(&common)
            } else {
                PathBuf::from(cwd).join(&common)
            };
            let resolved = common_path.canonicalize().unwrap_or(common_path);
            let repo_root = if resolved.file_name().map(|n| n == ".git").unwrap_or(false) {
                resolved.parent().unwrap_or(&resolved)
            } else {
                &resolved
            };
            repo_root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| cwd.to_string())
        }
        _ => Path::new(cwd)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| cwd.to_string()),
    }
}

fn fetch_git_branch(cwd: &str) -> Option<String> {
    let output = ProcessCommand::new("git")
        .args(["-C", cwd, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

fn fetch_relative_dir(cwd: &str) -> Option<String> {
    let output = ProcessCommand::new("git")
        .args(["-C", cwd, "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let toplevel = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let cwd_resolved = Path::new(cwd).canonicalize().unwrap_or_else(|_| PathBuf::from(cwd));
    let top_resolved = Path::new(&toplevel).canonicalize().unwrap_or_else(|_| PathBuf::from(&toplevel));
    let relative = cwd_resolved.strip_prefix(&top_resolved).unwrap_or(Path::new(""));
    if relative.as_os_str().is_empty() || relative == Path::new(".") {
        None
    } else {
        Some(relative.display().to_string())
    }
}

fn decode_project_path(project_dir: &Path) -> String {
    let name = project_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    if name.len() >= 3 && name.as_bytes()[0].is_ascii_alphabetic() && &name[1..3] == "--" {
        let drive = &name[0..1];
        let rest = &name[3..];
        format!("{}:/{}", drive, rest.replace('-', "/"))
    } else if name.starts_with("--") {
        format!("/{}", name[2..].replace('-', "/"))
    } else if name.starts_with('-') {
        name.replacen('-', "/", 1).replace('-', "/")
    } else {
        name
    }
}

// ── Status Detection ─────────────────────────────────────────────────

fn determine_status(path: &Path, input_tokens: u64, output_tokens: u64) -> SessionStatus {
    if input_tokens == 0 && output_tokens == 0 {
        return SessionStatus::New;
    }

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return SessionStatus::Idle,
    };

    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut reader = BufReader::new(file);

    let seek_pos = if file_size > 8192 {
        file_size - 8192
    } else {
        0
    };
    let _ = reader.seek(SeekFrom::Start(seek_pos));

    let mut last_type = String::new();
    let mut last_timestamp = String::new();
    let mut has_tool_use_permission = false;

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.contains("\"type\":\"assistant\"") {
            last_type = "assistant".to_string();
            if trimmed.contains("\"type\":\"tool_use\"") {
                has_tool_use_permission = true;
            } else {
                has_tool_use_permission = false;
            }
        } else if trimmed.contains("\"type\":\"user\"") {
            last_type = "user".to_string();
            has_tool_use_permission = false;
        } else if trimmed.contains("\"type\":\"progress\"") {
            last_type = "progress".to_string();
            has_tool_use_permission = false;
        }

        if let Some(ts_start) = trimmed.find("\"timestamp\":\"") {
            let after = &trimmed[ts_start + 13..];
            if let Some(ts_end) = after.find('"') {
                last_timestamp = after[..ts_end].to_string();
            }
        }
    }

    if last_type == "assistant" && has_tool_use_permission {
        return SessionStatus::Input;
    }

    if !last_timestamp.is_empty() {
        if let Ok(dt) = last_timestamp.parse::<chrono::DateTime<chrono::Utc>>() {
            let elapsed = chrono::Utc::now() - dt;
            if elapsed.num_seconds() < 30 {
                return SessionStatus::Working;
            }
        }
    }

    if last_type == "progress" {
        return SessionStatus::Working;
    }

    SessionStatus::Idle
}

fn truncate_to_minute(ts: &Option<String>) -> Option<String> {
    ts.as_ref().map(|s| s.get(..16).unwrap_or(s).to_string())
}

// ── Main Discovery Function ──────────────────────────────────────────

pub fn discover_sessions(prev_sessions: &HashMap<String, Session>) -> Vec<Session> {
    let claude_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    if !claude_dir.exists() {
        return vec![];
    }

    let live_map = build_live_session_map();
    let mut sessions: Vec<Session> = Vec::new();
    let mut matched_session_ids: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    let entries = match fs::read_dir(&claude_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        let project_dir = entry.path();
        if !project_dir.is_dir() {
            continue;
        }

        let jsonl_files = match fs::read_dir(&project_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for jentry in jsonl_files.flatten() {
            let path = jentry.path();
            if path.is_dir() {
                continue;
            }
            if !path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                continue;
            }

            let session_id = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let live = match live_map.get(&session_id) {
                Some(l) => l,
                None => continue,
            };

            if matched_session_ids.contains(&session_id) {
                continue;
            }

            let prev = prev_sessions.get(&session_id);
            let info = parse_jsonl(
                &path,
                prev.map(|s| s.last_file_size).unwrap_or(0),
                prev.map(|s| s.total_input_tokens).unwrap_or(0),
                prev.map(|s| s.total_output_tokens).unwrap_or(0),
                prev.and_then(|s| s.model.clone()),
                prev.and_then(|s| s.effort.clone()),
                prev.and_then(|s| s.last_activity.clone()),
            );

            let cwd = info
                .cwd
                .or_else(|| prev.map(|s| s.cwd.clone()))
                .unwrap_or_else(|| decode_project_path(&project_dir));
            let (project_name, relative_dir, branch) = git_project_info(&cwd);

            let status = determine_status(&path, info.input_tokens, info.output_tokens);

            matched_session_ids.insert(session_id.clone());

            sessions.push(Session {
                session_id,
                project_name,
                branch,
                cwd,
                relative_dir,
                model: info.model,
                effort: info.effort,
                total_input_tokens: info.input_tokens,
                total_output_tokens: info.output_tokens,
                status,
                pid: Some(live.pid),
                last_activity: info.last_activity,
                started_at: live.started_at,
                jsonl_path: path,
                last_file_size: info.file_size,
            });
        }
    }

    let known_pids: std::collections::HashSet<i32> = sessions
        .iter()
        .filter_map(|s| s.pid)
        .collect();

    for (_session_id_key, live) in &live_map {
        if known_pids.contains(&live.pid) {
            continue;
        }

        sessions.push(Session {
            session_id: format!("pid-{}", live.pid),
            project_name: "new session".to_string(),
            relative_dir: None,
            branch: None,
            cwd: String::new(),
            model: None,
            effort: None,
            total_input_tokens: 0,
            total_output_tokens: 0,
            status: SessionStatus::New,
            pid: Some(live.pid),
            last_activity: None,
            started_at: live.started_at,
            jsonl_path: PathBuf::new(),
            last_file_size: 0,
        });
    }

    sessions.sort_by(|a, b| {
        truncate_to_minute(&b.last_activity)
            .cmp(&truncate_to_minute(&a.last_activity))
            .then(b.started_at.cmp(&a.started_at))
    });
    sessions
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_format_window() {
        assert_eq!(format_window(1_000_000), "1M");
        assert_eq!(format_window(200_000), "200k");
    }

    #[test]
    fn test_session_token_display() {
        let session = Session {
            session_id: String::new(),
            project_name: String::new(),
            branch: None,
            cwd: String::new(),
            relative_dir: None,
            model: Some("claude-opus-4-6".to_string()),
            effort: None,
            total_input_tokens: 45_000,
            total_output_tokens: 5_000,
            status: SessionStatus::Working,
            pid: None,
            last_activity: None,
            started_at: 0,
            jsonl_path: PathBuf::new(),
            last_file_size: 0,
        };
        assert_eq!(session.token_display(), "50k / 1M");
    }

    #[test]
    fn test_session_token_ratio() {
        let session = Session {
            session_id: String::new(),
            project_name: String::new(),
            branch: None,
            cwd: String::new(),
            relative_dir: None,
            model: Some("claude-sonnet-4-6".to_string()),
            effort: None,
            total_input_tokens: 100_000,
            total_output_tokens: 100_000,
            status: SessionStatus::Working,
            pid: None,
            last_activity: None,
            started_at: 0,
            jsonl_path: PathBuf::new(),
            last_file_size: 0,
        };
        assert!((session.token_ratio() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_decode_project_path_windows() {
        let path = PathBuf::from("C--Users-fernandonepen-Documents-myapp");
        assert_eq!(decode_project_path(&path), "C:/Users/fernandonepen/Documents/myapp");
    }

    #[test]
    fn test_decode_project_path_wsl() {
        let path = PathBuf::from("--wsl-localhost-Ubuntu-home-user-project");
        assert_eq!(decode_project_path(&path), "/wsl/localhost/Ubuntu/home/user/project");
    }

    #[test]
    fn test_parse_jsonl_extracts_tokens_and_model() {
        let dir = std::env::temp_dir().join("agent_dashboard_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test-session.jsonl");

        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"type":"assistant","message":{{"model":"claude-opus-4-6","usage":{{"input_tokens":1000,"output_tokens":500,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}},"timestamp":"2026-03-24T10:00:00Z","cwd":"C:\\Users\\test\\project"}}"#).unwrap();

        let info = parse_jsonl(&path, 0, 0, 0, None, None, None);
        assert_eq!(info.input_tokens, 1000);
        assert_eq!(info.output_tokens, 500);
        assert_eq!(info.model, Some("claude-opus-4-6".to_string()));
        assert_eq!(info.last_activity, Some("2026-03-24T10:00:00Z".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_truncate_to_minute() {
        assert_eq!(
            truncate_to_minute(&Some("2026-03-19T21:25:34.098Z".to_string())),
            Some("2026-03-19T21:25".to_string())
        );
        assert_eq!(truncate_to_minute(&None), None);
    }

    #[test]
    fn test_status_label() {
        assert_eq!(SessionStatus::New.label(), "New");
        assert_eq!(SessionStatus::Working.label(), "Working");
        assert_eq!(SessionStatus::Idle.label(), "Idle");
        assert_eq!(SessionStatus::Input.label(), "Input");
    }

    #[test]
    fn test_room_id_with_relative_dir() {
        let mut session = Session {
            session_id: String::new(),
            project_name: "myapp".to_string(),
            branch: None,
            cwd: String::new(),
            relative_dir: Some("tools/cli".to_string()),
            model: None,
            effort: None,
            total_input_tokens: 0,
            total_output_tokens: 0,
            status: SessionStatus::New,
            pid: None,
            last_activity: None,
            started_at: 0,
            jsonl_path: PathBuf::new(),
            last_file_size: 0,
        };
        assert_eq!(session.room_id(), "myapp \u{203a} tools/cli");

        session.relative_dir = None;
        assert_eq!(session.room_id(), "myapp");
    }
}
