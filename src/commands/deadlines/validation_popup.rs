use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::validation::ValidationResult;

pub fn show_validation_popup(validation_result: &ValidationResult) -> Result<bool> {
    if validation_result.unmatched_columns.is_empty() {
        return Ok(true); // No popup needed, continue
    }

    let mut terminal = ratatui::init();
    terminal.clear()?;

    loop {
        terminal.draw(|f| {
            render_validation_popup(f, validation_result);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => {
                    ratatui::restore();
                    return Ok(true); // Continue
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    ratatui::restore();
                    return Ok(false); // Quit
                }
                _ => {}
            }
        }
    }
}

fn render_validation_popup(f: &mut Frame, validation_result: &ValidationResult) {
    let size = f.area();

    // Create popup area (60% width, 70% height, centered)
    let popup_area = centered_rect(60, 70, size);

    // Clear the area
    f.render_widget(Clear, popup_area);

    // Main popup block
    let popup_block = Block::default()
        .title("⚠️  Entity Validation Warning")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(popup_block, popup_area);

    // Inner area for content
    let inner_area = popup_area.inner(Margin::new(2, 1));

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Summary
            Constraint::Min(5),    // Unmatched list
            Constraint::Length(3), // Instructions
        ])
        .split(inner_area);

    // Summary text
    let summary_text = vec![
        Line::from(vec![
            Span::styled("Found ", Style::default()),
            Span::styled(
                format!("{}", validation_result.unmatched_columns.len()),
                Style::default().fg(Color::Red)
            ),
            Span::styled(" unmatched entity columns.", Style::default()),
        ]),
        Line::from("These columns were not found in the cached entity data."),
        Line::from(""),
    ];

    let summary = Paragraph::new(summary_text)
        .wrap(Wrap { trim: true });
    f.render_widget(summary, chunks[0]);

    // Unmatched columns list
    let unmatched_items: Vec<ListItem> = validation_result
        .unmatched_columns
        .iter()
        .map(|column| {
            ListItem::new(Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::Red)),
                Span::styled(format!("'{}'", column), Style::default()),
            ]))
        })
        .collect();

    let unmatched_list = List::new(unmatched_items)
        .block(
            Block::default()
                .title("Unmatched Columns")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Red))
        );
    f.render_widget(unmatched_list, chunks[1]);

    // Instructions
    let instructions = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::styled(" to continue, or ", Style::default()),
            Span::styled("Esc/Q", Style::default().fg(Color::Red)),
            Span::styled(" to quit", Style::default()),
        ]),
    ];

    let instructions_para = Paragraph::new(instructions)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(instructions_para, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}