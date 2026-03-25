use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::app::App;
use crate::session::SessionStatus;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    render_table(frame, app, chunks[0]);
    render_footer(frame, chunks[1]);
}

fn render_table(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from(" # "),
        Cell::from("PID"),
        Cell::from("Project"),
        Cell::from("Branch"),
        Cell::from("Status"),
        Cell::from("Model"),
        Cell::from("Context"),
        Cell::from("Last Activity"),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let num = format!(" {} ", i + 1);

            let (status_dot, status_label, status_color) = match session.status {
                SessionStatus::New => ("\u{25cf}", "New", Color::Blue),
                SessionStatus::Working => ("\u{25cf}", "Working", Color::Green),
                SessionStatus::Idle => ("\u{25cf}", "Idle", Color::DarkGray),
                SessionStatus::Input => ("\u{25cf}", "Input", Color::Yellow),
            };

            let token_ratio = session.token_ratio();
            let token_style = if token_ratio > 0.9 {
                Style::default().fg(Color::Red)
            } else if token_ratio > 0.75 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let activity = session
                .last_activity
                .as_deref()
                .map(format_timestamp)
                .unwrap_or_else(|| "\u{2014}".to_string());

            let project_cell = {
                let mut spans = vec![Span::raw(&session.project_name)];
                if let Some(dir) = &session.relative_dir {
                    spans.push(Span::styled("::", Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled(dir.clone(), Style::default().fg(Color::Cyan)));
                }
                Cell::from(Line::from(spans))
            };

            let branch_cell = Cell::from(
                session.branch.as_deref().unwrap_or("\u{2014}"),
            ).style(Style::default().fg(Color::Green));

            let status_cell = Cell::from(Line::from(vec![
                Span::styled(status_dot, Style::default().fg(status_color)),
                Span::styled(format!(" {status_label}"), Style::default().fg(status_color)),
            ]));

            let pid_str = session
                .pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "\u{2014}".to_string());

            let row = Row::new(vec![
                Cell::from(num),
                Cell::from(pid_str),
                project_cell,
                branch_cell,
                status_cell,
                Cell::from(session.model_display()),
                Cell::from(session.token_display()).style(token_style),
                Cell::from(activity),
            ]);

            if session.status == SessionStatus::Input {
                row.style(Style::default().bg(Color::Rgb(50, 40, 0)))
            } else if i == app.selected {
                row.style(Style::default().bg(Color::DarkGray))
            } else {
                row
            }
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Fill(2),
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(20),
        Constraint::Length(14),
        Constraint::Length(14),
    ];

    let any_input = app.sessions.iter().any(|s| s.status == SessionStatus::Input);
    let border_color = if any_input {
        if app.tick.is_multiple_of(2) { Color::Yellow } else { Color::White }
    } else {
        Color::Reset
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(" agent-dashboard \u{2014} Claude Code Sessions "),
        );

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Cyan)),
        Span::raw(" navigate  "),
        Span::styled("v", Style::default().fg(Color::Cyan)),
        Span::raw(" view  "),
        Span::styled("r", Style::default().fg(Color::Cyan)),
        Span::raw(" refresh  "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(" quit"),
    ]));
    frame.render_widget(footer, area);
}

fn format_timestamp(ts: &str) -> String {
    use chrono::{DateTime, Local, Utc};

    match ts.parse::<DateTime<Utc>>() {
        Ok(dt) => {
            let now = Utc::now();
            let diff = now - dt;

            if diff.num_seconds() < 60 {
                "< 1m".to_string()
            } else if diff.num_minutes() < 60 {
                format!("{}m ago", diff.num_minutes())
            } else if diff.num_hours() < 24 {
                format!("{}h ago", diff.num_hours())
            } else {
                dt.with_timezone(&Local).format("%b %d %H:%M").to_string()
            }
        }
        Err(_) => ts.to_string(),
    }
}
