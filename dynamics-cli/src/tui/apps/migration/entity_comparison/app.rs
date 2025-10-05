use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{Element, LayoutConstraint},
    subscription::Subscription,
    state::theme::Theme,
    renderer::LayeredView,
    Resource,
    widgets::TreeState,
    Alignment as LayerAlignment,
};
use crate::api::EntityMetadata;
use crossterm::event::KeyCode;
use ratatui::{
    prelude::Stylize,
    style::Style,
    text::{Line, Span},
};
use std::collections::HashMap;
use super::{Msg, Side, ExamplesState, ExamplePair, ActiveTab, FetchType, fetch_with_cache, extract_relationships, extract_entities, MatchInfo};
use super::matching::recompute_all_matches;
use super::tree_sync::{update_mirrored_selection, mirror_container_toggle};
use super::view::{render_main_layout, render_back_confirmation_modal, render_examples_modal};

pub struct EntityComparisonApp;

#[derive(Clone, Default)]
pub struct State {
    // Context
    pub(super) migration_name: String,
    pub(super) source_env: String,
    pub(super) target_env: String,
    pub(super) source_entity: String,
    pub(super) target_entity: String,

    // Active tab
    pub(super) active_tab: ActiveTab,

    // Metadata (from API)
    pub(super) source_metadata: Resource<EntityMetadata>,
    pub(super) target_metadata: Resource<EntityMetadata>,

    // Mapping state
    pub(super) field_mappings: HashMap<String, String>,  // source -> target (manual)
    pub(super) prefix_mappings: HashMap<String, String>, // source_prefix -> target_prefix
    pub(super) hide_matched: bool,
    pub(super) sort_mode: super::models::SortMode,
    pub(super) show_technical_names: bool, // true = logical names, false = display names

    // Computed matches (cached)
    pub(super) field_matches: HashMap<String, MatchInfo>,        // source_field -> match_info
    pub(super) relationship_matches: HashMap<String, MatchInfo>, // source_relationship -> match_info
    pub(super) entity_matches: HashMap<String, MatchInfo>,       // source_entity -> match_info

    // Entity lists (extracted from relationships)
    pub(super) source_entities: Vec<(String, usize)>,  // (entity_name, usage_count)
    pub(super) target_entities: Vec<(String, usize)>,

    // Tree UI state - one tree state per tab per side
    pub(super) source_fields_tree: TreeState,
    pub(super) source_relationships_tree: TreeState,
    pub(super) source_views_tree: TreeState,
    pub(super) source_forms_tree: TreeState,
    pub(super) source_entities_tree: TreeState,
    pub(super) target_fields_tree: TreeState,
    pub(super) target_relationships_tree: TreeState,
    pub(super) target_views_tree: TreeState,
    pub(super) target_forms_tree: TreeState,
    pub(super) target_entities_tree: TreeState,
    pub(super) focused_side: Side,

    // Examples
    pub(super) examples: ExamplesState,

    // Examples modal state
    pub(super) show_examples_modal: bool,
    pub(super) examples_list_state: crate::tui::widgets::ListState,
    pub(super) examples_source_input: crate::tui::widgets::TextInputField,
    pub(super) examples_target_input: crate::tui::widgets::TextInputField,
    pub(super) examples_label_input: crate::tui::widgets::TextInputField,

    // Prefix mappings modal state
    pub(super) show_prefix_mappings_modal: bool,
    pub(super) prefix_mappings_list_state: crate::tui::widgets::ListState,
    pub(super) prefix_source_input: crate::tui::widgets::TextInputField,
    pub(super) prefix_target_input: crate::tui::widgets::TextInputField,

    // Modal state
    pub(super) show_back_confirmation: bool,
}

pub struct EntityComparisonParams {
    pub migration_name: String,
    pub source_env: String,
    pub target_env: String,
    pub source_entity: String,
    pub target_entity: String,
}

impl Default for EntityComparisonParams {
    fn default() -> Self {
        Self {
            migration_name: String::new(),
            source_env: String::new(),
            target_env: String::new(),
            source_entity: String::new(),
            target_entity: String::new(),
        }
    }
}

impl crate::tui::AppState for State {}

