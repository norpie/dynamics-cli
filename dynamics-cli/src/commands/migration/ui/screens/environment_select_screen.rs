use crate::{
    commands::migration::ui::{
        components::{FooterAction, ListAction, ListComponent, ListConfig},
        screens::{ComparisonSelectScreen, MigrationSelectScreen, Screen, ScreenResult},
        styles::STYLES,
    },
    config::{Config, SavedMigration},
};
use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub struct EnvironmentSelectScreen {
    environments: Vec<String>,
    list: ListComponent<String>,
    config: Config,
    migration: Option<SavedMigration>,
    selection_phase: SelectionPhase,
    source_env: Option<String>,
    target_env: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum SelectionPhase {
    SelectingSource,
    SelectingTarget,
}

impl EnvironmentSelectScreen {
    pub fn new(config: Config, migration: Option<SavedMigration>) -> Self {
        let environments: Vec<String> = config.environments.keys().cloned().collect();

        let list_config = ListConfig {
            title: Some("Select Source Environment".to_string()),
            allow_create_new: false,
            create_new_key: 'n',
            create_new_label: String::new(),
            enable_mouse: true,
            enable_scroll: true,
            show_indices: false,
            highlight_selected: true,
        };

        let list = ListComponent::new(environments.clone()).with_config(list_config);

        Self {
            environments,
            list,
            config,
            migration,
            selection_phase: SelectionPhase::SelectingSource,
            source_env: None,
            target_env: None,
        }
    }

    fn handle_environment_selected(&mut self, index: usize) -> ScreenResult {
        // Get the environment name from the currently displayed list, not the original list
        if let Some(env_name) = self.list.items().get(index) {
            match self.selection_phase {
                SelectionPhase::SelectingSource => {
                    self.source_env = Some(env_name.clone());
                    self.selection_phase = SelectionPhase::SelectingTarget;

                    // Update list title for target selection
                    let mut new_environments = self.environments.clone();
                    // Remove source environment from target options
                    new_environments.retain(|e| e != env_name);

                    let list_config = ListConfig {
                        title: Some("Select Target Environment".to_string()),
                        allow_create_new: false,
                        create_new_key: 'n',
                        create_new_label: String::new(),
                        enable_mouse: true,
                        enable_scroll: true,
                        show_indices: false,
                        highlight_selected: true,
                    };

                    self.list = ListComponent::new(new_environments).with_config(list_config);

                    ScreenResult::Continue
                }
                SelectionPhase::SelectingTarget => {
                    self.target_env = Some(env_name.clone());
                    self.create_migration()
                }
            }
        } else {
            ScreenResult::Continue
        }
    }

    fn create_migration(&mut self) -> ScreenResult {
        let source_env = self.source_env.as_ref().unwrap();
        let target_env = self.target_env.as_ref().unwrap();

        let migration = if let Some(existing_migration) = &self.migration {
            // Editing existing migration
            existing_migration.clone()
        } else {
            // Creating new migration
            let migration_name = format!("{}_to_{}", source_env, target_env);
            let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

            let new_migration = SavedMigration {
                name: migration_name,
                source_env: source_env.clone(),
                target_env: target_env.clone(),
                comparisons: Vec::new(),
                created_at: now.clone(),
                last_used: now,
            };

            // Save the new migration to the config
            if let Err(e) = self.config.save_migration(new_migration.clone()) {
                log::error!("Failed to save migration: {}", e);
                return ScreenResult::Continue;
            }

            new_migration
        };

        // Navigate to comparison select screen
        ScreenResult::Navigate(Box::new(ComparisonSelectScreen::new(
            self.config.clone(),
            migration,
        )))
    }

    fn go_back(&mut self) -> ScreenResult {
        match self.selection_phase {
            SelectionPhase::SelectingSource => {
                // Go back to migration select screen
                ScreenResult::Navigate(Box::new(MigrationSelectScreen::new(self.config.clone())))
            }
            SelectionPhase::SelectingTarget => {
                // Go back to source selection within this screen
                self.selection_phase = SelectionPhase::SelectingSource;
                self.source_env = None;

                let list_config = ListConfig {
                    title: Some("Select Source Environment".to_string()),
                    allow_create_new: false,
                    create_new_key: 'n',
                    create_new_label: String::new(),
                    enable_mouse: true,
                    enable_scroll: true,
                    show_indices: false,
                    highlight_selected: true,
                };

                self.list = ListComponent::new(self.environments.clone()).with_config(list_config);

                ScreenResult::Continue
            }
        }
    }
}

impl Screen for EnvironmentSelectScreen {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        let content_area = Rect {
            x: area.x + 1,
            y: area.y,
            width: area.width.saturating_sub(2),
            height: area.height,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Status area
                Constraint::Min(0),    // List area
            ])
            .split(content_area);

        // Render status information
        self.render_status(f, chunks[0]);

        // Render environment list
        self.list.render(f, chunks[1]);
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
                    KeyCode::Esc => self.go_back(),
                    _ => {
                        match self.list.handle_key(key.code) {
                            ListAction::Selected(index) => self.handle_environment_selected(index),
                            ListAction::CreateNew => ScreenResult::Continue, // Not used
                            ListAction::None => ScreenResult::Continue,
                        }
                    }
                }
            }
            Event::Mouse(mouse) => {
                match self.list.handle_mouse(mouse, Rect::default()) {
                    ListAction::Selected(index) => self.handle_environment_selected(index),
                    ListAction::CreateNew => ScreenResult::Continue, // Not used
                    ListAction::None => ScreenResult::Continue,
                }
            }
            _ => ScreenResult::Continue,
        }
    }

    fn get_footer_actions(&self) -> Vec<FooterAction> {
        vec![
            FooterAction {
                key: "↑↓".to_string(),
                description: "Navigate".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "Enter".to_string(),
                description: "Select Environment".to_string(),
                enabled: !self.environments.is_empty(),
            },
            FooterAction {
                key: "Esc".to_string(),
                description: match self.selection_phase {
                    SelectionPhase::SelectingSource => "Back to Migrations".to_string(),
                    SelectionPhase::SelectingTarget => "Back to Source".to_string(),
                },
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
        Some("Environment Selection".to_string())
    }
}

impl EnvironmentSelectScreen {
    fn render_status(&self, f: &mut Frame, area: Rect) {
        let mut lines = Vec::new();

        match self.selection_phase {
            SelectionPhase::SelectingSource => {
                lines.push(Line::from(vec![
                    Span::styled("Step 1/2: ", STYLES.info),
                    Span::styled("Select the source environment", STYLES.normal),
                ]));
            }
            SelectionPhase::SelectingTarget => {
                lines.push(Line::from(vec![
                    Span::styled("Step 2/2: ", STYLES.info),
                    Span::styled("Select the target environment", STYLES.normal),
                ]));
                if let Some(ref source) = self.source_env {
                    lines.push(Line::from(vec![
                        Span::styled("Source: ", STYLES.info),
                        Span::styled(source, STYLES.highlighted),
                    ]));
                }
            }
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(STYLES.info)
                .title("Environment Selection"),
        );

        f.render_widget(paragraph, area);
    }
}
