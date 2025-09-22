use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

use crate::config::{Config, SavedMigration, SavedComparison, ComparisonType};
use crate::dynamics::metadata::ViewInfo;

pub struct MigrationSelectMenu {
    migrations: Vec<SavedMigration>,
    list_state: ListState,
    quit: bool,
    action: Option<MigrationSelectAction>,
    show_confirm_dialog: bool,
    confirm_migration_name: String,
}

#[derive(Debug, Clone)]
pub enum MigrationSelectAction {
    CreateNew,
    LoadExisting(String),
    Exit,
}

impl MigrationSelectMenu {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let migrations = config.list_migrations().into_iter().cloned().collect();

        let mut menu = Self {
            migrations,
            list_state: ListState::default(),
            quit: false,
            action: None,
            show_confirm_dialog: false,
            confirm_migration_name: String::new(),
        };

        menu.list_state.select(Some(0));

        Ok(menu)
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<MigrationSelectAction> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if self.show_confirm_dialog {
                    self.handle_confirm_events(key.code)?;
                } else {
                    self.handle_menu_events(key.code)?;
                }
            }

            if self.quit {
                break;
            }
        }

        Ok(self.action.clone().unwrap_or(MigrationSelectAction::Exit))
    }

    fn handle_menu_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('q') => {
                self.action = Some(MigrationSelectAction::Exit);
                self.quit = true;
            }
            KeyCode::Esc => {
                self.action = Some(MigrationSelectAction::Exit);
                self.quit = true;
            }
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
            KeyCode::Enter => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.migrations.len() {
                        let migration = &self.migrations[selected];

                        // Update last used timestamp
                        if let Ok(mut config) = Config::load() {
                            let _ = config.touch_migration(&migration.name);
                        }

                        self.action = Some(MigrationSelectAction::LoadExisting(migration.name.clone()));
                        self.quit = true;
                    }
                }
            }
            KeyCode::Char('n') => {
                self.action = Some(MigrationSelectAction::CreateNew);
                self.quit = true;
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.migrations.len() {
                        let migration = &self.migrations[selected];
                        self.confirm_migration_name = migration.name.clone();
                        self.show_confirm_dialog = true;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_confirm_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('y') | KeyCode::Enter => {
                // Confirm deletion
                if let Ok(mut config) = Config::load() {
                    if config.remove_migration(&self.confirm_migration_name).is_ok() {
                        // Reload migrations list
                        self.migrations = config.list_migrations().into_iter().cloned().collect();
                        // Adjust selection if needed
                        if let Some(selected) = self.list_state.selected() {
                            if selected >= self.migrations.len() && self.migrations.len() > 0 {
                                self.list_state.select(Some(self.migrations.len() - 1));
                            } else if self.migrations.is_empty() {
                                self.list_state.select(None);
                            }
                        }
                    }
                }
                self.show_confirm_dialog = false;
                self.confirm_migration_name.clear();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                // Cancel deletion
                self.show_confirm_dialog = false;
                self.confirm_migration_name.clear();
            }
            _ => {}
        }
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("Migration Management")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(header, chunks[0]);

        // Main content - list of migrations
        let mut items = vec![];

        // Add existing migrations
        for migration in &self.migrations {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("📁 ", Style::default().fg(Color::Blue)),
                Span::styled(&migration.name, Style::default().fg(Color::White)),
                Span::styled(
                    format!(" ({} → {})", migration.source_env, migration.target_env),
                    Style::default().fg(Color::Gray),
                ),
            ])));
        }

        if items.is_empty() {
            items.push(ListItem::new(Line::from(vec![
                Span::styled("No saved migrations. Press 'n' to create one.", Style::default().fg(Color::Gray)),
            ])));
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Select Migration")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Footer
        let footer_text = "Enter: open | n: new | d/Del: delete | ↑↓: navigate | q/Esc: exit";
        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(footer, chunks[2]);

        // Show confirmation dialog if needed
        if self.show_confirm_dialog {
            self.render_confirm_dialog(f);
        }
    }

    fn render_confirm_dialog(&self, f: &mut Frame) {
        let popup_area = centered_rect(50, 25, f.area());

        // Clear the background
        f.render_widget(ratatui::widgets::Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Confirm Deletion")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Red).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Message
                Constraint::Length(1), // Controls
            ])
            .split(inner_area);

        // Message
        let message = format!("Delete migration '{}'?\nThis will also delete all associated views.", self.confirm_migration_name);
        let message_widget = Paragraph::new(message)
            .style(Style::default().fg(Color::White))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(message_widget, chunks[0]);

        // Controls
        let controls = Paragraph::new("y/Enter: Yes | n/Esc: No")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(controls, chunks[1]);
    }

    fn next(&mut self) {
        let total_items = if self.migrations.is_empty() { 1 } else { self.migrations.len() }; // Show placeholder if empty
        if total_items > 0 {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= total_items - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    fn previous(&mut self) {
        let total_items = if self.migrations.is_empty() { 1 } else { self.migrations.len() }; // Show placeholder if empty
        if total_items > 0 {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        total_items - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }
}

pub struct EnvironmentSelectMenu {
    environments: Vec<String>,
    source_selected: Option<usize>,
    target_selected: Option<usize>,
    current_selection: SelectionStage,
    list_state: ListState,
    quit: bool,
    action: Option<EnvironmentSelectAction>,
}

#[derive(Debug, Clone)]
pub enum EnvironmentSelectAction {
    Selected { source: String, target: String },
    Back,
}

#[derive(Debug, Clone, PartialEq)]
enum SelectionStage {
    SelectingSource,
    SelectingTarget,
}

impl EnvironmentSelectMenu {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let environments: Vec<String> = config.list_environments().into_iter().cloned().collect();

        let mut menu = Self {
            environments,
            source_selected: None,
            target_selected: None,
            current_selection: SelectionStage::SelectingSource,
            list_state: ListState::default(),
            quit: false,
            action: None,
        };

        if !menu.environments.is_empty() {
            menu.list_state.select(Some(0));
        }

        Ok(menu)
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<EnvironmentSelectAction> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.action = Some(EnvironmentSelectAction::Back);
                        self.quit = true;
                    }
                    KeyCode::Down => self.next(),
                    KeyCode::Up => self.previous(),
                    KeyCode::Enter => {
                        if let Some(selected) = self.list_state.selected() {
                            match self.current_selection {
                                SelectionStage::SelectingSource => {
                                    self.source_selected = Some(selected);
                                    self.current_selection = SelectionStage::SelectingTarget;
                                    // Reset selection for target
                                    self.list_state.select(Some(0));
                                }
                                SelectionStage::SelectingTarget => {
                                    self.target_selected = Some(selected);

                                    if let (Some(source_idx), Some(target_idx)) = (self.source_selected, self.target_selected) {
                                        if source_idx < self.environments.len() && target_idx < self.environments.len() {
                                            let source = self.environments[source_idx].clone();
                                            let target = self.environments[target_idx].clone();

                                            self.action = Some(EnvironmentSelectAction::Selected { source, target });
                                            self.quit = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if self.current_selection == SelectionStage::SelectingTarget {
                            self.current_selection = SelectionStage::SelectingSource;
                            self.target_selected = None;
                            if let Some(source_idx) = self.source_selected {
                                self.list_state.select(Some(source_idx));
                            }
                        }
                    }
                    _ => {}
                }
            }

            if self.quit {
                break;
            }
        }

        Ok(self.action.clone().unwrap_or(EnvironmentSelectAction::Back))
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(5), // Footer with status
            ])
            .split(f.area());

        // Header
        let title = match self.current_selection {
            SelectionStage::SelectingSource => "Select Source Environment",
            SelectionStage::SelectingTarget => "Select Target Environment",
        };

        let header = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(header, chunks[0]);

        // Main content - list of environments
        let items: Vec<ListItem> = self.environments
            .iter()
            .enumerate()
            .map(|(i, env)| {
                let mut spans = vec![Span::styled("🌐 ", Style::default().fg(Color::Blue))];

                // Highlight selected environments differently
                let style = if Some(i) == self.source_selected {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else if Some(i) == self.target_selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                spans.push(Span::styled(env, style));

                if Some(i) == self.source_selected {
                    spans.push(Span::styled(" (SOURCE)", Style::default().fg(Color::Green)));
                } else if Some(i) == self.target_selected {
                    spans.push(Span::styled(" (TARGET)", Style::default().fg(Color::Cyan)));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Environments")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Footer with status and controls
        let status_text = if let Some(source_idx) = self.source_selected {
            let source_name = &self.environments[source_idx];
            if let Some(target_idx) = self.target_selected {
                let target_name = &self.environments[target_idx];
                format!("Migration: {} → {} | Enter: confirm", source_name, target_name)
            } else {
                format!("Source: {} | Select target environment", source_name)
            }
        } else {
            "Select source environment".to_string()
        };

        let footer_lines = vec![
            Line::from(status_text),
            Line::from("Enter: select | ↑↓: navigate | Backspace: go back | q/Esc: cancel"),
        ];

        let footer = Paragraph::new(footer_lines)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(footer, chunks[2]);
    }

    fn next(&mut self) {
        if !self.environments.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= self.environments.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    fn previous(&mut self) {
        if !self.environments.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.environments.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }
}

pub struct ComparisonSelectMenu {
    migration: SavedMigration,
    comparisons: Vec<SavedComparison>,
    list_state: ListState,
    quit: bool,
    action: Option<ComparisonSelectAction>,
    input_mode: bool,
    type_selection_mode: bool,
    selected_comparison_type: ComparisonType,
    source_entity_input: String,
    target_entity_input: String,
    view_source_entity_input: String,
    view_target_entity_input: String,
    view_scope_all: bool,
    current_input: InputField,
    show_confirm_dialog: bool,
    confirm_comparison_name: String,
}

#[derive(Debug, Clone)]
pub enum ComparisonSelectAction {
    CreateNew { source_entity: String, target_entity: String, comparison_type: ComparisonType },
    OpenExisting { source_entity: String, target_entity: String, comparison_type: ComparisonType },
    Back,
}

#[derive(Debug, Clone, PartialEq)]
enum InputField {
    SourceEntity,
    TargetEntity,
    ViewSourceEntity,
    ViewTargetEntity,
    ViewScope,
}

impl ComparisonSelectMenu {
    pub fn new(migration: SavedMigration) -> Result<Self> {
        let mut menu = Self {
            comparisons: migration.comparisons.clone(),
            migration,
            list_state: ListState::default(),
            quit: false,
            action: None,
            input_mode: false,
            type_selection_mode: false,
            selected_comparison_type: ComparisonType::Entity,
            source_entity_input: String::new(),
            target_entity_input: String::new(),
            view_source_entity_input: String::new(),
            view_target_entity_input: String::new(),
            view_scope_all: true,
            current_input: InputField::SourceEntity,
            show_confirm_dialog: false,
            confirm_comparison_name: String::new(),
        };

        menu.list_state.select(Some(0));
        Ok(menu)
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<ComparisonSelectAction> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if self.show_confirm_dialog {
                    self.handle_confirm_events(key.code)?;
                } else if self.type_selection_mode {
                    self.handle_type_selection_events(key.code)?;
                } else if self.input_mode {
                    self.handle_input_events(key.code)?;
                } else {
                    self.handle_menu_events(key.code)?;
                }
            }

            if self.quit {
                break;
            }
        }

        Ok(self.action.clone().unwrap_or(ComparisonSelectAction::Back))
    }

    fn handle_menu_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.action = Some(ComparisonSelectAction::Back);
                self.quit = true;
            }
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
            KeyCode::Enter => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.comparisons.len() {
                        let comparison = &self.comparisons[selected];

                        // Update last used timestamp
                        if let Ok(mut config) = Config::load() {
                            let _ = config.touch_migration(&self.migration.name);
                        }

                        self.action = Some(ComparisonSelectAction::OpenExisting {
                            source_entity: comparison.source_entity.clone(),
                            target_entity: comparison.target_entity.clone(),
                            comparison_type: comparison.comparison_type.clone(),
                        });
                        self.quit = true;
                    }
                }
            }
            KeyCode::Char('n') => {
                // Quick shortcut to create new comparison - start with type selection
                self.type_selection_mode = true;
                self.selected_comparison_type = ComparisonType::Entity;
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(selected) = self.list_state.selected() {
                    if selected < self.comparisons.len() {
                        let comparison = &self.comparisons[selected];
                        self.confirm_comparison_name = comparison.name.clone();
                        self.show_confirm_dialog = true;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_type_selection_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => {
                self.type_selection_mode = false;
            }
            KeyCode::Up | KeyCode::Down => {
                // Toggle between Entity and View
                self.selected_comparison_type = match self.selected_comparison_type {
                    ComparisonType::Entity => ComparisonType::View,
                    ComparisonType::View => ComparisonType::Entity,
                };
            }
            KeyCode::Enter => {
                // Proceed to input for the selected type
                self.type_selection_mode = false;
                self.input_mode = true;

                match self.selected_comparison_type {
                    ComparisonType::Entity => {
                        self.source_entity_input.clear();
                        self.target_entity_input.clear();
                        self.current_input = InputField::SourceEntity;
                    }
                    ComparisonType::View => {
                        self.view_source_entity_input.clear();
                        self.view_target_entity_input.clear();
                        self.view_scope_all = true;
                        self.current_input = InputField::ViewSourceEntity;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_input_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => {
                self.input_mode = false;
                self.source_entity_input.clear();
                self.target_entity_input.clear();
            }
            KeyCode::Tab => {
                match self.selected_comparison_type {
                    ComparisonType::Entity => {
                        match self.current_input {
                            InputField::SourceEntity => self.current_input = InputField::TargetEntity,
                            InputField::TargetEntity => self.current_input = InputField::SourceEntity,
                            _ => self.current_input = InputField::SourceEntity,
                        }
                    }
                    ComparisonType::View => {
                        match self.current_input {
                            InputField::ViewSourceEntity => self.current_input = InputField::ViewTargetEntity,
                            InputField::ViewTargetEntity => self.current_input = InputField::ViewScope,
                            InputField::ViewScope => self.current_input = InputField::ViewSourceEntity,
                            _ => self.current_input = InputField::ViewSourceEntity,
                        }
                    }
                }
            }
            KeyCode::Enter => {
                match self.selected_comparison_type {
                    ComparisonType::Entity => {
                        if !self.source_entity_input.is_empty() {
                            let target_entity = if self.target_entity_input.is_empty() {
                                self.source_entity_input.clone()
                            } else {
                                self.target_entity_input.clone()
                            };

                            let comparison_name = if self.source_entity_input == target_entity {
                                self.source_entity_input.clone()
                            } else {
                                format!("{} → {}", self.source_entity_input, target_entity)
                            };

                            let new_comparison = SavedComparison {
                                name: comparison_name,
                                source_entity: self.source_entity_input.clone(),
                                target_entity: target_entity.clone(),
                                comparison_type: self.selected_comparison_type.clone(),
                                created_at: chrono::Utc::now().to_rfc3339(),
                                last_used: chrono::Utc::now().to_rfc3339(),
                            };

                            if let Ok(mut config) = Config::load() {
                                let _ = config.save_migration(self.migration.clone());
                                let _ = config.add_comparison_to_migration(&self.migration.name, new_comparison);
                            }

                            self.action = Some(ComparisonSelectAction::CreateNew {
                                source_entity: self.source_entity_input.clone(),
                                target_entity,
                                comparison_type: self.selected_comparison_type.clone(),
                            });
                            self.quit = true;
                        }
                    }
                    ComparisonType::View => {
                        if !self.view_source_entity_input.is_empty() {
                            let target_entity = if self.view_target_entity_input.is_empty() {
                                self.view_source_entity_input.clone()
                            } else {
                                self.view_target_entity_input.clone()
                            };

                            let scope_label = if self.view_scope_all { "All Views" } else { "Specific Views" };
                            let comparison_name = if self.view_source_entity_input == target_entity {
                                format!("{} Views - {}", self.view_source_entity_input, scope_label)
                            } else {
                                format!("{} → {} Views - {}", self.view_source_entity_input, target_entity, scope_label)
                            };

                            let new_comparison = SavedComparison {
                                name: comparison_name,
                                source_entity: self.view_source_entity_input.clone(),
                                target_entity: target_entity.clone(),
                                comparison_type: self.selected_comparison_type.clone(),
                                created_at: chrono::Utc::now().to_rfc3339(),
                                last_used: chrono::Utc::now().to_rfc3339(),
                            };

                            if let Ok(mut config) = Config::load() {
                                let _ = config.save_migration(self.migration.clone());
                                let _ = config.add_comparison_to_migration(&self.migration.name, new_comparison);
                            }

                            self.action = Some(ComparisonSelectAction::CreateNew {
                                source_entity: self.view_source_entity_input.clone(),
                                target_entity,
                                comparison_type: self.selected_comparison_type.clone(),
                            });
                            self.quit = true;
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                match self.current_input {
                    InputField::SourceEntity => {
                        self.source_entity_input.pop();
                    }
                    InputField::TargetEntity => {
                        self.target_entity_input.pop();
                    }
                    InputField::ViewSourceEntity => {
                        self.view_source_entity_input.pop();
                    }
                    InputField::ViewTargetEntity => {
                        self.view_target_entity_input.pop();
                    }
                    InputField::ViewScope => {
                        // Toggle scope when backspace is pressed
                        self.view_scope_all = !self.view_scope_all;
                    }
                }
            }
            KeyCode::Char(' ') => {
                // Handle space specifically for scope toggle
                if self.current_input == InputField::ViewScope {
                    self.view_scope_all = !self.view_scope_all;
                }
            }
            KeyCode::Char(c) => {
                match self.current_input {
                    InputField::SourceEntity => {
                        self.source_entity_input.push(c);
                    }
                    InputField::TargetEntity => {
                        self.target_entity_input.push(c);
                    }
                    InputField::ViewSourceEntity => {
                        self.view_source_entity_input.push(c);
                    }
                    InputField::ViewTargetEntity => {
                        self.view_target_entity_input.push(c);
                    }
                    InputField::ViewScope => {
                        // Toggle scope when any non-space char is pressed in scope field
                        self.view_scope_all = !self.view_scope_all;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_confirm_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('y') | KeyCode::Enter => {
                // Confirm deletion
                if let Ok(mut config) = Config::load() {
                    if config.remove_comparison_from_migration(&self.migration.name, &self.confirm_comparison_name).is_ok() {
                        // Reload views list
                        if let Some(updated_migration) = config.get_migration(&self.migration.name) {
                            self.comparisons = updated_migration.comparisons.clone();
                            self.migration = updated_migration.clone();
                        }
                        // Adjust selection if needed
                        if let Some(selected) = self.list_state.selected() {
                            if selected >= self.comparisons.len() && self.comparisons.len() > 0 {
                                self.list_state.select(Some(self.comparisons.len() - 1));
                            } else if self.comparisons.is_empty() {
                                self.list_state.select(None);
                            }
                        }
                    }
                }
                self.show_confirm_dialog = false;
                self.confirm_comparison_name.clear();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                // Cancel deletion
                self.show_confirm_dialog = false;
                self.confirm_comparison_name.clear();
            }
            _ => {}
        }
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        if self.type_selection_mode {
            self.render_type_selection_dialog(f);
        } else if self.input_mode {
            self.render_input_dialog(f);
        } else {
            self.render_comparison_list(f);
        }
    }

    fn render_type_selection_dialog(&mut self, f: &mut Frame) {
        let popup_area = centered_rect(50, 30, f.area());

        // Clear the background
        f.render_widget(ratatui::widgets::Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Select Comparison Type")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Instructions
                Constraint::Length(3), // Entity option
                Constraint::Length(3), // View option
                Constraint::Min(1),    // Help text
            ])
            .split(inner_area);

        // Instructions
        let instructions = Paragraph::new("Choose the type of comparison to create")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(instructions, chunks[0]);

        // Entity option
        let entity_style = if matches!(self.selected_comparison_type, ComparisonType::Entity) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let entity_option = Paragraph::new("● Entity Field Comparison")
            .block(Block::default().borders(Borders::ALL))
            .style(entity_style);
        f.render_widget(entity_option, chunks[1]);

        // View option
        let view_style = if matches!(self.selected_comparison_type, ComparisonType::View) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let view_option = Paragraph::new("● View Structure Comparison")
            .block(Block::default().borders(Borders::ALL))
            .style(view_style);
        f.render_widget(view_option, chunks[2]);

        // Help text
        let help_text = "↑↓: select type | Enter: continue | Esc: cancel";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[3]);
    }

    fn render_comparison_list(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer
            ])
            .split(f.area());

        // Header
        let header_text = format!("Comparisons for Migration: {}", self.migration.name);
        let header = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(header, chunks[0]);

        // Main content - list of comparisons
        let mut items = vec![];

        // Add existing comparisons
        for comparison in &self.comparisons {
            let display_text = match comparison.comparison_type {
                ComparisonType::Entity => {
                    // For entity comparisons, show arrow only when different
                    if comparison.source_entity == comparison.target_entity {
                        comparison.source_entity.clone()
                    } else {
                        format!("{} → {}", comparison.source_entity, comparison.target_entity)
                    }
                }
                ComparisonType::View => {
                    // For view comparisons, use the stored name (which already includes scope info)
                    // but ensure entity differences are clear
                    comparison.name.clone()
                }
            };

            let type_icon = match comparison.comparison_type {
                ComparisonType::Entity => "🔍 ",
                ComparisonType::View => "👁 ",
            };

            let type_label = match comparison.comparison_type {
                ComparisonType::Entity => " [Entity]",
                ComparisonType::View => " [View]",
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(type_icon, Style::default().fg(Color::Blue)),
                Span::styled(display_text, Style::default().fg(Color::White)),
                Span::styled(type_label, Style::default().fg(Color::Gray)),
            ])));
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Select Comparison")
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Footer
        let footer_text = "Enter: select | n: new | d/Del: delete | ↑↓: navigate | q/Esc: back";
        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(footer, chunks[2]);

        // Show confirmation dialog if needed
        if self.show_confirm_dialog {
            self.render_comparison_confirm_dialog(f);
        }
    }

    fn render_input_dialog(&mut self, f: &mut Frame) {
        match self.selected_comparison_type {
            ComparisonType::Entity => self.render_entity_input_dialog(f),
            ComparisonType::View => self.render_view_input_dialog(f),
        }
    }

    fn render_entity_input_dialog(&mut self, f: &mut Frame) {
        let popup_area = centered_rect(60, 40, f.area());

        // Clear the background
        f.render_widget(ratatui::widgets::Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Create New Comparison")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Instructions
                Constraint::Length(3), // Source entity input
                Constraint::Length(3), // Target entity input
                Constraint::Min(1),    // Help text
            ])
            .split(inner_area);

        // Instructions
        let type_name = match self.selected_comparison_type {
            ComparisonType::Entity => "entity field",
            ComparisonType::View => "view structure",
        };
        let instructions = Paragraph::new(format!("Create a new {} comparison", type_name))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(instructions, chunks[0]);

        // Source entity input
        let source_style = if self.current_input == InputField::SourceEntity {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let source_input = Paragraph::new(self.source_entity_input.as_str())
            .block(Block::default().title("Source Entity").borders(Borders::ALL))
            .style(source_style);
        f.render_widget(source_input, chunks[1]);

        // Target entity input
        let target_style = if self.current_input == InputField::TargetEntity {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let target_input = Paragraph::new(self.target_entity_input.as_str())
            .block(Block::default().title("Target Entity (optional - defaults to source)").borders(Borders::ALL))
            .style(target_style);
        f.render_widget(target_input, chunks[2]);

        // Help text
        let help_text = "Tab: switch fields | Enter: create comparison | Esc: cancel";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[3]);
    }

    fn render_view_input_dialog(&mut self, f: &mut Frame) {
        let popup_area = centered_rect(60, 40, f.area());

        // Clear the background
        f.render_widget(ratatui::widgets::Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Create New View Comparison")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Instructions
                Constraint::Length(3), // Source entity input
                Constraint::Length(3), // Target entity input
                Constraint::Length(3), // Scope selection
                Constraint::Min(1),    // Help text
            ])
            .split(inner_area);

        // Instructions
        let instructions = Paragraph::new("Create a new view structure comparison")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(instructions, chunks[0]);

        // Source entity input
        let source_entity_style = if self.current_input == InputField::ViewSourceEntity {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let source_entity_input = Paragraph::new(self.view_source_entity_input.as_str())
            .block(Block::default().title("Source Entity").borders(Borders::ALL))
            .style(source_entity_style);
        f.render_widget(source_entity_input, chunks[1]);

        // Target entity input
        let target_entity_style = if self.current_input == InputField::ViewTargetEntity {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let target_entity_input = Paragraph::new(self.view_target_entity_input.as_str())
            .block(Block::default().title("Target Entity (optional - defaults to source)").borders(Borders::ALL))
            .style(target_entity_style);
        f.render_widget(target_entity_input, chunks[2]);

        // Scope selection
        let scope_style = if self.current_input == InputField::ViewScope {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let scope_text = if self.view_scope_all { "● All Views" } else { "● Specific Views (not implemented)" };
        let scope_input = Paragraph::new(scope_text)
            .block(Block::default().title("Scope").borders(Borders::ALL))
            .style(scope_style);
        f.render_widget(scope_input, chunks[3]);

        // Help text
        let help_text = "Tab: switch fields | Space/Backspace: toggle scope | Enter: create | Esc: cancel";
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[4]);
    }

    fn render_comparison_confirm_dialog(&self, f: &mut Frame) {
        let popup_area = centered_rect(50, 25, f.area());

        // Clear the background
        f.render_widget(ratatui::widgets::Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Confirm Deletion")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Red).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Message
                Constraint::Length(1), // Controls
            ])
            .split(inner_area);

        // Message
        let message = format!("Delete comparison '{}'?", self.confirm_comparison_name);
        let message_widget = Paragraph::new(message)
            .style(Style::default().fg(Color::White))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(message_widget, chunks[0]);

        // Controls
        let controls = Paragraph::new("y/Enter: Yes | n/Esc: No")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(controls, chunks[1]);
    }

    fn next(&mut self) {
        let total_items = self.comparisons.len();
        if total_items > 0 {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i >= total_items - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    fn previous(&mut self) {
        let total_items = self.comparisons.len();
        if total_items > 0 {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        total_items - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }
}

pub struct ViewSelectMenu {
    source_views: Vec<ViewInfo>,
    target_views: Vec<ViewInfo>,
    source_list_state: ListState,
    target_list_state: ListState,
    focused_panel: ViewSelectPanel,
    quit: bool,
    action: Option<ViewSelectAction>,
}

#[derive(Debug, Clone, PartialEq)]
enum ViewSelectPanel {
    Source,
    Target,
}

#[derive(Debug, Clone)]
pub enum ViewSelectAction {
    Selected { source_view: ViewInfo, target_view: ViewInfo },
    Back,
}

impl ViewSelectMenu {
    pub fn new(source_views: Vec<ViewInfo>, target_views: Vec<ViewInfo>) -> Self {
        let mut menu = Self {
            source_views,
            target_views,
            source_list_state: ListState::default(),
            target_list_state: ListState::default(),
            focused_panel: ViewSelectPanel::Source,
            quit: false,
            action: None,
        };

        // Initialize with first item selected and sync target
        if !menu.source_views.is_empty() {
            menu.source_list_state.select(Some(0));
            menu.sync_target_selection(0);
        } else if !menu.target_views.is_empty() {
            menu.target_list_state.select(Some(0));
        }

        menu
    }


    fn calculate_name_similarity(&self, name1: &str, name2: &str) -> f32 {
        // Simple similarity calculation based on common characters and length
        if name1 == name2 {
            return 1.0;
        }

        let name1_lower = name1.to_lowercase();
        let name2_lower = name2.to_lowercase();

        // Check for exact match after lowercasing
        if name1_lower == name2_lower {
            return 0.9;
        }

        // Calculate similarity based on common characters
        let mut common_chars = 0;
        let name1_chars: Vec<char> = name1_lower.chars().collect();
        let name2_chars: Vec<char> = name2_lower.chars().collect();

        for c1 in &name1_chars {
            if name2_chars.contains(c1) {
                common_chars += 1;
            }
        }

        let max_len = name1_chars.len().max(name2_chars.len()) as f32;
        if max_len == 0.0 {
            return 0.0;
        }

        common_chars as f32 / max_len
    }

    fn sync_target_selection(&mut self, source_idx: usize) {
        // Only sync if source index is valid
        if source_idx >= self.source_views.len() {
            return;
        }

        let source_view = &self.source_views[source_idx];

        // Look for exact match first
        for (idx, target_view) in self.target_views.iter().enumerate() {
            if source_view.name.eq_ignore_ascii_case(&target_view.name) {
                self.target_list_state.select(Some(idx));
                return;
            }
        }

        // If no exact match, find best similarity match
        let mut best_idx = 0;
        let mut best_score = 0.0;

        for (idx, target_view) in self.target_views.iter().enumerate() {
            let score = self.calculate_name_similarity(&source_view.name, &target_view.name);
            if score > best_score {
                best_score = score;
                best_idx = idx;
            }
        }

        // Only sync if we found a reasonably good match (similarity > 0.3)
        if best_score > 0.3 {
            self.target_list_state.select(Some(best_idx));
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<ViewSelectAction> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key.code)?;
            }

            if self.quit {
                break;
            }
        }

        Ok(self.action.take().unwrap_or(ViewSelectAction::Back))
    }

    fn handle_key_event(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.quit = true;
                self.action = Some(ViewSelectAction::Back);
            }
            KeyCode::Tab => {
                self.focused_panel = match self.focused_panel {
                    ViewSelectPanel::Source => ViewSelectPanel::Target,
                    ViewSelectPanel::Target => ViewSelectPanel::Source,
                };
            }
            KeyCode::Up => {
                match self.focused_panel {
                    ViewSelectPanel::Source => {
                        if let Some(selected) = self.source_list_state.selected() {
                            if selected > 0 {
                                let new_idx = selected - 1;
                                self.source_list_state.select(Some(new_idx));
                                self.sync_target_selection(new_idx);
                            }
                        } else if !self.source_views.is_empty() {
                            self.source_list_state.select(Some(0));
                            self.sync_target_selection(0);
                        }
                    }
                    ViewSelectPanel::Target => {
                        if let Some(selected) = self.target_list_state.selected() {
                            if selected > 0 {
                                self.target_list_state.select(Some(selected - 1));
                            }
                        } else if !self.target_views.is_empty() {
                            self.target_list_state.select(Some(0));
                        }
                    }
                }
            }
            KeyCode::Down => {
                match self.focused_panel {
                    ViewSelectPanel::Source => {
                        if let Some(selected) = self.source_list_state.selected() {
                            if selected < self.source_views.len() - 1 {
                                let new_idx = selected + 1;
                                self.source_list_state.select(Some(new_idx));
                                self.sync_target_selection(new_idx);
                            }
                        } else if !self.source_views.is_empty() {
                            self.source_list_state.select(Some(0));
                            self.sync_target_selection(0);
                        }
                    }
                    ViewSelectPanel::Target => {
                        if let Some(selected) = self.target_list_state.selected() {
                            if selected < self.target_views.len() - 1 {
                                self.target_list_state.select(Some(selected + 1));
                            }
                        } else if !self.target_views.is_empty() {
                            self.target_list_state.select(Some(0));
                        }
                    }
                }
            }
            KeyCode::Enter => {
                let source_idx = self.source_list_state.selected().unwrap_or(0);
                let target_idx = self.target_list_state.selected().unwrap_or(0);

                if source_idx < self.source_views.len() && target_idx < self.target_views.len() {
                    self.quit = true;
                    self.action = Some(ViewSelectAction::Selected {
                        source_view: self.source_views[source_idx].clone(),
                        target_view: self.target_views[target_idx].clone(),
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer (increased height)
            ])
            .split(f.area());

        // Header
        let header = Paragraph::new("Select Views to Compare")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(header, chunks[0]);

        // Main content - split into two panels
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        // Source panel
        self.draw_source_panel(f, main_chunks[0]);

        // Target panel
        self.draw_target_panel(f, main_chunks[1]);

        // Footer
        let footer_text = "↑↓: Navigate Lists  Tab: Switch Between Panels  Enter: Compare Selected Views  q/Esc: Back";
        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
        f.render_widget(footer, chunks[2]);
    }

    fn draw_source_panel(&mut self, f: &mut Frame, area: Rect) {
        let source_items: Vec<ListItem> = self.source_views
            .iter()
            .map(|view| {
                let type_indicator = if view.is_custom { " (Custom)" } else { " (System)" };
                ListItem::new(Line::from(vec![
                    Span::raw(&view.name),
                    Span::styled(type_indicator, Style::default().fg(Color::Gray)),
                ]))
            })
            .collect();

        let selected_idx = self.source_list_state.selected().map(|i| i + 1).unwrap_or(0);
        let total_count = self.source_views.len();
        let title = format!("Source Views ({}/{})", selected_idx, total_count);

        let border_style = if self.focused_panel == ViewSelectPanel::Source {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let list = List::new(source_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );

        f.render_stateful_widget(list, area, &mut self.source_list_state);
    }

    fn draw_target_panel(&mut self, f: &mut Frame, area: Rect) {
        let target_items: Vec<ListItem> = self.target_views
            .iter()
            .map(|view| {
                let type_indicator = if view.is_custom { " (Custom)" } else { " (System)" };
                ListItem::new(Line::from(vec![
                    Span::raw(&view.name),
                    Span::styled(type_indicator, Style::default().fg(Color::Gray)),
                ]))
            })
            .collect();

        let selected_idx = self.target_list_state.selected().map(|i| i + 1).unwrap_or(0);
        let total_count = self.target_views.len();
        let title = format!("Target Views ({}/{})", selected_idx, total_count);

        let border_style = if self.focused_panel == ViewSelectPanel::Target {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let list = List::new(target_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );

        f.render_stateful_widget(list, area, &mut self.target_list_state);
    }
}

// Helper function to create a centered rectangle
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