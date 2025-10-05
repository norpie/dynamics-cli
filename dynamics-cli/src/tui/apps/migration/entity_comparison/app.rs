use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{Element, LayoutConstraint},
    subscription::Subscription,
    state::theme::Theme,
    renderer::LayeredView,
    Resource,
    widgets::TreeState,
    modals::ConfirmationModal,
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
use crate::{col, row, use_constraints};
use super::{Msg, Side, ExamplesState, ActiveTab, FetchType, fetch_with_cache, extract_relationships};
use super::tree_builder::build_tree_items;
use super::matching::{compute_field_matches, compute_relationship_matches};
use super::models::MatchInfo;

pub struct EntityComparisonApp;

#[derive(Clone, Default)]
pub struct State {
    // Context
    migration_name: String,
    source_env: String,
    target_env: String,
    source_entity: String,
    target_entity: String,

    // Active tab
    active_tab: ActiveTab,

    // Metadata (from API)
    source_metadata: Resource<EntityMetadata>,
    target_metadata: Resource<EntityMetadata>,

    // Mapping state
    field_mappings: HashMap<String, String>,  // source -> target (manual)
    prefix_mappings: HashMap<String, String>, // source_prefix -> target_prefix
    hide_matched: bool,

    // Computed matches (cached)
    field_matches: HashMap<String, MatchInfo>,        // source_field -> match_info
    relationship_matches: HashMap<String, MatchInfo>, // source_relationship -> match_info

    // Tree UI state - one tree state per tab per side
    source_fields_tree: TreeState,
    source_relationships_tree: TreeState,
    source_views_tree: TreeState,
    source_forms_tree: TreeState,
    target_fields_tree: TreeState,
    target_relationships_tree: TreeState,
    target_views_tree: TreeState,
    target_forms_tree: TreeState,
    focused_side: Side,

    // Examples
    examples: ExamplesState,

    // Modal state
    show_back_confirmation: bool,
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
    fn source_tree_for_tab(&mut self) -> &mut TreeState {
        match self.active_tab {
            ActiveTab::Fields => &mut self.source_fields_tree,
            ActiveTab::Relationships => &mut self.source_relationships_tree,
            ActiveTab::Views => &mut self.source_views_tree,
            ActiveTab::Forms => &mut self.source_forms_tree,
        }
    }

