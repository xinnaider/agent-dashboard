use std::collections::HashMap;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};

use crate::session::{self, Session, SessionStatus};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ViewMode {
    Table,
    View,
    Detail,
}

pub struct App {
    pub sessions: Vec<Session>,
    pub selected: usize,
    pub should_quit: bool,
    pub view_mode: ViewMode,
    pub tick: u64,
    pub view_page: usize,
    pub view_zoomed_room: Option<String>,
    pub view_zoom_index: Option<usize>,
    pub view_selected_agent: usize,
    pub detail_selected: usize,
    pub detail_scroll: usize,
    pub detail_auto_scroll: bool,
    pub detail_expanded: Option<usize>,
    pub input_mode: bool,
    pub input_buffer: String,
    pub last_send: Option<Instant>,
    pub last_send_label: Option<String>,
    prev_sessions: HashMap<String, Session>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        App {
            sessions: Vec::new(),
            selected: 0,
            should_quit: false,
            view_mode: ViewMode::Table,
            tick: 0,
            view_page: 0,
            view_zoomed_room: None,
            view_zoom_index: None,
            view_selected_agent: 0,
            detail_selected: 0,
            detail_scroll: 0,
            detail_auto_scroll: true,
            detail_expanded: None,
            input_mode: false,
            input_buffer: String::new(),
            last_send: None,
            last_send_label: None,
            prev_sessions: HashMap::new(),
        }
    }

    pub fn refresh(&mut self) {
        let sessions = session::discover_sessions(&self.prev_sessions);

        // Ring bell whenever any session is in Input state (repeats each refresh)
        let any_input = sessions.iter().any(|s| s.status == SessionStatus::Input);
        if any_input {
            eprint!("\x07");
        }

        self.prev_sessions = sessions
            .iter()
            .map(|s| (s.session_id.clone(), s.clone()))
            .collect();

        self.sessions = sessions;

        if self.selected >= self.sessions.len() && !self.sessions.is_empty() {
            self.selected = self.sessions.len() - 1;
        }
        if self.detail_selected >= self.sessions.len() && !self.sessions.is_empty() {
            self.detail_selected = self.sessions.len() - 1;
        }
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.view_mode {
            ViewMode::Table => self.handle_key_table(key),
            ViewMode::View => self.handle_key_view(key),
            ViewMode::Detail => self.handle_key_detail(key),
        }
    }

    fn handle_key_table(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('v') => self.view_mode = ViewMode::View,
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.sessions.is_empty() {
                    self.selected = (self.selected + 1).min(self.sessions.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Char('r') => {
                self.refresh();
            }
            KeyCode::Char('d') => self.view_mode = ViewMode::Detail,
            _ => {}
        }
    }

    fn handle_key_view(&mut self, key: KeyEvent) {
        if self.view_zoomed_room.is_some() {
            match key.code {
                KeyCode::Char('l') | KeyCode::Right => {
                    self.view_selected_agent = self.view_selected_agent.saturating_add(1);
                    return;
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    self.view_selected_agent = self.view_selected_agent.saturating_sub(1);
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => {
                if self.view_zoomed_room.is_some() {
                    self.view_zoomed_room = None;
                    self.view_selected_agent = 0;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('v') => {
                self.view_zoomed_room = None;
                self.view_selected_agent = 0;
                self.view_zoom_index = None;
                self.view_mode = ViewMode::Table;
            }
            KeyCode::Char('d') => {
                self.view_zoomed_room = None;
                self.view_selected_agent = 0;
                self.view_zoom_index = None;
                self.view_mode = ViewMode::Detail;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.view_page = self.view_page.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.view_page = self.view_page.saturating_sub(1);
            }
            KeyCode::Char(c @ '1'..='9') => {
                let idx = (c as usize) - ('1' as usize);
                self.view_zoom_index = Some(idx);
                self.view_selected_agent = 0;
            }
            _ => {}
        }
    }

    fn handle_key_detail(&mut self, key: KeyEvent) {
        // Input mode: typing a prompt to send to the agent
        if self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input_mode = false;
                    self.input_buffer.clear();
                }
                KeyCode::Enter => {
                    if !self.input_buffer.is_empty() {
                        if let Some(idx) = self.detail_expanded {
                            if let Some(s) = self.sessions.get(idx) {
                                if let Some(pid) = s.pid {
                                    let text = self.input_buffer.clone();
                                    session::send_keys_to_pid(pid, &text);
                                }
                            }
                        }
                    }
                    self.input_mode = false;
                    self.input_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        if self.detail_expanded.is_some() {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                    self.detail_auto_scroll = false;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    self.detail_auto_scroll = false;
                }
                KeyCode::Char('g') => {
                    // Jump to bottom and re-enable auto-scroll
                    self.detail_auto_scroll = true;
                }
                KeyCode::Esc => {
                    self.detail_expanded = None;
                    self.detail_scroll = 0;
                    self.detail_auto_scroll = true;
                }
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('y') | KeyCode::Char('n') => {
                    if self.is_send_debounced() { return; }
                    if let Some(idx) = self.detail_expanded {
                        if let Some(s) = self.sessions.get(idx) {
                            if let Some(pid) = s.pid {
                                let (ch, label) = if key.code == KeyCode::Char('y') {
                                    ("1", "accepted")
                                } else {
                                    ("3", "rejected")
                                };
                                session::send_keys_to_pid(pid, ch);
                                self.mark_sent(label);
                            }
                        }
                    }
                }
                KeyCode::Char('i') => {
                    self.input_mode = true;
                    self.input_buffer.clear();
                }
                KeyCode::Char('d') | KeyCode::Char('t') => {
                    self.detail_expanded = None;
                    self.detail_scroll = 0;
                    self.view_mode = ViewMode::Table;
                }
                KeyCode::Char('v') => {
                    self.detail_expanded = None;
                    self.detail_scroll = 0;
                    self.view_mode = ViewMode::View;
                }
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.sessions.is_empty() {
                    self.detail_selected =
                        (self.detail_selected + 1).min(self.sessions.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_selected = self.detail_selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if !self.sessions.is_empty() {
                    self.detail_expanded =
                        Some(self.detail_selected.min(self.sessions.len() - 1));
                    self.detail_auto_scroll = true;
                }
            }
            KeyCode::Char('y') | KeyCode::Char('n') => {
                if self.is_send_debounced() { return; }
                if let Some(s) = self.sessions.get(self.detail_selected) {
                    if let Some(pid) = s.pid {
                        let (ch, label) = if key.code == KeyCode::Char('y') {
                            ("1", "accepted")
                        } else {
                            ("3", "rejected")
                        };
                        session::send_keys_to_pid(pid, ch);
                        self.mark_sent(label);
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('t') => {
                self.detail_expanded = None;
                self.detail_scroll = 0;
                self.view_mode = ViewMode::Table;
            }
            KeyCode::Char('v') => {
                self.detail_expanded = None;
                self.detail_scroll = 0;
                self.view_mode = ViewMode::View;
            }
            _ => {}
        }
    }

    fn is_send_debounced(&self) -> bool {
        self.last_send
            .map_or(false, |t| t.elapsed().as_millis() < 1500)
    }

    fn mark_sent(&mut self, label: &str) {
        self.last_send = Some(Instant::now());
        self.last_send_label = Some(label.to_string());
    }

    pub fn to_json(&self) -> String {
        let sessions: Vec<serde_json::Value> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                serde_json::json!({
                    "index": i + 1,
                    "session_id": s.session_id,
                    "project_name": s.project_name,
                    "branch": s.branch,
                    "cwd": s.cwd,
                    "room_id": s.room_id(),
                    "relative_dir": s.relative_dir,
                    "model": s.model,
                    "model_display": s.model_display(),
                    "total_input_tokens": s.total_input_tokens,
                    "total_output_tokens": s.total_output_tokens,
                    "context_display": s.token_display(),
                    "token_ratio": s.token_ratio(),
                    "status": s.status.label(),
                    "pid": s.pid,
                    "last_activity": s.last_activity,
                    "started_at": s.started_at,
                })
            })
            .collect();

        serde_json::to_string_pretty(&serde_json::json!({
            "sessions": sessions,
        }))
        .unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn handle_key_detail_j_increments_selected_when_sessions_present() {
        let mut app = App::new();
        app.view_mode = ViewMode::Detail;
        use crate::session::{Session, SessionStatus};
        for _ in 0..3 {
            app.sessions.push(Session {
                session_id: "x".to_string(),
                project_name: "p".to_string(),
                branch: None,
                cwd: "/".to_string(),
                relative_dir: None,
                model: None,
                effort: None,
                total_input_tokens: 0,
                total_output_tokens: 0,
                status: SessionStatus::Idle,
                pid: None,
                last_activity: None,
                last_action: None,
                activity_log: Vec::new(),
                started_at: 0,
                last_file_size: 0,
            });
        }
        app.handle_key(press(KeyCode::Char('j')));
        assert_eq!(app.detail_selected, 1);
        app.handle_key(press(KeyCode::Char('j')));
        assert_eq!(app.detail_selected, 2);
        // Can't go past last
        app.handle_key(press(KeyCode::Char('j')));
        assert_eq!(app.detail_selected, 2);
    }

    #[test]
    fn handle_key_detail_k_decrements_selected() {
        let mut app = App::new();
        app.view_mode = ViewMode::Detail;
        app.detail_selected = 2;
        app.handle_key(press(KeyCode::Char('k')));
        assert_eq!(app.detail_selected, 1);
        app.handle_key(press(KeyCode::Char('k')));
        assert_eq!(app.detail_selected, 0);
        // Can't go below 0
        app.handle_key(press(KeyCode::Char('k')));
        assert_eq!(app.detail_selected, 0);
    }

    #[test]
    fn handle_key_detail_expanded_esc_collapses() {
        let mut app = App::new();
        app.view_mode = ViewMode::Detail;
        app.detail_expanded = Some(0);
        app.detail_scroll = 5;
        app.handle_key(press(KeyCode::Esc));
        assert!(app.detail_expanded.is_none());
        assert_eq!(app.detail_scroll, 0);
    }

    #[test]
    fn handle_key_detail_enter_expands_selected() {
        let mut app = App::new();
        app.view_mode = ViewMode::Detail;
        use crate::session::{Session, SessionStatus};
        app.sessions.push(Session {
            session_id: "x".to_string(),
            project_name: "p".to_string(),
            branch: None,
            cwd: "/".to_string(),
            relative_dir: None,
            model: None,
            effort: None,
            total_input_tokens: 0,
            total_output_tokens: 0,
            status: SessionStatus::Idle,
            pid: None,
            last_activity: None,
            last_action: None,
            activity_log: Vec::new(),
            started_at: 0,
            last_file_size: 0,
        });
        app.detail_selected = 0;
        app.handle_key(press(KeyCode::Enter));
        assert_eq!(app.detail_expanded, Some(0));
    }

    #[test]
    fn handle_key_detail_d_returns_to_table() {
        let mut app = App::new();
        app.view_mode = ViewMode::Detail;
        app.detail_expanded = Some(0);
        app.detail_scroll = 3;
        app.handle_key(press(KeyCode::Char('d')));
        assert_eq!(app.view_mode, ViewMode::Table);
        assert!(app.detail_expanded.is_none());
        assert_eq!(app.detail_scroll, 0);
    }
}
