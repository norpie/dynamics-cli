use crate::{
    commands::migration::{
        ui::{
            components::{ExamplesModal, ExamplesAction, FooterAction, ManualMappingModal, ModalComponent, PrefixMappingModal, hierarchy_tree::SortMode},
            screens::{
                ComparisonSelectScreen, Screen, ScreenResult,
                comparison::{
                    data_models::*, event_handlers::EventHandlers,
                    field_mapping_manager::FieldMappingManager, render_helpers::RenderHelpers,
                },
                comparison_apps::{ComparisonApp, common::ComparisonApp as ComparisonAppTrait},
            },
        },
    },
    config::{Config, SavedComparison},
    dynamics::metadata::{FieldInfo, FormInfo, ViewInfo},
};
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{Frame, layout::Rect, widgets::ListState};
use std::collections::HashMap;
use serde_json::Value;

// Type alias to avoid namespace conflict
type CommonComparisonData = super::comparison_apps::common::ComparisonData;

pub struct UnifiedCompareScreen {
    pub comparison: SavedComparison,
    pub config: Config,
    loading_state: LoadingState,
    comparison_data: Option<ComparisonData>,
    shared_state: SharedState,
    active_tab: ActiveTab,
    focused_side: FocusedSide,
    sort_mode: SortMode,

    // UI components (apps)
    fields_app: Box<dyn ComparisonAppTrait>,
    relationships_app: Box<dyn ComparisonAppTrait>,
    views_app: Box<dyn ComparisonAppTrait>,
    forms_app: Box<dyn ComparisonAppTrait>,

    // UI state - kept for compatibility
    source_area: Option<Rect>,
    target_area: Option<Rect>,
    tab_area: Option<Rect>,

    // Legacy state for Views and Forms (to be removed when fully migrated)
    source_views_state: ListState,
    target_views_state: ListState,
    source_forms_state: ListState,
    target_forms_state: ListState,
    source_view_items: Vec<String>,
    target_view_items: Vec<String>,
    source_form_items: Vec<String>,
    target_form_items: Vec<String>,

    // Modal state
    prefix_modal: Option<ModalComponent<PrefixMappingModal>>,
    manual_modal: Option<ModalComponent<ManualMappingModal>>,
    examples_modal: Option<ModalComponent<ExamplesModal>>,
}

impl UnifiedCompareScreen {
    pub fn new(config: Config, comparison: SavedComparison) -> Self {
        let prefix_mappings = config
            .get_prefix_mappings(&comparison.source_entity, &comparison.target_entity)
            .cloned()
            .unwrap_or_default();

        let field_mappings = config
            .get_field_mappings(&comparison.source_entity, &comparison.target_entity)
            .cloned()
            .unwrap_or_default();

        // Load examples from config
        let examples: Vec<ExamplePair> = config
            .get_examples(&comparison.source_entity, &comparison.target_entity)
            .map(|config_examples| {
                config_examples.iter().map(ExamplePair::from_config).collect()
            })
            .unwrap_or_default();

        let mut examples_state = ExamplesState::new();
        for example in examples {
            examples_state.add_example(example);
        }

        let shared_state = SharedState {
            field_mappings,
            prefix_mappings,
            hide_matched: false,
            examples: examples_state,
        };

        // Create placeholder apps - these will be properly initialized later
        let fields_app: Box<dyn ComparisonAppTrait> = Box::new(ComparisonApp::new_fields_app(
            comparison.source_entity.clone(),
            comparison.target_entity.clone(),
        ));
        let relationships_app: Box<dyn ComparisonAppTrait> =
            Box::new(ComparisonApp::new_relationships_app(
                comparison.source_entity.clone(),
                comparison.target_entity.clone(),
            ));
        let views_app: Box<dyn ComparisonAppTrait> = Box::new(ComparisonApp::new_views_app(
            comparison.source_entity.clone(),
            comparison.target_entity.clone(),
        ));
        let forms_app: Box<dyn ComparisonAppTrait> = Box::new(ComparisonApp::new_forms_app(
            comparison.source_entity.clone(),
            comparison.target_entity.clone(),
        ));

        Self {
            comparison,
            config,
            loading_state: LoadingState::NotStarted,
            comparison_data: None,
            shared_state,
            active_tab: ActiveTab::Fields,
            focused_side: FocusedSide::Source,
            sort_mode: SortMode::Alphabetical,
            fields_app,
            relationships_app,
            views_app,
            forms_app,
            source_area: None,
            target_area: None,
            tab_area: None,
            source_views_state: ListState::default(),
            target_views_state: ListState::default(),
            source_forms_state: ListState::default(),
            target_forms_state: ListState::default(),
            source_view_items: Vec::new(),
            target_view_items: Vec::new(),
            source_form_items: Vec::new(),
            target_form_items: Vec::new(),
            prefix_modal: None,
            manual_modal: None,
            examples_modal: None,
        }
    }

