use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::session::{Session, SessionStatus};
use crate::ui::format_timestamp;
use crate::view_ui::{action_or_label, status_color, truncate_str};

const BLOCK_H: u16 = 7; // border(1) + 5 inner lines + border(1)

pub fn render(frame: &mut Frame, app: &App) {
    let input_h = if app.input_mode { 1 } else { 0 };
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(input_h),
        Constraint::Length(1),
    ])
    .split(frame.area());

    if let Some(idx) = app.detail_expanded {
        if let Some(session) = app.sessions.get(idx) {
            render_expanded(frame, session, app, chunks[0]);
        }
    } else {
        render_grid(frame, app, chunks[0]);
    }

    if app.input_mode {
        render_input_bar(frame, &app.input_buffer, chunks[1]);
    }

    render_footer(frame, app, chunks[2]);
}

fn render_grid(frame: &mut Frame, app: &App, area: Rect) {
    if app.sessions.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "No active sessions",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(msg, area);
        return;
    }

    let n = app.sessions.len();
    let selected = app.detail_selected.min(n - 1);
    let visible = ((area.height / BLOCK_H) as usize).max(1);

    // Scroll to keep selected visible
    let scroll = (selected + 1).saturating_sub(visible);

    let mut y = area.y;
    for (i, session) in app.sessions.iter().enumerate().skip(scroll) {
        if y + BLOCK_H > area.y + area.height {
            break;
        }
        let block_area = Rect { x: area.x, y, width: area.width, height: BLOCK_H };
        render_block(frame, session, block_area, i == selected, app.tick);
        y += BLOCK_H;
    }
}