    /// Get the appropriate target tree state for the active tab
    fn target_tree_for_tab(&mut self) -> &mut TreeState {
        match self.active_tab {
            ActiveTab::Fields => &mut self.target_fields_tree,
            ActiveTab::Relationships => &mut self.target_relationships_tree,
            ActiveTab::Views => &mut self.target_views_tree,
            ActiveTab::Forms => &mut self.target_forms_tree,
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
            field_matches: HashMap::new(),
            relationship_matches: HashMap::new(),
            source_fields_tree: TreeState::with_selection(),
            source_relationships_tree: TreeState::with_selection(),
            source_views_tree: TreeState::with_selection(),
            source_forms_tree: TreeState::with_selection(),
            target_fields_tree: TreeState::with_selection(),
            target_relationships_tree: TreeState::with_selection(),
            target_views_tree: TreeState::with_selection(),
            target_forms_tree: TreeState::with_selection(),
            focused_side: Side::Source,
            examples: ExamplesState::new(),
            show_back_confirmation: false,
        };

        // Load metadata in parallel with automatic LoadingScreen (6 tasks total)
        let cmd = Command::perform_parallel()
            // Source entity fetches
            .add_task(
                format!("Loading {} fields ({})", params.source_entity, params.source_env),
                {
                    let env = params.source_env.clone();
                    let entity = params.source_entity.clone();
                    async move {
                        fetch_with_cache(&env, &entity, FetchType::SourceFields, true).await
                    }
                }
            )
            .add_task(
                format!("Loading {} forms ({})", params.source_entity, params.source_env),
                {
                    let env = params.source_env.clone();
                    let entity = params.source_entity.clone();
                    async move {
                        fetch_with_cache(&env, &entity, FetchType::SourceForms, true).await
                    }
                }
            )
            .add_task(
                format!("Loading {} views ({})", params.source_entity, params.source_env),
                {
                    let env = params.source_env.clone();
                    let entity = params.source_entity.clone();
                    async move {
                        fetch_with_cache(&env, &entity, FetchType::SourceViews, true).await
                    }
                }
            )
            // Target entity fetches
            .add_task(
                format!("Loading {} fields ({})", params.target_entity, params.target_env),
                {
                    let env = params.target_env.clone();
                    let entity = params.target_entity.clone();
                    async move {
                        fetch_with_cache(&env, &entity, FetchType::TargetFields, true).await
                    }
                }
            )
            .add_task(
                format!("Loading {} forms ({})", params.target_entity, params.target_env),
                {
                    let env = params.target_env.clone();
                    let entity = params.target_entity.clone();
                    async move {
                        fetch_with_cache(&env, &entity, FetchType::TargetForms, true).await
                    }
                }
            )
            .add_task(
                format!("Loading {} views ({})", params.target_entity, params.target_env),
                {
                    let env = params.target_env.clone();
                    let entity = params.target_entity.clone();
                    async move {
                        fetch_with_cache(&env, &entity, FetchType::TargetViews, true).await
                    }
                }
            )
            .with_title("Loading Entity Metadata")
            .on_complete(AppId::EntityComparison)
            .build(|task_idx, result| {
                let data = result.downcast::<Result<super::FetchedData, String>>().unwrap();
                Msg::ParallelDataLoaded(task_idx, *data)
            });

        (state, cmd)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        match msg {
            Msg::Back => {
                state.show_back_confirmation = true;
                Command::None
            }
            Msg::ConfirmBack => {
                Command::navigate_to(AppId::MigrationComparisonSelect)
            }
            Msg::CancelBack => {
                state.show_back_confirmation = false;
                Command::None
            }
            Msg::SwitchTab(n) => {
                if let Some(tab) = ActiveTab::from_number(n) {
                    state.active_tab = tab;
                }
                Command::None
            }
            Msg::ParallelDataLoaded(_task_idx, result) => {
                match result {
                    Ok(data) => {
                        // Update the appropriate metadata based on the data variant
                        match data {
                            super::FetchedData::SourceFields(fields) => {
                                if let Resource::Success(ref mut meta) = state.source_metadata {
                                    meta.fields = fields;
                                } else {
                                    state.source_metadata = Resource::Success(crate::api::EntityMetadata {
                                        fields,
                                        ..Default::default()
                                    });
                                }
                            }
                            super::FetchedData::SourceForms(forms) => {
                                if let Resource::Success(ref mut meta) = state.source_metadata {
                                    meta.forms = forms;
                                } else {
                                    state.source_metadata = Resource::Success(crate::api::EntityMetadata {
                                        forms,
                                        ..Default::default()
                                    });
                                }
                            }
                            super::FetchedData::SourceViews(views) => {
                                if let Resource::Success(ref mut meta) = state.source_metadata {
                                    meta.views = views;
                                } else {
                                    state.source_metadata = Resource::Success(crate::api::EntityMetadata {
                                        views,
                                        ..Default::default()
                                    });
                                }
                            }
                            super::FetchedData::TargetFields(fields) => {
                                if let Resource::Success(ref mut meta) = state.target_metadata {
                                    meta.fields = fields;
                                } else {
                                    state.target_metadata = Resource::Success(crate::api::EntityMetadata {
                                        fields,
                                        ..Default::default()
                                    });
                                }
                            }
                            super::FetchedData::TargetForms(forms) => {
                                if let Resource::Success(ref mut meta) = state.target_metadata {
                                    meta.forms = forms;
                                } else {
                                    state.target_metadata = Resource::Success(crate::api::EntityMetadata {
                                        forms,
                                        ..Default::default()
                                    });
                                }
                            }
                            super::FetchedData::TargetViews(views) => {
                                if let Resource::Success(ref mut meta) = state.target_metadata {
                                    meta.views = views;
                                } else {
                                    state.target_metadata = Resource::Success(crate::api::EntityMetadata {
                                        views,
                                        ..Default::default()
                                    });
                                }
                            }
                        }

                        // Extract relationships from fields after fields are loaded
                        if let Resource::Success(ref mut meta) = state.source_metadata {
                            if meta.relationships.is_empty() && !meta.fields.is_empty() {
                                meta.relationships = extract_relationships(&meta.fields);
                            }
                        }
                        if let Resource::Success(ref mut meta) = state.target_metadata {
                            if meta.relationships.is_empty() && !meta.fields.is_empty() {
                                meta.relationships = extract_relationships(&meta.fields);
                            }
                        }

                        // Write complete metadata to cache and focus tree when both fully loaded
                        if let (Resource::Success(source), Resource::Success(target)) =
                            (&state.source_metadata, &state.target_metadata)
                        {
                            if !source.fields.is_empty() && !target.fields.is_empty()
                                && !source.forms.is_empty() && !target.forms.is_empty()
                                && !source.views.is_empty() && !target.views.is_empty() {

                                // Compute field and relationship matches
                                state.field_matches = compute_field_matches(
                                    &source.fields,
                                    &target.fields,
                                    &state.field_mappings,
                                    &state.prefix_mappings,
                                );
                                state.relationship_matches = compute_relationship_matches(
                                    &source.relationships,
                                    &target.relationships,
                                    &state.field_mappings, // Reuse field mappings for relationships
                                    &state.prefix_mappings,
                                );

                                // Cache both metadata objects asynchronously
                                let source_env = state.source_env.clone();
                                let source_entity = state.source_entity.clone();
                                let source_meta = source.clone();
                                tokio::spawn(async move {
                                    let config = crate::global_config();
                                    if let Err(e) = config.set_entity_metadata_cache(&source_env, &source_entity, &source_meta).await {
                                        log::error!("Failed to cache source metadata for {}/{}: {}", source_env, source_entity, e);
                                    } else {
                                        log::debug!("Cached source metadata for {}/{}", source_env, source_entity);
                                    }
                                });

                                let target_env = state.target_env.clone();
                                let target_entity = state.target_entity.clone();
                                let target_meta = target.clone();
                                tokio::spawn(async move {
                                    let config = crate::global_config();
                                    if let Err(e) = config.set_entity_metadata_cache(&target_env, &target_entity, &target_meta).await {
                                        log::error!("Failed to cache target metadata for {}/{}: {}", target_env, target_entity, e);
                                    } else {
                                        log::debug!("Cached target metadata for {}/{}", target_env, target_entity);
                                    }
                                });

                                return Command::set_focus("source_tree".into());
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to load metadata: {}", e);
                        // Navigate to error screen
                        return Command::start_app(
                            AppId::ErrorScreen,
                            crate::tui::apps::screens::ErrorScreenParams {
                                message: format!("Failed to load entity metadata:\n\n{}", e),
                                target: Some(AppId::MigrationComparisonSelect),
                            }
                        );
                    }
                }

                Command::None
            }
            Msg::SourceTreeEvent(event) => {
                // Handle source tree navigation/interaction
                let tree_state = match state.active_tab {
                    ActiveTab::Fields => &mut state.source_fields_tree,
                    ActiveTab::Relationships => &mut state.source_relationships_tree,
                    ActiveTab::Views => &mut state.source_views_tree,
                    ActiveTab::Forms => &mut state.source_forms_tree,
                };
                tree_state.handle_event(event);
                Command::None
            }
            Msg::TargetTreeEvent(event) => {
                // Handle target tree navigation/interaction
                let tree_state = match state.active_tab {
                    ActiveTab::Fields => &mut state.target_fields_tree,
                    ActiveTab::Relationships => &mut state.target_relationships_tree,
                    ActiveTab::Views => &mut state.target_views_tree,
                    ActiveTab::Forms => &mut state.target_forms_tree,
                };
                tree_state.handle_event(event);
                Command::None
            }
            Msg::SourceViewportHeight(height) => {
                // Renderer calls this with actual viewport height
                let tree_state = match state.active_tab {
                    ActiveTab::Fields => &mut state.source_fields_tree,
                    ActiveTab::Relationships => &mut state.source_relationships_tree,
                    ActiveTab::Views => &mut state.source_views_tree,
                    ActiveTab::Forms => &mut state.source_forms_tree,
                };
                tree_state.set_viewport_height(height);
                Command::None
            }
            Msg::TargetViewportHeight(height) => {
                // Renderer calls this with actual viewport height
                let tree_state = match state.active_tab {
                    ActiveTab::Fields => &mut state.target_fields_tree,
                    ActiveTab::Relationships => &mut state.target_relationships_tree,
                    ActiveTab::Views => &mut state.target_views_tree,
                    ActiveTab::Forms => &mut state.target_forms_tree,
                };
                tree_state.set_viewport_height(height);
                Command::None
            }
            Msg::Refresh => {
                // Re-fetch metadata for both entities
                state.source_metadata = Resource::Loading;
                state.target_metadata = Resource::Loading;

                Command::perform_parallel()
                    // Source entity fetches - bypass cache for manual refresh
                    .add_task(
                        format!("Refreshing {} fields ({})", state.source_entity, state.source_env),
                        {
                            let env = state.source_env.clone();
                            let entity = state.source_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::SourceFields, false).await
                            }
                        }
                    )
                    .add_task(
                        format!("Refreshing {} forms ({})", state.source_entity, state.source_env),
                        {
                            let env = state.source_env.clone();
                            let entity = state.source_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::SourceForms, false).await
                            }
                        }
                    )
                    .add_task(
                        format!("Refreshing {} views ({})", state.source_entity, state.source_env),
                        {
                            let env = state.source_env.clone();
                            let entity = state.source_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::SourceViews, false).await
                            }
                        }
                    )
                    // Target entity fetches - bypass cache for manual refresh
                    .add_task(
                        format!("Refreshing {} fields ({})", state.target_entity, state.target_env),
                        {
                            let env = state.target_env.clone();
                            let entity = state.target_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::TargetFields, false).await
                            }
                        }
                    )
                    .add_task(
                        format!("Refreshing {} forms ({})", state.target_entity, state.target_env),
                        {
                            let env = state.target_env.clone();
                            let entity = state.target_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::TargetForms, false).await
                            }
                        }
                    )
                    .add_task(
                        format!("Refreshing {} views ({})", state.target_entity, state.target_env),
                        {
                            let env = state.target_env.clone();
                            let entity = state.target_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::TargetViews, false).await
                            }
                        }
                    )
                    .with_title("Refreshing Entity Metadata")
                    .on_complete(AppId::EntityComparison)
                    .build(|task_idx, result| {
                        let data = result.downcast::<Result<super::FetchedData, String>>().unwrap();
                        Msg::ParallelDataLoaded(task_idx, *data)
                    })
            }
            Msg::CreateManualMapping => {
                // Get selected items from both source and target trees
                let source_id = state.source_tree_for_tab().selected().map(|s| s.to_string());
                let target_id = state.target_tree_for_tab().selected().map(|s| s.to_string());

                if let (Some(source_id), Some(target_id)) = (source_id, target_id) {
                    // Extract the field/relationship name from the ID
                    // For now, IDs are the logical names for fields and relationships
                    // For containers, we skip manual mapping

                    // Only create mappings for fields and relationships
                    if !source_id.starts_with("view_") && !source_id.starts_with("form_")
                        && !source_id.starts_with("viewtype_") && !source_id.starts_with("formtype_")
                        && !source_id.starts_with("tab_") && !source_id.starts_with("section_")
                        && !source_id.starts_with("rel_") {
                        // It's a field - add to field_mappings
                        state.field_mappings.insert(source_id, target_id);

                        // Recompute matches
                        recompute_matches(state);
                    } else if source_id.starts_with("rel_") && target_id.starts_with("rel_") {
                        // It's a relationship - extract name and add to field_mappings
                        // (we reuse field_mappings for relationships)
                        let source_name = source_id.strip_prefix("rel_").unwrap_or(&source_id);
                        let target_name = target_id.strip_prefix("rel_").unwrap_or(&target_id);
                        state.field_mappings.insert(source_name.to_string(), target_name.to_string());

                        // Recompute matches
                        recompute_matches(state);
                    }
                }
                Command::None
            }
            Msg::DeleteManualMapping => {
                // Get selected item from source tree
                let source_id = state.source_tree_for_tab().selected().map(|s| s.to_string());

                if let Some(source_id) = source_id {
                    // Try to remove from field_mappings
                    let removed = if source_id.starts_with("rel_") {
                        let source_name = source_id.strip_prefix("rel_").unwrap_or(&source_id);
                        state.field_mappings.remove(source_name).is_some()
                    } else if !source_id.starts_with("view_") && !source_id.starts_with("form_")
                        && !source_id.starts_with("viewtype_") && !source_id.starts_with("formtype_")
                        && !source_id.starts_with("tab_") && !source_id.starts_with("section_") {
                        state.field_mappings.remove(&source_id).is_some()
                    } else {
                        false
                    };

                    if removed {
                        // Recompute matches
                        recompute_matches(state);
                    }
                }
                Command::None
            }
        }
    }

    fn view(state: &mut Self::State, theme: &Theme) -> LayeredView<Self::Msg> {
        let main_ui = render_main_layout(state, theme);
        let mut view = LayeredView::new(main_ui);

        if state.show_back_confirmation {
            view = view.with_app_modal(render_back_confirmation_modal(theme), LayerAlignment::Center);
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

            // Refresh metadata
            Subscription::keyboard(KeyCode::F(5), "Refresh metadata", Msg::Refresh),

            // Manual mapping actions
            Subscription::keyboard(KeyCode::Char('m'), "Create manual mapping", Msg::CreateManualMapping),
            Subscription::keyboard(KeyCode::Char('d'), "Delete manual mapping", Msg::DeleteManualMapping),
        ];

        // When showing confirmation modal, add y/n hotkeys
        if state.show_back_confirmation {
            subs.push(Subscription::keyboard(KeyCode::Char('y'), "Confirm", Msg::ConfirmBack));
            subs.push(Subscription::keyboard(KeyCode::Char('Y'), "Confirm", Msg::ConfirmBack));
            subs.push(Subscription::keyboard(KeyCode::Char('n'), "Cancel", Msg::CancelBack));
            subs.push(Subscription::keyboard(KeyCode::Char('N'), "Cancel", Msg::CancelBack));
            subs.push(Subscription::keyboard(KeyCode::Enter, "Confirm", Msg::ConfirmBack));
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

        Some(Line::from(spans))
    }
}