    pub fn new_with_data(
        config: Config,
        comparison: SavedComparison,
        source_fields: Vec<FieldInfo>,
        target_fields: Vec<FieldInfo>,
        source_views: Vec<ViewInfo>,
        target_views: Vec<ViewInfo>,
        source_forms: Vec<FormInfo>,
        target_forms: Vec<FormInfo>,
        source_env: String,
        target_env: String,
        example_data: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        let mut screen = Self::new(config, comparison);

        // Compute field matches
        let field_matches = FieldMappingManager::compute_field_matches(
            &screen.shared_state,
            &source_fields,
            &target_fields,
        );

        // Create comparison data
        screen.comparison_data = Some(ComparisonData {
            source_fields,
            target_fields,
            source_views,
            target_views,
            source_forms,
            target_forms,
            source_entity: screen.comparison.source_entity.clone(),
            target_entity: screen.comparison.target_entity.clone(),
            source_env,
            target_env,
            field_matches,
            view_matches: Vec::new(), // TODO: implement view matching
            form_matches: Vec::new(), // TODO: implement form matching
        });

        screen.loading_state = LoadingState::Complete;

        // Store example data in shared state
        log::debug!("Storing {} example data items in shared state", example_data.len());
        for (uuid, _) in &example_data {
            log::debug!("  Stored example data for UUID: {}", uuid);
        }
        screen.shared_state.examples.example_data = example_data;

        log::debug!("Examples state after loading:");
        log::debug!("  examples_mode_enabled: {}", screen.shared_state.examples.examples_mode_enabled);
        log::debug!("  examples count: {}", screen.shared_state.examples.examples.len());
        log::debug!("  example_data count: {}", screen.shared_state.examples.example_data.len());
        if let Some(active_id) = &screen.shared_state.examples.active_example_id {
            log::debug!("  active_example_id: {}", active_id);
        } else {
            log::debug!("  active_example_id: None");
        }

        screen.create_ui_components();

        screen
    }

    fn start_loading(&mut self) {
        self.loading_state = LoadingState::LoadingSourceFields;
        self.create_mock_data();
    }

    fn create_mock_data(&mut self) {
        self.loading_state = LoadingState::Complete;
    }

    fn create_ui_components(&mut self) {
        if let Some(ref data) = self.comparison_data {
            let common_data = CommonComparisonData {
                source_fields: data.source_fields.clone(),
                target_fields: data.target_fields.clone(),
                source_views: data.source_views.clone(),
                target_views: data.target_views.clone(),
                source_forms: data.source_forms.clone(),
                target_forms: data.target_forms.clone(),
                source_entity: data.source_entity.clone(),
                target_entity: data.target_entity.clone(),
                field_mappings: self.shared_state.field_mappings.clone(),
                prefix_mappings: self.shared_state.prefix_mappings.clone(),
                hide_matched: self.shared_state.hide_matched,
            };

            self.fields_app.update_data(&common_data);
            self.relationships_app.update_data(&common_data);
            self.views_app.update_data(&common_data);
            self.forms_app.update_data(&common_data);

            // Apply initial sorting
            self.apply_sorting_to_all_apps();
        }
    }

