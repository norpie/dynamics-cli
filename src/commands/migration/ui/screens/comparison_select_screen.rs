use crate::{
    commands::migration::ui::{
        components::{
            FooterAction, ListAction, ListComponent, ListConfig, ModalComponent, TextModalContent,
        },
        screens::{EntitySelectScreen, LoadingScreen, MigrationSelectScreen, Screen, ScreenResult},
        styles::STYLES,
    },
    config::{Config, SavedComparison, SavedMigration},
};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub struct ComparisonSelectScreen {
    migration: SavedMigration,
    comparisons: Vec<SavedComparison>,
    list: ListComponent<ComparisonItem>,
    config: Config,
    show_entity_modal: bool,
    entity_modal: Option<ModalComponent<TextModalContent>>,
}

#[derive(Clone)]
struct ComparisonItem {
    comparison: SavedComparison,
}

impl std::fmt::Display for ComparisonItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} → {}",
            self.comparison.source_entity, self.comparison.target_entity
        )
    }
}

impl ComparisonSelectScreen {
    pub fn new(config: Config, migration: SavedMigration) -> Self {
        let comparisons = migration.comparisons.clone();
        let comparison_items: Vec<ComparisonItem> = comparisons
            .iter()
            .map(|c| ComparisonItem {
                comparison: c.clone(),
            })
            .collect();

        let list_config = ListConfig {
            title: Some(format!("Comparisons - {}", migration.name)),
            allow_create_new: true,
            create_new_key: 'n',
            create_new_label: "Create New Comparison".to_string(),
            enable_mouse: true,
            enable_scroll: true,
            show_indices: false,
            highlight_selected: true,
        };

        let list = ListComponent::new(comparison_items).with_config(list_config);

        Self {
            migration,
            comparisons,
            list,
            config,
            show_entity_modal: false,
            entity_modal: None,
        }
    }

    fn handle_comparison_selected(&mut self, index: usize) -> ScreenResult {
        if let Some(comparison_item) = self.list.items().get(index) {
            let comparison = comparison_item.comparison.clone();
            // Navigate to LoadingScreen which will fetch data and then show UnifiedCompareScreen
            ScreenResult::Navigate(Box::new(LoadingScreen::new(
                self.config.clone(),
                self.migration.clone(),
                comparison,
            )))
        } else {
            ScreenResult::Continue
        }
    }

    fn handle_create_new(&mut self) -> ScreenResult {
        // Navigate to entity selection screen
        ScreenResult::Navigate(Box::new(EntitySelectScreen::new(
            self.config.clone(),
            self.migration.clone(),
        )))
    }

    fn show_entity_selection_modal(&mut self) {
        let content = TextModalContent::new(
            "Entity Selection\n\nThis would normally show a list of available entities from both source and target environments.\n\nFor now, this is a placeholder that demonstrates the modal system.\n\nPress Esc to close.".to_string()
        );

        self.entity_modal = Some(
            ModalComponent::new(content)
                .with_title("Select Entities to Compare".to_string())
                .with_size(70, 50),
        );
        self.show_entity_modal = true;
    }

    fn show_comparison_placeholder(&mut self, comparison_name: &str) -> ScreenResult {
        let content = TextModalContent::new(format!(
            "Opening comparison: {}\n\nThis would normally navigate to the UnifiedCompareScreen to show the detailed comparison view.\n\nFor now, this is a placeholder.\n\nPress Esc to close.",
            comparison_name
        ));

        self.entity_modal = Some(
            ModalComponent::new(content)
                .with_title("Comparison View".to_string())
                .with_size(70, 40),
        );
        self.show_entity_modal = true;
        ScreenResult::Continue
    }

    fn close_modal(&mut self) {
        self.show_entity_modal = false;
        self.entity_modal = None;
    }
}

impl Screen for ComparisonSelectScreen {
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
                Constraint::Length(4), // Migration info area
                Constraint::Min(0),    // List area
            ])
            .split(content_area);

        // Render migration information
        self.render_migration_info(f, chunks[0]);

        // Render comparison list
        self.list.render(f, chunks[1]);

        // Render modal if active
        if self.show_entity_modal
            && let Some(ref mut modal) = self.entity_modal
        {
            modal.render(f, area);
        }
    }

    fn handle_event(&mut self, event: Event) -> ScreenResult {
        // Handle modal events first if modal is active
        if self.show_entity_modal {
            if let Some(ref mut modal) = self.entity_modal {
                match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        match modal.handle_key(key.code) {
                            crate::commands::migration::ui::components::ModalAction::Close => {
                                self.close_modal();
                                ScreenResult::Continue
                            }
                            _ => ScreenResult::Continue,
                        }
                    }
                    Event::Mouse(mouse) => match modal.handle_mouse(mouse, Rect::default()) {
                        crate::commands::migration::ui::components::ModalAction::Close => {
                            self.close_modal();
                            ScreenResult::Continue
                        }
                        _ => ScreenResult::Continue,
                    },
                    _ => ScreenResult::Continue,
                }
            } else {
                ScreenResult::Continue
            }
        } else {
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
                            // Go back to migration select screen
                            ScreenResult::Navigate(Box::new(MigrationSelectScreen::new(
                                self.config.clone(),
                            )))
                        }
                        _ => match self.list.handle_key(key.code) {
                            ListAction::Selected(index) => self.handle_comparison_selected(index),
                            ListAction::CreateNew => self.handle_create_new(),
                            ListAction::None => ScreenResult::Continue,
                        },
                    }
                }
                Event::Mouse(mouse) => match self.list.handle_mouse(mouse, Rect::default()) {
                    ListAction::Selected(index) => self.handle_comparison_selected(index),
                    ListAction::CreateNew => self.handle_create_new(),
                    ListAction::None => ScreenResult::Continue,
                },
                _ => ScreenResult::Continue,
            }
        }
    }

    fn get_footer_actions(&self) -> Vec<FooterAction> {
        if self.show_entity_modal {
            vec![FooterAction {
                key: "Esc".to_string(),
                description: "Close Modal".to_string(),
                enabled: true,
            }]
        } else {
            vec![
                FooterAction {
                    key: "↑↓".to_string(),
                    description: "Navigate".to_string(),
                    enabled: true,
                },
                FooterAction {
                    key: "Enter".to_string(),
                    description: "Open Comparison".to_string(),
                    enabled: !self.comparisons.is_empty(),
                },
                FooterAction {
                    key: "n".to_string(),
                    description: "New Comparison".to_string(),
                    enabled: true,
                },
                FooterAction {
                    key: "Esc".to_string(),
                    description: "Back to Migrations".to_string(),
                    enabled: true,
                },
                FooterAction {
                    key: "Ctrl+Q".to_string(),
                    description: "Quit".to_string(),
                    enabled: true,
                },
            ]
        }
    }

    fn get_title(&self) -> Option<String> {
        Some(format!("Migration: {}", self.migration.name))
    }
}

impl ComparisonSelectScreen {
    fn render_migration_info(&self, f: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(vec![
                Span::styled("Source: ", STYLES.info),
                Span::styled(&self.migration.source_env, STYLES.highlighted),
                Span::styled(" → Target: ", STYLES.info),
                Span::styled(&self.migration.target_env, STYLES.highlighted),
            ]),
            Line::from(vec![
                Span::styled("Comparisons: ", STYLES.info),
                Span::styled(self.comparisons.len().to_string(), STYLES.highlighted),
                Span::styled(" total", STYLES.normal),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(STYLES.info)
                .title("Migration Information"),
        );

        f.render_widget(paragraph, area);
    }
}