impl State {
    /// Get the appropriate source tree state for the active tab
    pub(super) fn source_tree_for_tab(&mut self) -> &mut TreeState {
        match self.active_tab {
            ActiveTab::Fields => &mut self.source_fields_tree,
            ActiveTab::Relationships => &mut self.source_relationships_tree,
            ActiveTab::Views => &mut self.source_views_tree,
            ActiveTab::Forms => &mut self.source_forms_tree,
            ActiveTab::Entities => &mut self.source_entities_tree,
        }
    }

    /// Get the appropriate target tree state for the active tab
    pub(super) fn target_tree_for_tab(&mut self) -> &mut TreeState {
        match self.active_tab {
            ActiveTab::Fields => &mut self.target_fields_tree,
            ActiveTab::Relationships => &mut self.target_relationships_tree,
            ActiveTab::Views => &mut self.target_views_tree,
            ActiveTab::Forms => &mut self.target_forms_tree,
            ActiveTab::Entities => &mut self.target_entities_tree,
        }
    }
}

impl App for EntityComparisonApp {
    type State = State;
    type Msg = Msg;
    type InitParams = EntityComparisonParams;

    fn init(params: EntityComparisonParams) -> (State, Command<Msg>) {
        let mut state = State {
            migration_name: params.migration_name.clone(),
            source_env: params.source_env.clone(),
            target_env: params.target_env.clone(),
            source_entity: params.source_entity.clone(),
            target_entity: params.target_entity.clone(),
            active_tab: ActiveTab::default(),
            source_metadata: Resource::Loading,
            target_metadata: Resource::Loading,
            field_mappings: HashMap::new(),
            prefix_mappings: HashMap::new(),
            hide_matched: false,
            sort_mode: super::models::SortMode::default(),
            show_technical_names: true, // Default to showing logical/technical names
            field_matches: HashMap::new(),
            relationship_matches: HashMap::new(),
            entity_matches: HashMap::new(),
            source_entities: Vec::new(),
            target_entities: Vec::new(),
            source_fields_tree: TreeState::with_selection(),
            source_relationships_tree: TreeState::with_selection(),
            source_views_tree: TreeState::with_selection(),
            source_forms_tree: TreeState::with_selection(),
            source_entities_tree: TreeState::with_selection(),
            target_fields_tree: TreeState::with_selection(),
            target_relationships_tree: TreeState::with_selection(),
            target_views_tree: TreeState::with_selection(),
            target_forms_tree: TreeState::with_selection(),
            target_entities_tree: TreeState::with_selection(),
            focused_side: Side::Source,
            examples: ExamplesState::new(),
            show_examples_modal: false,
            examples_list_state: crate::tui::widgets::ListState::new(),
            examples_source_input: crate::tui::widgets::TextInputField::new(),
            examples_target_input: crate::tui::widgets::TextInputField::new(),
            examples_label_input: crate::tui::widgets::TextInputField::new(),
            show_prefix_mappings_modal: false,
            prefix_mappings_list_state: crate::tui::widgets::ListState::new(),
            prefix_source_input: crate::tui::widgets::TextInputField::new(),
            prefix_target_input: crate::tui::widgets::TextInputField::new(),
            show_back_confirmation: false,
        };

        // First, load mappings to know which example pairs to fetch
        let init_cmd = Command::perform({
            let source_entity = params.source_entity.clone();
            let target_entity = params.target_entity.clone();
            async move {
                let config = crate::global_config();
                let field_mappings = config.get_field_mappings(&source_entity, &target_entity).await
                    .unwrap_or_else(|e| {
                        log::error!("Failed to load field mappings: {}", e);
                        HashMap::new()
                    });
                let prefix_mappings = config.get_prefix_mappings(&source_entity, &target_entity).await
                    .unwrap_or_else(|e| {
                        log::error!("Failed to load prefix mappings: {}", e);
                        HashMap::new()
                    });
                let example_pairs = config.get_example_pairs(&source_entity, &target_entity).await
                    .unwrap_or_else(|e| {
                        log::error!("Failed to load example pairs: {}", e);
                        Vec::new()
                    });
                (field_mappings, prefix_mappings, example_pairs)
            }
        }, |(field_mappings, prefix_mappings, example_pairs)| {
            Msg::MappingsLoaded(field_mappings, prefix_mappings, example_pairs)
        });

        (state, init_cmd)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        super::update::update(state, msg)
    }