    fn open_prefix_modal(&mut self) {
        let modal_content = PrefixMappingModal::new(self.shared_state.prefix_mappings.clone());
        self.prefix_modal = Some(ModalComponent::new(modal_content));
    }

    fn recompute_field_matches(&mut self) {
        if let Some(ref mut data) = self.comparison_data {
            FieldMappingManager::recompute_field_matches(&self.shared_state, data);
            self.create_ui_components();
        }
    }

    fn open_manual_modal(&mut self) {
        let modal_content = ManualMappingModal::new(self.shared_state.field_mappings.clone());
        self.manual_modal = Some(ModalComponent::new(modal_content));
    }

    fn add_manual_match(&mut self) {
        // Only proceed if we're on the Fields tab and have comparison data
        if !matches!(self.active_tab, ActiveTab::Fields) || self.comparison_data.is_none() {
            return;
        }

        // Get selected fields from both sides
        // For now, we'll need to add a method to get selected field names from the comparison apps
        // This is a placeholder - we'll need to implement getting selected fields from the fields app
        if let (Some(source_field), Some(target_field)) = (
            self.get_selected_source_field(),
            self.get_selected_target_field(),
        ) {
            if let Err(e) = FieldMappingManager::add_manual_mapping(
                &mut self.config,
                &mut self.shared_state,
                self.comparison_data.as_ref().unwrap(),
                &source_field,
                &target_field,
            ) {
                log::error!("Failed to add manual mapping: {}", e);
            } else {
                self.recompute_field_matches();
                log::info!("Added manual mapping: {} → {}", source_field, target_field);
            }
        }
    }

    fn get_selected_source_field(&self) -> Option<String> {
        // TODO: Implement getting selected field from source side of fields app
        // This would need to be implemented in the comparison apps
        None
    }

    fn get_selected_target_field(&self) -> Option<String> {
        // TODO: Implement getting selected field from target side of fields app
        // This would need to be implemented in the comparison apps
        None
    }

    fn toggle_examples_mode(&mut self) {
        self.shared_state.examples.toggle_examples_mode();

        // Trigger a re-render by recomputing matches (this will refresh the field rendering)
        if let Some(data) = &self.comparison_data {
            self.recompute_field_matches();
        }
    }

    fn open_examples_modal(&mut self) {
        let modal_content = ExamplesModal::new(
            self.shared_state.examples.examples.clone(),
            self.shared_state.examples.active_example_id.clone(),
        );
        self.examples_modal = Some(ModalComponent::new(modal_content));
    }

    fn handle_examples_action(&mut self, action: ExamplesAction) {
        use crate::commands::migration::ui::components::examples_modal::ExamplesAction;
        match action {
            ExamplesAction::Delete(id) => {
                self.shared_state.examples.remove_example(&id);
                log::info!("Deleted example with id: {}", id);

                // Save to config
                if let Err(e) = self.config.remove_example(
                    &self.comparison.source_entity,
                    &self.comparison.target_entity,
                    &id,
                ) {
                    log::error!("Failed to save example deletion to config: {}", e);
                }

                // Refresh modal to show updated list
                self.refresh_examples_modal();
            }
            ExamplesAction::SetActive(id) => {
                self.shared_state.examples.set_active_example(&id);
                log::info!("Set active example: {}", id);

                // Save entire examples list to config (to persist active selection)
                self.save_examples_to_config();

                // Refresh modal to show updated selection
                self.refresh_examples_modal();

                // Example data is now fetched during loading screen

                // If examples mode is enabled, refresh the field rendering
                if self.shared_state.examples.examples_mode_enabled {
                    self.recompute_field_matches();
                }
            }
            ExamplesAction::AddExample(source_uuid, target_uuid) => {
                let example_pair = ExamplePair::new(source_uuid.clone(), target_uuid.clone());

                // Add to config
                if let Err(e) = self.config.add_example(
                    &self.comparison.source_entity,
                    &self.comparison.target_entity,
                    example_pair.to_config(),
                ) {
                    log::error!("Failed to save example to config: {}", e);
                }

                self.shared_state.examples.add_example(example_pair);
                log::info!("Added new example: {} → {}", source_uuid, target_uuid);

                // Refresh modal to show updated list
                self.refresh_examples_modal();

                // Example data is now fetched during loading screen

                // If examples mode is enabled, refresh the field rendering
                if self.shared_state.examples.examples_mode_enabled {
                    self.recompute_field_matches();
                }
            }
        }
    }

