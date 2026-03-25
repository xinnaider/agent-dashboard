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
    let chunks =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());
    if let Some(idx) = app.detail_expanded {
        if let Some(session) = app.sessions.get(idx) {
            render_expanded(frame, session, app.detail_scroll, chunks[0]);
        }
    } else {
        render_grid(frame, app, chunks[0]);
    }
    render_footer(frame, app, chunks[1]);
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
    let scroll = if selected + 1 > visible { selected + 1 - visible } else { 0 };

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

    // Lines 4+: bash output
    if inner.height > 3 {
        let bash_lines: Vec<&str> = session
            .last_bash_lines
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        let available = (inner.height - 3) as usize;
        let start = bash_lines.len().saturating_sub(available);
        for bash_line in bash_lines.iter().skip(start).take(available) {
            lines.push(Line::from(Span::styled(
                truncate_str(bash_line, w),
                Style::default().fg(Color::Rgb(140, 140, 140)),
            )));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_expanded(frame: &mut Frame, session: &Session, scroll: usize, area: Rect) {
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

    let bash_lines: Vec<&str> = session
        .last_bash_lines
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let mut lines: Vec<Line> = Vec::new();

    if bash_lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No bash output recorded",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let max_scroll = bash_lines.len().saturating_sub(available);
        let start = scroll.min(max_scroll);
        for bash_line in bash_lines.iter().skip(start).take(available) {
            lines.push(Line::from(Span::styled(
                truncate_str(bash_line, w),
                Style::default().fg(Color::Rgb(200, 200, 200)),
            )));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let spans = if app.detail_expanded.is_some() {
        vec![
            Span::styled("j/k", Style::default().fg(Color::Cyan)),
            Span::raw(" scroll  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" back  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]
    } else {
        vec![
            Span::styled("j/k", Style::default().fg(Color::Cyan)),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" expand  "),
            Span::styled("d", Style::default().fg(Color::Cyan)),
            Span::raw(" table  "),
            Span::styled("v", Style::default().fg(Color::Cyan)),
            Span::raw(" view  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]
    };
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
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
            last_bash_lines: Some(vec!["running 5 tests".to_string(), "ok".to_string()]),
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