    fn view(state: &mut Self::State, theme: &Theme) -> LayeredView<Self::Msg> {
        let main_ui = render_main_layout(state, theme);
        let mut view = LayeredView::new(main_ui);

        if state.show_back_confirmation {
            view = view.with_app_modal(render_back_confirmation_modal(theme), LayerAlignment::Center);
        }

        if state.show_examples_modal {
            view = view.with_app_modal(render_examples_modal(state, theme), LayerAlignment::Center);
        }

        if state.show_prefix_mappings_modal {
            view = view.with_app_modal(super::view::render_prefix_mappings_modal(state, theme), LayerAlignment::Center);
        }

        view
    }

    fn subscriptions(state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        let mut subs = vec![
            Subscription::keyboard(KeyCode::Esc, "Back to comparison list", Msg::Back),
            Subscription::keyboard(KeyCode::Char('b'), "Back to comparison list", Msg::Back),
            Subscription::keyboard(KeyCode::Char('B'), "Back to comparison list", Msg::Back),

            // Tab switching
            Subscription::keyboard(KeyCode::Char('1'), "Switch to Fields", Msg::SwitchTab(1)),
            Subscription::keyboard(KeyCode::Char('2'), "Switch to Relationships", Msg::SwitchTab(2)),
            Subscription::keyboard(KeyCode::Char('3'), "Switch to Views", Msg::SwitchTab(3)),
            Subscription::keyboard(KeyCode::Char('4'), "Switch to Forms", Msg::SwitchTab(4)),
            Subscription::keyboard(KeyCode::Char('5'), "Switch to Entities", Msg::SwitchTab(5)),

            // AZERTY keyboard aliases
            Subscription::keyboard(KeyCode::Char('&'), "Switch to Fields", Msg::SwitchTab(1)),
            Subscription::keyboard(KeyCode::Char('é'), "Switch to Relationships", Msg::SwitchTab(2)),
            Subscription::keyboard(KeyCode::Char('"'), "Switch to Views", Msg::SwitchTab(3)),
            Subscription::keyboard(KeyCode::Char('\''), "Switch to Forms", Msg::SwitchTab(4)),
            Subscription::keyboard(KeyCode::Char('('), "Switch to Entities", Msg::SwitchTab(5)),

            // Refresh metadata
            Subscription::keyboard(KeyCode::F(5), "Refresh metadata", Msg::Refresh),

            // Manual mapping actions
            Subscription::keyboard(KeyCode::Char('m'), "Create manual mapping", Msg::CreateManualMapping),
            Subscription::keyboard(KeyCode::Char('d'), "Delete manual mapping", Msg::DeleteManualMapping),

            // Hide matched toggle
            Subscription::keyboard(KeyCode::Char('h'), "Toggle hide matched", Msg::ToggleHideMatched),
            Subscription::keyboard(KeyCode::Char('H'), "Toggle hide matched", Msg::ToggleHideMatched),

            // Sort mode toggle
            Subscription::keyboard(KeyCode::Char('s'), "Toggle sort mode", Msg::ToggleSortMode),
            Subscription::keyboard(KeyCode::Char('S'), "Toggle sort mode", Msg::ToggleSortMode),

            // Technical/display name toggle
            Subscription::keyboard(KeyCode::Char('t'), "Toggle technical names", Msg::ToggleTechnicalNames),
            Subscription::keyboard(KeyCode::Char('T'), "Toggle technical names", Msg::ToggleTechnicalNames),

            // Examples management
            Subscription::keyboard(KeyCode::Char('e'), "Cycle example pairs", Msg::CycleExamplePair),
            Subscription::keyboard(KeyCode::Char('x'), "Open examples modal", Msg::OpenExamplesModal),
            Subscription::keyboard(KeyCode::Char('X'), "Open examples modal", Msg::OpenExamplesModal),

            // Prefix mappings
            Subscription::keyboard(KeyCode::Char('p'), "Open prefix mappings modal", Msg::OpenPrefixMappingsModal),
            Subscription::keyboard(KeyCode::Char('P'), "Open prefix mappings modal", Msg::OpenPrefixMappingsModal),

            // Export
            Subscription::keyboard(KeyCode::F(10), "Export to Excel", Msg::ExportToExcel),
        ];

        // When showing confirmation modal, add y/n hotkeys
        if state.show_back_confirmation {
            subs.push(Subscription::keyboard(KeyCode::Char('y'), "Confirm", Msg::ConfirmBack));
            subs.push(Subscription::keyboard(KeyCode::Char('Y'), "Confirm", Msg::ConfirmBack));
            subs.push(Subscription::keyboard(KeyCode::Char('n'), "Cancel", Msg::CancelBack));
            subs.push(Subscription::keyboard(KeyCode::Char('N'), "Cancel", Msg::CancelBack));
            subs.push(Subscription::keyboard(KeyCode::Enter, "Confirm", Msg::ConfirmBack));
        }

        // When showing examples modal, add hotkeys
        if state.show_examples_modal {
            subs.push(Subscription::keyboard(KeyCode::Char('a'), "Add example pair", Msg::AddExamplePair));
            subs.push(Subscription::keyboard(KeyCode::Char('d'), "Delete example pair", Msg::DeleteExamplePair));
            subs.push(Subscription::keyboard(KeyCode::Char('c'), "Close modal", Msg::CloseExamplesModal));
            subs.push(Subscription::keyboard(KeyCode::Esc, "Close modal", Msg::CloseExamplesModal));
        }

        // When showing prefix mappings modal, add hotkeys
        if state.show_prefix_mappings_modal {
            subs.push(Subscription::keyboard(KeyCode::Char('a'), "Add prefix mapping", Msg::AddPrefixMapping));
            subs.push(Subscription::keyboard(KeyCode::Char('d'), "Delete prefix mapping", Msg::DeletePrefixMapping));
            subs.push(Subscription::keyboard(KeyCode::Char('c'), "Close modal", Msg::ClosePrefixMappingsModal));
            subs.push(Subscription::keyboard(KeyCode::Esc, "Close modal", Msg::ClosePrefixMappingsModal));
        }

        subs
    }