    fn refresh_examples_modal(&mut self) {
        // Recreate the modal with updated data
        let modal_content = ExamplesModal::new(
            self.shared_state.examples.examples.clone(),
            self.shared_state.examples.active_example_id.clone(),
        );
        self.examples_modal = Some(ModalComponent::new(modal_content));
    }

    fn save_examples_to_config(&mut self) {
        let config_examples: Vec<crate::config::ConfigExamplePair> = self
            .shared_state
            .examples
            .examples
            .iter()
            .map(|e| e.to_config())
            .collect();

        if let Err(e) = self.config.update_examples(
            &self.comparison.source_entity,
            &self.comparison.target_entity,
            config_examples,
        ) {
            log::error!("Failed to save examples to config: {}", e);
        }
    }

    /// Apply current sort mode to all comparison apps
    fn apply_sorting_to_all_apps(&mut self) {
        self.fields_app.apply_sorting(&self.sort_mode);
        self.relationships_app.apply_sorting(&self.sort_mode);
        self.views_app.apply_sorting(&self.sort_mode);
        self.forms_app.apply_sorting(&self.sort_mode);
    }

}

impl Screen for UnifiedCompareScreen {
    fn render(&mut self, f: &mut Frame, area: Rect) {
        // Handle modal rendering first
        if let Some(ref mut modal) = self.prefix_modal {
            modal.render(f, area);
            return;
        }

        if let Some(ref mut modal) = self.manual_modal {
            modal.render(f, area);
            return;
        }

        if let Some(ref mut modal) = self.examples_modal {
            modal.render(f, area);
            return;
        }

        RenderHelpers::render_main_screen(
            f,
            area,
            &self.loading_state,
            &self.comparison,
            &self.active_tab,
            self.focused_side,
            &mut *self.fields_app,
            &mut *self.relationships_app,
            &mut *self.views_app,
            &mut *self.forms_app,
            &mut self.tab_area,
            &mut self.source_area,
            &mut self.target_area,
            &self.shared_state.examples,
            &mut || {}, // No loading functionality in unified screen
        );
    }

    fn handle_event(&mut self, event: Event) -> ScreenResult {
        // Handle modal events first
        if let Some(ref mut modal) = self.prefix_modal {
            let modal_action = match event {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    modal.handle_key(key_event.code)
                }
                Event::Mouse(mouse_event) => modal.handle_mouse(mouse_event, Rect::default()),
                _ => crate::commands::migration::ui::components::modal_component::ModalAction::None,
            };

            match modal_action {
                crate::commands::migration::ui::components::modal_component::ModalAction::ContentAction(_) => {
                    // Check if the modal has a specific action to take
                    if let Some(action) = modal.content_mut().take_action() {
                        if let Err(e) = FieldMappingManager::handle_prefix_action(
                            &mut self.config,
                            &mut self.shared_state,
                            self.comparison_data.as_ref().unwrap(),
                            action,
                        ) {
                            log::error!("Failed to handle prefix action: {}", e);
                        }
                        self.recompute_field_matches();
                    }
                }
                crate::commands::migration::ui::components::modal_component::ModalAction::Close => {
                    self.prefix_modal = None;
                }
                _ => {}
            }
            return ScreenResult::Continue;
        }

