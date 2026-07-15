//! Full-screen local skill discovery view.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use codexctl_core::skills::DiscoveredSkill;
use codexctl_core::theme::Theme;

pub fn render_skills_screen(frame: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let title = Line::from(vec![
        Span::styled(" codexctl ", Style::default().fg(t.text_primary)),
        Span::styled(
            "│ Skills ",
            Style::default().fg(t.header).add_modifier(Modifier::BOLD),
        ),
    ]);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.header));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let footer_height = if app
        .skills_status_msg
        .as_deref()
        .is_some_and(|msg| !msg.is_empty())
    {
        2
    } else {
        1
    };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(footer_height),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("{} skills discovered", app.skills.len()),
            Style::default()
                .fg(t.text_primary)
                .add_modifier(Modifier::BOLD),
        ))),
        layout[0],
    );
    render_skills_body(frame, layout[1], app);
    render_footer(frame, layout[2], app);
}

fn render_skills_body(frame: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    if app.skills.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No skills found in ~/.codex/skills, ~/.codex/plugins/*/skills, or ./.codex/skills.",
                    Style::default().fg(t.text_muted),
                )),
            ])
            .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(area);
    let items = app
        .skills
        .iter()
        .map(|skill| ListItem::new(skill_line(skill, t)))
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(
        app.skills_selected.min(app.skills.len().saturating_sub(1)),
    ));
    let list = List::new(items)
        .highlight_style(Style::default().fg(t.header).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, chunks[0], &mut state);

    let detail = app
        .skills
        .get(app.skills_selected)
        .map(|skill| {
            vec![
                Line::from(vec![
                    Span::styled("Path:   ", Style::default().fg(t.text_muted)),
                    Span::styled(
                        skill.path.display().to_string(),
                        Style::default().fg(t.text_primary),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Source: ", Style::default().fg(t.text_muted)),
                    Span::styled(skill.source.label(), Style::default().fg(t.text_primary)),
                ]),
            ]
        })
        .unwrap_or_else(|| vec![Line::from("Select a skill with j/k.")]);
    frame.render_widget(Paragraph::new(detail), chunks[1]);
}

fn skill_line<'a>(skill: &'a DiscoveredSkill, t: &'a Theme) -> Line<'a> {
    let source = skill
        .plugin
        .as_ref()
        .map(|plugin| format!("{}:{plugin}", skill.source.label()))
        .unwrap_or_else(|| skill.source.label().to_string());
    let description = if skill.description.is_empty() {
        "(no description)".to_string()
    } else {
        truncate(&skill.description, 60)
    };

    Line::from(vec![
        Span::styled(" • ", Style::default().fg(t.text_muted)),
        Span::styled(
            format!("{:<28}", truncate(&skill.name, 28)),
            Style::default()
                .fg(t.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<18}", truncate(&source, 18)),
            Style::default().fg(t.text_muted),
        ),
        Span::styled(
            format!("{:>6.1}kb  ", skill.size_bytes as f64 / 1024.0),
            Style::default().fg(t.text_muted),
        ),
        Span::styled(description, Style::default().fg(t.text_muted)),
    ])
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        let prefix = value
            .chars()
            .take(max.saturating_sub(1))
            .collect::<String>();
        format!("{prefix}…")
    }
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let mut lines = vec![Line::from(vec![
        Span::styled(" j/k", Style::default().fg(t.highlight_key)),
        Span::raw(":nav  "),
        Span::styled("r", Style::default().fg(t.highlight_key)),
        Span::raw(":rescan  "),
        Span::styled("Esc/K", Style::default().fg(t.highlight_key)),
        Span::raw(":close"),
    ])];
    if let Some(message) = app
        .skills_status_msg
        .as_deref()
        .filter(|msg| !msg.is_empty())
    {
        lines.push(Line::from(Span::styled(
            format!(" {message}"),
            Style::default().fg(t.success).add_modifier(Modifier::BOLD),
        )));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_strings() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghijklm", 6), "abcde…");
    }
}