    fn title() -> &'static str {
        "Entity Comparison"
    }

    fn status(state: &Self::State, theme: &Theme) -> Option<Line<'static>> {
        // Build tab indicator with active tab highlighted
        let tabs = [
            ActiveTab::Fields,
            ActiveTab::Relationships,
            ActiveTab::Views,
            ActiveTab::Forms,
            ActiveTab::Entities,
        ];

        let mut spans = vec![];

        for (i, tab) in tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" ", Style::default()));
            }

            let is_active = *tab == state.active_tab;
            let label = format!("[{}] {}", tab.number(), tab.label());

            spans.push(Span::styled(
                label,
                if is_active {
                    Style::default().fg(theme.lavender).italic()
                } else {
                    Style::default().fg(theme.subtext1)
                },
            ));
        }

        // Add separator
        spans.push(Span::styled(" | ", Style::default().fg(theme.overlay1)));

        // Hide matched status
        if state.hide_matched {
            spans.push(Span::styled(
                "Hide Matched: ON",
                Style::default().fg(theme.green),
            ));
        } else {
            spans.push(Span::styled(
                "Hide Matched: OFF",
                Style::default().fg(theme.subtext1),
            ));
        }

        // Sort mode
        spans.push(Span::styled(" | ", Style::default().fg(theme.overlay1)));
        spans.push(Span::styled(
            format!("Sort: {}", state.sort_mode.label()),
            Style::default().fg(theme.subtext1),
        ));

        // Technical/display names indicator
        spans.push(Span::styled(" | ", Style::default().fg(theme.overlay1)));
        spans.push(Span::styled(
            if state.show_technical_names { "Names: Technical" } else { "Names: Display" },
            Style::default().fg(theme.subtext1),
        ));

        // Example display status
        if state.examples.enabled {
            if let Some(active_pair_id) = &state.examples.active_pair_id {
                // Find the index of the active pair
                if let Some(index) = state.examples.pairs.iter().position(|p| &p.id == active_pair_id) {
                    let pair_num = index + 1;
                    let total = state.examples.pairs.len();
                    spans.push(Span::styled(" | ", Style::default().fg(theme.overlay1)));
                    spans.push(Span::styled(
                        format!("Example: {}/{}", pair_num, total),
                        Style::default().fg(theme.sky),
                    ));
                }
            }
        }

        Some(Line::from(spans))
    }
}