        // Handle manual modal events
        if let Some(ref mut modal) = self.manual_modal {
            let modal_action = match event {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    modal.handle_key(key_event.code)
                }
                Event::Mouse(mouse_event) => modal.handle_mouse(mouse_event, Rect::default()),
                _ => crate::commands::migration::ui::components::modal_component::ModalAction::None,
            };

            match modal_action {
                crate::commands::migration::ui::components::modal_component::ModalAction::ContentAction(_) => {
                    // Check if the modal has a specific action to take
                    if let Some(action) = modal.content_mut().take_action() {
                        if let Err(e) = FieldMappingManager::handle_manual_action(
                            &mut self.config,
                            &mut self.shared_state,
                            self.comparison_data.as_ref().unwrap(),
                            action,
                        ) {
                            log::error!("Failed to handle manual action: {}", e);
                        }
                        self.recompute_field_matches();
                    }
                }
                crate::commands::migration::ui::components::modal_component::ModalAction::Close => {
                    self.manual_modal = None;
                }
                _ => {}
            }
            return ScreenResult::Continue;
        }

        // Handle examples modal events
        if let Some(ref mut modal) = self.examples_modal {
            let modal_action = match event {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    modal.handle_key(key_event.code)
                }
                Event::Mouse(mouse_event) => modal.handle_mouse(mouse_event, Rect::default()),
                _ => crate::commands::migration::ui::components::modal_component::ModalAction::None,
            };

            match modal_action {
                crate::commands::migration::ui::components::modal_component::ModalAction::ContentAction(_) => {
                    // Check if the modal has a specific action to take
                    if let Some(action) = modal.content_mut().take_action() {
                        self.handle_examples_action(action);
                    }
                }
                crate::commands::migration::ui::components::modal_component::ModalAction::Close => {
                    self.examples_modal = None;
                }
                _ => {}
            }
            return ScreenResult::Continue;
        }

        // Handle main screen events
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
                        // Go back to comparison select screen
                        ScreenResult::Navigate(Box::new(ComparisonSelectScreen::new(
                            self.config.clone(),
                            crate::config::SavedMigration {
                                name: format!(
                                    "{}_to_{}",
                                    self.comparison.source_entity, self.comparison.target_entity
                                ),
                                source_env: self
                                    .comparison_data
                                    .as_ref()
                                    .unwrap()
                                    .source_env
                                    .clone(),
                                target_env: self
                                    .comparison_data
                                    .as_ref()
                                    .unwrap()
                                    .target_env
                                    .clone(),
                                comparisons: vec![self.comparison.clone()],
                                created_at: String::new(),
                                last_used: String::new(),
                            },
                        )))
                    }
                    KeyCode::Tab => {
                        EventHandlers::switch_side(&mut self.focused_side);
                        ScreenResult::Continue
                    }
                    KeyCode::Char('1') | KeyCode::F(1) => {
                        EventHandlers::switch_tab(&mut self.active_tab, 1);
                        ScreenResult::Continue
                    }
                    KeyCode::Char('2') | KeyCode::F(2) => {
                        EventHandlers::switch_tab(&mut self.active_tab, 2);
                        ScreenResult::Continue
                    }
                    KeyCode::Char('3') | KeyCode::F(3) => {
                        EventHandlers::switch_tab(&mut self.active_tab, 3);
                        ScreenResult::Continue
                    }
                    KeyCode::Char('4') | KeyCode::F(4) => {
                        EventHandlers::switch_tab(&mut self.active_tab, 4);
                        ScreenResult::Continue
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        EventHandlers::handle_list_navigation(
                            true,
                            &self.active_tab,
                            &self.focused_side,
                            &mut *self.fields_app,
                            &mut *self.relationships_app,
                            &mut *self.views_app,
                            &mut *self.forms_app,
                        );
                        ScreenResult::Continue
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        EventHandlers::handle_list_navigation(
                            false,
                            &self.active_tab,
                            &self.focused_side,
                            &mut *self.fields_app,
                            &mut *self.relationships_app,
                            &mut *self.views_app,
                            &mut *self.forms_app,
                        );
                        ScreenResult::Continue
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        EventHandlers::handle_action_key(
                            &self.active_tab,
                            &self.focused_side,
                            &mut *self.fields_app,
                            &mut *self.relationships_app,
                            &mut *self.views_app,
                            &mut *self.forms_app,
                        );
                        ScreenResult::Continue
                    }
                    KeyCode::Char('h') => {
                        EventHandlers::toggle_hide_matched(
                            &mut self.shared_state,
                            &self.comparison_data,
                            &mut *self.fields_app,
                            &mut *self.relationships_app,
                            &mut *self.views_app,
                            &mut *self.forms_app,
                        );
                        ScreenResult::Continue
                    }
                    KeyCode::Char('s') => {
                        // Cycle between alphabetical and reverse alphabetical
                        self.sort_mode = match self.sort_mode {
                            SortMode::Alphabetical => SortMode::ReverseAlphabetical,
                            SortMode::ReverseAlphabetical => SortMode::Alphabetical,
                        };
                        self.apply_sorting_to_all_apps();
                        ScreenResult::Continue
                    }
                    KeyCode::Char('p') => {
                        self.open_prefix_modal();
                        ScreenResult::Continue
                    }
                    KeyCode::Char('M') => {
                        self.open_manual_modal();
                        ScreenResult::Continue
                    }
                    KeyCode::Char('m') => {
                        self.add_manual_match();
                        ScreenResult::Continue
                    }
                    KeyCode::Char('e') => {
                        self.toggle_examples_mode();
                        ScreenResult::Continue
                    }
                    KeyCode::Char('E') => {
                        self.open_examples_modal();
                        ScreenResult::Continue
                    }
                    _ => ScreenResult::Continue,
                }
            }
            Event::Mouse(mouse) => EventHandlers::handle_mouse_event(
                mouse,
                &mut self.active_tab,
                &mut self.focused_side,
                self.tab_area,
                self.source_area,
                self.target_area,
                &mut *self.fields_app,
                &mut *self.relationships_app,
                &mut *self.views_app,
                &mut *self.forms_app,
            ),
            _ => ScreenResult::Continue,
        }
    }

    fn get_footer_actions(&self) -> Vec<FooterAction> {
        vec![
            // Navigation & Core Actions (most used)
            FooterAction {
                key: "↑↓/jk".to_string(),
                description: "Nav".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "Enter".to_string(),
                description: "Expand".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "Tab".to_string(),
                description: "Switch Side".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "1-4".to_string(),
                description: "Tabs".to_string(),
                enabled: true,
            },
            // View Options
            FooterAction {
                key: "h".to_string(),
                description: "Hide Matched".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "s".to_string(),
                description: "Sort".to_string(),
                enabled: true,
            },
            // Mapping Actions (grouped together)
            FooterAction {
                key: "m".to_string(),
                description: "Add Match".to_string(),
                enabled: matches!(self.active_tab, ActiveTab::Fields),
            },
            FooterAction {
                key: "M".to_string(),
                description: "Manage Matches".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "p".to_string(),
                description: "Prefix Maps".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "e".to_string(),
                description: "Toggle Examples".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "E".to_string(),
                description: "Manage Examples".to_string(),
                enabled: true,
            },
            // System Actions (last)
            FooterAction {
                key: "Esc".to_string(),
                description: "← Back".to_string(),
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
        Some(format!(
            "Migration Comparison: {} → {}",
            self.comparison.source_entity, self.comparison.target_entity
        ))
    }
}