/// Render the main side-by-side layout with source and target trees
fn render_main_layout(state: &mut State, theme: &Theme) -> Element<Msg> {
    use_constraints!();

    // Build tree items for the active tab from metadata
    let active_tab = state.active_tab;
    let source_items = if let Resource::Success(ref metadata) = state.source_metadata {
        build_tree_items(metadata, active_tab, &state.field_matches, &state.relationship_matches)
    } else {
        vec![]
    };

    let target_items = if let Resource::Success(ref metadata) = state.target_metadata {
        // Target tree doesn't need matches (or could use reverse matches in the future)
        let empty_field_matches = HashMap::new();
        let empty_relationship_matches = HashMap::new();
        build_tree_items(metadata, active_tab, &empty_field_matches, &empty_relationship_matches)
    } else {
        vec![]
    };

    // Cache entity names before borrowing tree states
    let source_entity_name = state.source_entity.clone();
    let target_entity_name = state.target_entity.clone();

    // Get the appropriate tree state for the active tab based on which side
    let (source_tree_state, target_tree_state) = match active_tab {
        ActiveTab::Fields => (&mut state.source_fields_tree, &mut state.target_fields_tree),
        ActiveTab::Relationships => (&mut state.source_relationships_tree, &mut state.target_relationships_tree),
        ActiveTab::Views => (&mut state.source_views_tree, &mut state.target_views_tree),
        ActiveTab::Forms => (&mut state.source_forms_tree, &mut state.target_forms_tree),
    };

    // Source panel with tree - renderer will call on_render with actual area.height
    let source_panel = Element::panel(
        Element::tree("source_tree", &source_items, source_tree_state, theme)
            .on_event(Msg::SourceTreeEvent)
            .on_render(Msg::SourceViewportHeight)
            .build()
    )
    .title(format!("Source: {}", source_entity_name))
    .build();

    // Target panel with tree - renderer will call on_render with actual area.height
    let target_panel = Element::panel(
        Element::tree("target_tree", &target_items, target_tree_state, theme)
            .on_event(Msg::TargetTreeEvent)
            .on_render(Msg::TargetViewportHeight)
            .build()
    )
    .title(format!("Target: {}", target_entity_name))
    .build();

    // Side-by-side layout
    row![
        source_panel => Fill(1),
        target_panel => Fill(1),
    ]
}

/// Render the back confirmation modal
fn render_back_confirmation_modal(theme: &Theme) -> Element<Msg> {
    ConfirmationModal::new("Go Back?")
        .message("Are you sure you want to go back to the comparison list?")
        .confirm_text("Yes")
        .cancel_text("No")
        .on_confirm(Msg::ConfirmBack)
        .on_cancel(Msg::CancelBack)
        .width(60)
        .height(10)
        .build(theme)
}

/// Recompute field and relationship matches based on current mappings
fn recompute_matches(state: &mut State) {
    if let (Resource::Success(source), Resource::Success(target)) =
        (&state.source_metadata, &state.target_metadata)
    {
        state.field_matches = compute_field_matches(
            &source.fields,
            &target.fields,
            &state.field_mappings,
            &state.prefix_mappings,
        );
        state.relationship_matches = compute_relationship_matches(
            &source.relationships,
            &target.relationships,
            &state.field_mappings,
            &state.prefix_mappings,
        );
    }
}
