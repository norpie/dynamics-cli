use crate::{
    commands::migration::ui::{
        components::{
            ConfirmationAction, ConfirmationDialog, FooterAction, ListAction, ListComponent,
            ListConfig, ModalComponent,
        },
        screens::{ComparisonSelectScreen, EnvironmentSelectScreen, Screen, ScreenResult},
    },
    config::{Config, SavedMigration},
};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

pub struct MigrationSelectScreen {
    migrations: Vec<SavedMigration>,
    list: ListComponent<MigrationItem>,
    config: Config,
    show_delete_confirmation: bool,
    delete_confirmation_modal: Option<ModalComponent<ConfirmationDialog>>,
}

#[derive(Clone)]
struct MigrationItem {
    migration: SavedMigration,
}

impl std::fmt::Display for MigrationItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} → {}",
            self.migration.source_env, self.migration.target_env
        )
    }
}

impl MigrationSelectScreen {
    pub fn new(config: Config) -> Self {
        let migrations: Vec<SavedMigration> = config.migrations.values().cloned().collect();
        let migration_items: Vec<MigrationItem> = migrations
            .iter()
            .map(|m| MigrationItem {
                migration: m.clone(),
            })
            .collect();

        let list_config = ListConfig {
            title: Some("Select Migration".to_string()),
            allow_create_new: true,
            create_new_key: 'n',
            create_new_label: "Create New Migration".to_string(),
            enable_mouse: true,
            enable_scroll: true,
            show_indices: false,
            highlight_selected: true,
        };

        let list = ListComponent::new(migration_items).with_config(list_config);

        Self {
            migrations,
            list,
            config,
            show_delete_confirmation: false,
            delete_confirmation_modal: None,
        }
    }

    fn handle_migration_selected(&mut self, index: usize) -> ScreenResult {
        if let Some(migration_item) = self.list.items().get(index) {
            let migration = migration_item.migration.clone();
            // Navigate to comparison select screen for this migration
            ScreenResult::Navigate(Box::new(ComparisonSelectScreen::new(
                self.config.clone(),
                migration,
            )))
        } else {
            ScreenResult::Continue
        }
    }

    fn handle_create_new(&mut self) -> ScreenResult {
        // Navigate to environment select screen to create new migration
        ScreenResult::Navigate(Box::new(EnvironmentSelectScreen::new(
            self.config.clone(),
            None,
        )))
    }

    fn handle_delete_selected(&mut self) -> ScreenResult {
        if self.migrations.is_empty() {
            return ScreenResult::Continue;
        }

        if let Some(selected) = self.list.selected() {
            if let Some(migration_item) = self.list.items().get(selected) {
                let migration_name = &migration_item.migration.name;
                let dialog = ConfirmationDialog::new(
                    "Delete Migration".to_string(),
                    format!(
                        "Are you sure you want to delete the migration '{}'?\n\nThis action cannot be undone.",
                        migration_name
                    ),
                )
                .with_buttons("Delete".to_string(), "Cancel".to_string());

                self.delete_confirmation_modal = Some(ModalComponent::new(dialog));
                self.show_delete_confirmation = true;
            }
        }
        ScreenResult::Continue
    }

    fn handle_confirmation_result(&mut self, action: ConfirmationAction) -> ScreenResult {
        if let ConfirmationAction::Confirmed = action {
            if let Some(selected) = self.list.selected() {
                if let Some(migration_item) = self.list.items().get(selected) {
                    let migration_name = &migration_item.migration.name;

                    // Remove from config
                    if let Err(e) = self.config.remove_migration(migration_name) {
                        log::error!("Failed to delete migration: {}", e);
                        self.show_delete_confirmation = false;
                        self.delete_confirmation_modal = None;
                        return ScreenResult::Continue;
                    }

                    // Update local state - recreate the screen with updated data
                    return ScreenResult::Navigate(Box::new(MigrationSelectScreen::new(
                        self.config.clone(),
                    )));
                }
            }
        }

        self.show_delete_confirmation = false;
        self.delete_confirmation_modal = None;
        ScreenResult::Continue
    }
}

impl Screen for MigrationSelectScreen {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        // Create a simple layout with left/right margin only
        let content_area = Rect {
            x: area.x + 1,
            y: area.y,
            width: area.width.saturating_sub(2),
            height: area.height,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(content_area);

        self.list.render(f, chunks[0]);

        // Render modal if open
        if self.show_delete_confirmation {
            if let Some(modal) = &mut self.delete_confirmation_modal {
                modal.render(f, area);
            }
        }
    }

    fn handle_event(&mut self, event: Event) -> ScreenResult {
        // Handle modal events first if modal is open
        if self.show_delete_confirmation {
            if let Some(modal) = &mut self.delete_confirmation_modal {
                match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        match modal.handle_key(key.code) {
                            crate::commands::migration::ui::components::modal_component::ModalAction::Close => {
                                let dialog = modal.content_mut();
                                if let Some(action) = dialog.take_action() {
                                    return self.handle_confirmation_result(action);
                                }
                                self.show_delete_confirmation = false;
                                self.delete_confirmation_modal = None;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            return ScreenResult::Continue;
        }

        // Handle normal screen events
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
                    KeyCode::Esc => {
                        ScreenResult::Exit // Root screen - exit application
                    }
                    KeyCode::Delete => {
                        self.handle_delete_selected()
                    }
                    _ => match self.list.handle_key(key.code) {
                        ListAction::Selected(index) => self.handle_migration_selected(index),
                        ListAction::CreateNew => self.handle_create_new(),
                        ListAction::None => ScreenResult::Continue,
                    },
                }
            }
            Event::Mouse(mouse) => match self.list.handle_mouse(mouse, Rect::default()) {
                ListAction::Selected(index) => self.handle_migration_selected(index),
                ListAction::CreateNew => self.handle_create_new(),
                ListAction::None => ScreenResult::Continue,
            },
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
                description: "Select Migration".to_string(),
                enabled: !self.migrations.is_empty(),
            },
            FooterAction {
                key: "n".to_string(),
                description: "New Migration".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "Del".to_string(),
                description: "Delete Migration".to_string(),
                enabled: !self.migrations.is_empty(),
            },
            FooterAction {
                key: "Ctrl+Q".to_string(),
                description: "Quit".to_string(),
                enabled: true,
            },
        ]
    }

    fn get_title(&self) -> Option<String> {
        Some("Migration Manager".to_string())
    }
}