fn render_block(frame: &mut Frame, session: &Session, area: Rect, is_selected: bool, tick: u64) {
    let border_color = if session.status == SessionStatus::Input {
        if tick.is_multiple_of(2) { Color::Yellow } else { Color::White }
    } else if is_selected {
        status_color(&session.status)
    } else {
        Color::DarkGray
    };

    let title = build_block_title(session);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title, Style::default().fg(border_color)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 {
        return;
    }

    let w = inner.width as usize;
    let action_color = if session.status == SessionStatus::Input {
        Color::Yellow
    } else {
        status_color(&session.status)
    };
    let activity = session
        .last_activity
        .as_deref()
        .map(format_timestamp)
        .unwrap_or_else(|| "\u{2014}".to_string());

    let mut lines: Vec<Line> = Vec::new();

    // Line 1: metadata
    lines.push(Line::from(Span::styled(
        truncate_str(&build_meta_line(session), w),
        Style::default().fg(Color::Gray),
    )));

    // Line 2: action (left) + activity (right)
    if inner.height > 1 {
        let action = action_or_label(session);
        let action_len = action.chars().count();
        let activity_len = activity.chars().count();
        let combined = action_len + activity_len + 1; // 1 for the leading space
        let padding = if combined < w { " ".repeat(w - combined) } else { String::new() };
        lines.push(Line::from(vec![
            Span::styled(format!(" {action}"), Style::default().fg(action_color)),
            Span::raw(padding),
            Span::styled(activity, Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Line 3: separator
    if inner.height > 2 {
        lines.push(Line::from(Span::styled(
            "\u{2500}".repeat(w),
            Style::default().fg(Color::Rgb(50, 50, 50)),
        )));
    }

    // Lines 4+: activity log
    if inner.height > 3 {
        let available = (inner.height - 3) as usize;
        let start = session.activity_log.len().saturating_sub(available);
        for log_line in session.activity_log.iter().skip(start).take(available) {
            let (text, color) = activity_line_style(log_line, w);
            lines.push(Line::from(Span::styled(text, Style::default().fg(color))));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_expanded(frame: &mut Frame, session: &Session, app: &App, area: Rect) {
    let color = status_color(&session.status);
    let title = build_block_title(session);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Span::styled(title, Style::default().fg(color)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 {
        return;
    }
    let w = inner.width as usize;
    let available = inner.height as usize;

    let mut lines: Vec<Line> = Vec::new();

    if session.activity_log.is_empty() {
        lines.push(Line::from(Span::styled(
            "No activity recorded",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let max_scroll = session.activity_log.len().saturating_sub(available);
        let scroll = if app.detail_auto_scroll {
            max_scroll
        } else {
            app.detail_scroll.min(max_scroll)
        };
        for log_line in session.activity_log.iter().skip(scroll).take(available) {
            let (text, color) = activity_line_style(log_line, w);
            lines.push(Line::from(Span::styled(text, Style::default().fg(color))));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_input_bar(frame: &mut Frame, buffer: &str, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" \u{276f} ", Style::default().fg(Color::Yellow)),
        Span::styled(buffer, Style::default().fg(Color::White)),
        Span::styled("\u{2588}", Style::default().fg(Color::Yellow)), // cursor block
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    // Show "✓ accepted" or "✗ rejected" feedback for 1.5s after sending
    let send_feedback: Option<(&str, Color)> = app.last_send.and_then(|t| {
        if t.elapsed().as_millis() < 1500 {
            app.last_send_label.as_deref().map(|label| {
                if label == "accepted" {
                    ("\u{2713} accepted  ", Color::Green)
                } else {
                    ("\u{2717} rejected  ", Color::Red)
                }
            })
        } else {
            None
        }
    });

    let spans = if app.input_mode {
        vec![
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" send  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ]
    } else if app.detail_expanded.is_some() {
        let expanded_is_input = app
            .detail_expanded
            .and_then(|idx| app.sessions.get(idx))
            .map_or(false, |s| s.status == SessionStatus::Input);
        let mut s = vec![
            Span::styled("j/k", Style::default().fg(Color::Cyan)),
            Span::raw(" scroll  "),
        ];
        if !app.detail_auto_scroll {
            s.extend([
                Span::styled("g", Style::default().fg(Color::Cyan)),
                Span::raw(" bottom  "),
            ]);
        }
        s.extend([
            Span::styled("i", Style::default().fg(Color::Cyan)),
            Span::raw(" prompt  "),
        ]);
        if expanded_is_input {
            s.extend([
                Span::styled("y", Style::default().fg(Color::Yellow)),
                Span::raw(" accept  "),
                Span::styled("n", Style::default().fg(Color::Yellow)),
                Span::raw(" reject  "),
            ]);
        }
        s.extend([
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" back  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]);
        s
    } else {
        let selected_is_input = app
            .sessions
            .get(app.detail_selected)
            .map_or(false, |s| s.status == SessionStatus::Input);
        let mut spans = vec![
            Span::styled("j/k", Style::default().fg(Color::Cyan)),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" expand  "),
        ];
        if selected_is_input {
            spans.extend([
                Span::styled("y", Style::default().fg(Color::Yellow)),
                Span::raw(" accept  "),
                Span::styled("n", Style::default().fg(Color::Yellow)),
                Span::raw(" reject  "),
            ]);
        }
        spans.extend([
            Span::styled("d", Style::default().fg(Color::Cyan)),
            Span::raw(" table  "),
            Span::styled("v", Style::default().fg(Color::Cyan)),
            Span::raw(" view  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]);
        spans
    };

    let mut final_spans = Vec::new();
    if let Some((label, color)) = send_feedback {
        final_spans.push(Span::styled(label, Style::default().fg(color)));
    }
    final_spans.extend(spans);
    frame.render_widget(Paragraph::new(Line::from(final_spans)), area);
}

fn activity_line_style(line: &str, max_width: usize) -> (String, Color) {
    if line.starts_with('\u{25b6}') {
        // Tool call: ▶ Edit …/file.rs
        (truncate_str(line, max_width), Color::Cyan)
    } else if line.starts_with('\u{276f}') {
        // User input: ❯ prompt text
        (truncate_str(line, max_width), Color::Yellow)
    } else if line.starts_with("  ") {
        // Tool output (indented)
        (truncate_str(line, max_width), Color::Rgb(140, 140, 140))
    } else {
        // Claude text
        (truncate_str(line, max_width), Color::White)
    }
}

pub(crate) fn build_block_title(session: &Session) -> String {
    let status = session.status.label();
    let project = &session.project_name;
    let dot = "\u{25cf}"; // ●
    match session.branch.as_deref() {
        Some(branch) if !branch.is_empty() => {
            format!(" {dot} {status}  {project} \u{b7} {branch} ") // · middle dot
        }
        _ => format!(" {dot} {status}  {project} "),
    }
}

pub(crate) fn build_meta_line(session: &Session) -> String {
    let pid = session
        .pid
        .map(|p| p.to_string())
        .unwrap_or_else(|| "\u{2014}".to_string());
    let model = session.model_display();
    let tokens = session.token_display();
    let pct = (session.token_ratio() * 100.0) as u32;
    format!(" pid: {pid}  \u{2502}  {model}  \u{2502}  {tokens}  {pct}%")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, SessionStatus};

    fn make_session(status: SessionStatus) -> Session {
        Session {
            session_id: "test-id".to_string(),
            project_name: "myapp".to_string(),
            branch: Some("main".to_string()),
            cwd: "/repos/myapp".to_string(),
            relative_dir: None,
            model: Some("claude-sonnet-4-6".to_string()),
            effort: None,
            total_input_tokens: 45_000,
            total_output_tokens: 5_000,
            status,
            pid: Some(1234),
            last_activity: Some("2026-03-24T10:00:00Z".to_string()),
            last_action: Some("Bash".to_string()),
            activity_log: vec!["\u{25b6} Bash cargo test".to_string(), "  running 5 tests".to_string(), "  ok".to_string()],
            started_at: 0,
            last_file_size: 0,
        }
    }

    #[test]
    fn block_title_includes_status_and_project_and_branch() {
        let s = make_session(SessionStatus::Working);
        let title = build_block_title(&s);
        assert!(title.contains("Working"), "title missing status: {title}");
        assert!(title.contains("myapp"), "title missing project: {title}");
        assert!(title.contains("main"), "title missing branch: {title}");
    }

    #[test]
    fn block_title_works_without_branch() {
        let mut s = make_session(SessionStatus::Idle);
        s.branch = None;
        let title = build_block_title(&s);
        assert!(title.contains("Idle"));
        assert!(title.contains("myapp"));
    }

    #[test]
    fn meta_line_includes_pid_and_tokens() {
        let s = make_session(SessionStatus::Idle);
        let meta = build_meta_line(&s);
        assert!(meta.contains("1234"), "meta missing pid: {meta}");
        assert!(meta.contains("50k"), "meta missing tokens: {meta}");
    }
}
