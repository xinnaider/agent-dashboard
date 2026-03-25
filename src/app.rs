use std::collections::HashMap;

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
    pub detail_expanded: Option<usize>,
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
            detail_expanded: None,
            prev_sessions: HashMap::new(),
        }
    }

    pub fn refresh(&mut self) {
        let sessions = session::discover_sessions(&self.prev_sessions);

        // Ring bell when any session newly enters Input state
        let newly_input = sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Input)
            .any(|s| {
                self.prev_sessions
                    .get(&s.session_id)
                    .map_or(true, |prev| prev.status != SessionStatus::Input)
            });
        if newly_input {
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
        if self.detail_expanded.is_some() {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                }
                KeyCode::Esc => {
                    self.detail_expanded = None;
                    self.detail_scroll = 0;
                }
                KeyCode::Char('q') => self.should_quit = true,
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
                last_bash_lines: None,
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
            last_bash_lines: None,
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
