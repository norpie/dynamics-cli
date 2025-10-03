use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use anyhow::Result;
use std::io;

use super::data_transformer::TransformedRecord;

#[derive(Clone)]
pub struct ValidationError {
    pub row_number: usize,
    pub message: String,
    pub error_type: ValidationErrorType,
}

#[derive(Clone)]
pub enum ValidationErrorType {
    EntityWarning,    // Yellow - unmatched entity columns from validation popup
    TransformationError, // Red - transformation validation errors
}

pub struct ValidationErrorsState {
    pub errors: Vec<ValidationError>,
    pub list_state: ListState,
    pub summary: String,
    pub should_exit: bool,
    pub continue_processing: bool,
}

impl ValidationErrorsState {
    pub fn new(
        transformed_records: &[TransformedRecord],
        validation_result: Option<&crate::commands::deadlines::validation::ValidationResult>
    ) -> Self {
        let mut errors = Vec::new();
        let mut total_warnings = 0;

        // Add entity validation warnings (from popup) first
        if let Some(validation_result) = validation_result {
            for unmatched_column in &validation_result.unmatched_columns {
                errors.push(ValidationError {
                    row_number: 0, // Entity warnings are not row-specific
                    message: format!("Unmatched entity column: '{}'", unmatched_column),
                    error_type: ValidationErrorType::EntityWarning,
                });
                total_warnings += 1;
            }
        }

        // Collect all transformation validation errors
        for record in transformed_records {
            if !record.validation_warnings.is_empty() {
                for warning in &record.validation_warnings {
                    errors.push(ValidationError {
                        row_number: record.excel_row_number,
                        message: warning.clone(),
                        error_type: ValidationErrorType::TransformationError,
                    });
                    total_warnings += 1;
                }
            }
        }

        let summary = if total_warnings == 0 {
            "✅ No validation errors found!".to_string()
        } else {
            format!(
                "⚠️  {} validation errors found across {} rows",
                total_warnings,
                errors.iter().map(|e| e.row_number).collect::<std::collections::HashSet<_>>().len()
            )
        };

        let mut list_state = ListState::default();
        if !errors.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            errors,
            list_state,
            summary,
            should_exit: false,
            continue_processing: false,
        }
    }

    pub fn next(&mut self) {
        if self.errors.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.errors.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.errors.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.errors.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn continue_processing(&mut self) {
        self.continue_processing = true;
        self.should_exit = true;
    }

    pub fn quit(&mut self) {
        self.continue_processing = false;
        self.should_exit = true;
    }
}

pub fn run_validation_errors_tui(
    transformed_records: &[TransformedRecord],
    validation_result: Option<&crate::commands::deadlines::validation::ValidationResult>
) -> Result<bool> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut state = ValidationErrorsState::new(transformed_records, validation_result);

    // If no errors, return immediately
    if state.errors.is_empty() {
        cleanup_terminal(&mut terminal)?;
        return Ok(true); // Continue processing
    }

    // Run the TUI
    let result = run_tui(&mut terminal, &mut state);

    // Cleanup
    cleanup_terminal(&mut terminal)?;

    match result {
        Ok(_) => Ok(state.continue_processing),
        Err(e) => Err(e),
    }
}

fn cleanup_terminal<B: Backend + std::io::Write>(terminal: &mut Terminal<B>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_tui<B: Backend>(terminal: &mut Terminal<B>, state: &mut ValidationErrorsState) -> Result<()> {
    loop {
        terminal.draw(|f| render_validation_errors_screen(f, state))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    state.quit();
                    break;
                }
                KeyCode::Char('c') | KeyCode::Enter => {
                    state.continue_processing();
                    break;
                }
                KeyCode::Down => {
                    state.next();
                }
                KeyCode::Up => {
                    state.previous();
                }
                KeyCode::Esc => {
                    state.quit();
                    break;
                }
                _ => {}
            }
        }

        if state.should_exit {
            break;
        }
    }

    Ok(())
}

fn render_validation_errors_screen(f: &mut Frame, state: &mut ValidationErrorsState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Summary
            Constraint::Min(0),    // Error list
            Constraint::Length(3), // Instructions
        ])
        .split(f.area());

    // Summary section
    render_summary(f, chunks[0], &state.summary);

    // Error list
    render_error_list(f, chunks[1], state);

    // Instructions
    render_instructions(f, chunks[2]);
}

fn render_summary(f: &mut Frame, area: Rect, summary: &str) {
    let summary_text = vec![
        Line::from(vec![
            Span::styled("Validation Errors", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(summary),
    ];

    let summary_paragraph = Paragraph::new(summary_text)
        .block(Block::default().borders(Borders::ALL).title("Summary"))
        .wrap(Wrap { trim: true });

    f.render_widget(summary_paragraph, area);
}

fn render_error_list(f: &mut Frame, area: Rect, state: &mut ValidationErrorsState) {
    if state.errors.is_empty() {
        let no_errors = Paragraph::new("✅ No validation errors found!")
            .block(Block::default().borders(Borders::ALL).title("Errors"))
            .style(Style::default().fg(Color::Green));
        f.render_widget(no_errors, area);
        return;
    }

    let items: Vec<ListItem> = state
        .errors
        .iter()
        .enumerate()
        .map(|(i, error)| {
            let content = match error.error_type {
                ValidationErrorType::EntityWarning => {
                    error.message.clone() // Entity warnings don't have row numbers
                }
                ValidationErrorType::TransformationError => {
                    format!("Row {}: {}", error.row_number, error.message)
                }
            };

            let style = if state.list_state.selected() == Some(i) {
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else {
                match error.error_type {
                    ValidationErrorType::EntityWarning => Style::default().fg(Color::Yellow),
                    ValidationErrorType::TransformationError => Style::default().fg(Color::Red),
                }
            };

            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    let error_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Validation Errors"))
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black));

    f.render_stateful_widget(error_list, area, &mut state.list_state);
}

fn render_instructions(f: &mut Frame, area: Rect) {
    let instructions = vec![
        Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(": Navigate  "),
            Span::styled("Enter/c", Style::default().fg(Color::Green)),
            Span::raw(": Continue Processing  "),
            Span::styled("q/Esc", Style::default().fg(Color::Red)),
            Span::raw(": Quit"),
        ]),
    ];

    let instructions_paragraph = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .wrap(Wrap { trim: true });

    f.render_widget(instructions_paragraph, area);
}