use crate::{
    commands::migration::ui::{
        components::FooterAction,
        screens::{ComparisonSelectScreen, LoadingScreen, Screen, ScreenResult},
    },
    config::{Config, EntityComparison, SavedComparison, SavedMigration},
};
use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub struct EntitySelectScreen {
    config: Config,
    migration: SavedMigration,
    source_entity: String,
    target_entity: String,
    focused_field: FocusedField,
}

#[derive(Debug, Clone, PartialEq)]
enum FocusedField {
    SourceEntity,
    TargetEntity,
}

impl EntitySelectScreen {
    pub fn new(config: Config, migration: SavedMigration) -> Self {
        Self {
            config,
            migration,
            source_entity: String::new(),
            target_entity: String::new(),
            focused_field: FocusedField::SourceEntity,
        }
    }

    fn handle_create_comparison(&mut self) -> ScreenResult {
        if self.source_entity.trim().is_empty() {
            return ScreenResult::Continue; // Could show error message
        }
        if self.target_entity.trim().is_empty() {
            return ScreenResult::Continue; // Could show error message
        }

        // Create new comparison
        let comparison_name = format!("{}_to_{}", self.source_entity.trim(), self.target_entity.trim());
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let new_comparison = SavedComparison {
            name: comparison_name.clone(),
            source_entity: self.source_entity.trim().to_string(),
            target_entity: self.target_entity.trim().to_string(),
            entity_comparison: EntityComparison::default(),
            view_comparisons: Vec::new(),
            created_at: now.clone(),
            last_used: now,
        };

        // Add comparison to migration
        if let Err(e) = self.config.add_comparison_to_migration(&self.migration.name, new_comparison.clone()) {
            log::error!("Failed to save comparison: {}", e);
            return ScreenResult::Continue;
        }

        // Update the migration with the new comparison
        let mut updated_migration = self.migration.clone();
        updated_migration.comparisons.push(new_comparison.clone());

        // Navigate to loading screen to fetch data and show the comparison
        ScreenResult::Navigate(Box::new(LoadingScreen::new(
            self.config.clone(),
            updated_migration,
            new_comparison,
        )))
    }

    fn handle_back(&mut self) -> ScreenResult {
        ScreenResult::Navigate(Box::new(ComparisonSelectScreen::new(
            self.config.clone(),
            self.migration.clone(),
        )))
    }

    fn handle_char_input(&mut self, c: char) {
        match self.focused_field {
            FocusedField::SourceEntity => {
                self.source_entity.push(c);
            }
            FocusedField::TargetEntity => {
                self.target_entity.push(c);
            }
        }
    }

    fn handle_backspace(&mut self) {
        match self.focused_field {
            FocusedField::SourceEntity => {
                self.source_entity.pop();
            }
            FocusedField::TargetEntity => {
                self.target_entity.pop();
            }
        }
    }

    fn switch_focus(&mut self) {
        self.focused_field = match self.focused_field {
            FocusedField::SourceEntity => FocusedField::TargetEntity,
            FocusedField::TargetEntity => FocusedField::SourceEntity,
        };
    }
}

impl Screen for EntitySelectScreen {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        // Create centered area for the form
        let form_width = 60;
        let form_height = 12;
        let x = (area.width.saturating_sub(form_width)) / 2;
        let y = (area.height.saturating_sub(form_height)) / 2;

        let form_area = Rect {
            x: x.max(1),
            y: y.max(1),
            width: form_width.min(area.width.saturating_sub(2)),
            height: form_height.min(area.height.saturating_sub(2)),
        };

        // Clear the area
        f.render_widget(Clear, form_area);

        // Main form block
        let form_block = Block::default()
            .title("Create New Comparison")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));
        f.render_widget(form_block, form_area);

        // Inner area for form content
        let inner = form_area.inner(Margin { horizontal: 1, vertical: 1 });

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Source entity field
                Constraint::Length(3), // Target entity field
                Constraint::Length(2), // Instructions
                Constraint::Min(1),    // Spacer
            ])
            .split(inner);

        // Source entity field
        let source_focused = self.focused_field == FocusedField::SourceEntity;
        let source_style = if source_focused {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let source_block = Block::default()
            .title("Source Entity")
            .borders(Borders::ALL)
            .border_style(source_style);

        let source_text = if source_focused && self.source_entity.is_empty() {
            "Enter source entity name..."
        } else {
            &self.source_entity
        };

        let source_paragraph = Paragraph::new(source_text)
            .block(source_block)
            .style(if source_focused && self.source_entity.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            });
        f.render_widget(source_paragraph, chunks[0]);

        // Target entity field
        let target_focused = self.focused_field == FocusedField::TargetEntity;
        let target_style = if target_focused {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let target_block = Block::default()
            .title("Target Entity")
            .borders(Borders::ALL)
            .border_style(target_style);

        let target_text = if target_focused && self.target_entity.is_empty() {
            "Enter target entity name..."
        } else {
            &self.target_entity
        };

        let target_paragraph = Paragraph::new(target_text)
            .block(target_block)
            .style(if target_focused && self.target_entity.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            });
        f.render_widget(target_paragraph, chunks[1]);

        // Instructions
        let instructions = Paragraph::new("Use Tab to switch fields, Enter to create comparison")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(instructions, chunks[2]);
    }

    fn handle_event(&mut self, event: Event) -> ScreenResult {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                match key.code {
                    KeyCode::Char('q')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        ScreenResult::Exit
                    }
                    KeyCode::Esc => self.handle_back(),
                    KeyCode::Enter => self.handle_create_comparison(),
                    KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                        self.switch_focus();
                        ScreenResult::Continue
                    }
                    KeyCode::Char(c) => {
                        self.handle_char_input(c);
                        ScreenResult::Continue
                    }
                    KeyCode::Backspace => {
                        self.handle_backspace();
                        ScreenResult::Continue
                    }
                    _ => ScreenResult::Continue,
                }
            }
            _ => ScreenResult::Continue,
        }
    }

    fn get_footer_actions(&self) -> Vec<FooterAction> {
        vec![
            FooterAction {
                key: "Tab".to_string(),
                description: "Switch Field".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "Enter".to_string(),
                description: "Create Comparison".to_string(),
                enabled: !self.source_entity.trim().is_empty() && !self.target_entity.trim().is_empty(),
            },
            FooterAction {
                key: "Esc".to_string(),
                description: "Back".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "Ctrl+Q".to_string(),
                description: "Quit".to_string(),
                enabled: true,
            },
        ]
    }

    fn get_title(&self) -> Option<String> {
        Some(format!("New Comparison - {}", self.migration.name))
    }
}

// Need to add import for Margin
use ratatui::layout::Margin;