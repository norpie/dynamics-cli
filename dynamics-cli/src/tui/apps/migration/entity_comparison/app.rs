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
use super::{Msg, Side, ExamplesState, ActiveTab, FetchType, fetch_with_cache, extract_relationships, extract_entities};
use super::tree_builder::build_tree_items;
use super::matching::{compute_field_matches, compute_relationship_matches, compute_hierarchical_field_matches, compute_entity_matches};
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
    entity_matches: HashMap<String, MatchInfo>,       // source_entity -> match_info

    // Entity lists (extracted from relationships)
    source_entities: Vec<(String, usize)>,  // (entity_name, usage_count)
    target_entities: Vec<(String, usize)>,

    // Tree UI state - one tree state per tab per side
    source_fields_tree: TreeState,
    source_relationships_tree: TreeState,
    source_views_tree: TreeState,
    source_forms_tree: TreeState,
    source_entities_tree: TreeState,
    target_fields_tree: TreeState,
    target_relationships_tree: TreeState,
    target_views_tree: TreeState,
    target_forms_tree: TreeState,
    target_entities_tree: TreeState,
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
            ActiveTab::Entities => &mut self.source_entities_tree,
        }
    }

    /// Get the appropriate target tree state for the active tab
    fn target_tree_for_tab(&mut self) -> &mut TreeState {
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

        // Also load saved field and prefix mappings
        let mappings_cmd = Command::perform({
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
                (field_mappings, prefix_mappings)
            }
        }, |(field_mappings, prefix_mappings)| Msg::MappingsLoaded(field_mappings, prefix_mappings));

        (state, Command::batch(vec![cmd, mappings_cmd]))
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
                                // Use the same logic as recompute_matches
                                let mut all_field_matches = compute_field_matches(
                                    &source.fields,
                                    &target.fields,
                                    &state.field_mappings,
                                    &state.prefix_mappings,
                                );

                                // Hierarchical matching for Forms tab
                                let forms_matches = compute_hierarchical_field_matches(
                                    source,
                                    target,
                                    &state.field_mappings,
                                    &state.prefix_mappings,
                                    "forms",
                                );
                                all_field_matches.extend(forms_matches);

                                // Hierarchical matching for Views tab
                                let views_matches = compute_hierarchical_field_matches(
                                    source,
                                    target,
                                    &state.field_mappings,
                                    &state.prefix_mappings,
                                    "views",
                                );
                                all_field_matches.extend(views_matches);

                                state.field_matches = all_field_matches;

                                // Extract entities from relationships
                                state.source_entities = extract_entities(&source.relationships);
                                state.target_entities = extract_entities(&target.relationships);

                                // Compute entity matches (uses same mappings as fields)
                                state.entity_matches = compute_entity_matches(
                                    &state.source_entities,
                                    &state.target_entities,
                                    &state.field_mappings,  // Reuse field mappings for entities
                                    &state.prefix_mappings,
                                );

                                // Relationship matching (now entity-aware)
                                state.relationship_matches = compute_relationship_matches(
                                    &source.relationships,
                                    &target.relationships,
                                    &state.field_mappings, // Reuse field mappings for relationships
                                    &state.prefix_mappings,
                                    &state.entity_matches,  // Pass entity matches for entity-aware matching
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
                    ActiveTab::Entities => &mut state.source_entities_tree,
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
                    ActiveTab::Entities => &mut state.target_entities_tree,
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
                    ActiveTab::Entities => &mut state.source_entities_tree,
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
                    ActiveTab::Entities => &mut state.target_entities_tree,
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
                    // Handle different ID formats based on tab type
                    let (source_key, target_key) = match state.active_tab {
                        ActiveTab::Fields => {
                            // Fields tab: IDs are simple field names
                            (source_id.clone(), target_id.clone())
                        }
                        ActiveTab::Relationships => {
                            // Relationships tab: IDs have "rel_" prefix
                            let source_name = source_id.strip_prefix("rel_").unwrap_or(&source_id).to_string();
                            let target_name = target_id.strip_prefix("rel_").unwrap_or(&target_id).to_string();
                            (source_name, target_name)
                        }
                        ActiveTab::Entities => {
                            // Entities tab: IDs have "entity_" prefix
                            let source_name = source_id.strip_prefix("entity_").unwrap_or(&source_id).to_string();
                            let target_name = target_id.strip_prefix("entity_").unwrap_or(&target_id).to_string();
                            (source_name, target_name)
                        }
                        ActiveTab::Forms | ActiveTab::Views => {
                            // Forms/Views tabs: IDs are paths, support both fields and containers
                            (source_id.clone(), target_id.clone())
                        }
                    };

                    // Add to state mappings
                    state.field_mappings.insert(source_key.clone(), target_key.clone());

                    // Recompute matches
                    recompute_matches(state);

                    // Save to database
                    let source_entity = state.source_entity.clone();
                    let target_entity = state.target_entity.clone();
                    tokio::spawn(async move {
                        let config = crate::global_config();
                        if let Err(e) = config.set_field_mapping(&source_entity, &target_entity, &source_key, &target_key).await {
                            log::error!("Failed to save field mapping: {}", e);
                        }
                    });
                }
                Command::None
            }
            Msg::DeleteManualMapping => {
                // Get selected item from source tree
                let source_id = state.source_tree_for_tab().selected().map(|s| s.to_string());

                if let Some(source_id) = source_id {
                    // Extract the key based on tab type (same logic as CreateManualMapping)
                    let source_key = match state.active_tab {
                        ActiveTab::Fields => source_id.clone(),
                        ActiveTab::Relationships => {
                            source_id.strip_prefix("rel_").unwrap_or(&source_id).to_string()
                        }
                        ActiveTab::Entities => {
                            source_id.strip_prefix("entity_").unwrap_or(&source_id).to_string()
                        }
                        ActiveTab::Forms | ActiveTab::Views => source_id.clone(),
                    };

                    // Try to remove from field_mappings
                    if state.field_mappings.remove(&source_key).is_some() {
                        // Recompute matches
                        recompute_matches(state);

                        // Delete from database
                        let source_entity = state.source_entity.clone();
                        let target_entity = state.target_entity.clone();
                        tokio::spawn(async move {
                            let config = crate::global_config();
                            if let Err(e) = config.delete_field_mapping(&source_entity, &target_entity, &source_key).await {
                                log::error!("Failed to delete field mapping: {}", e);
                            }
                        });
                    }
                }
                Command::None
            }
            Msg::MappingsLoaded(field_mappings, prefix_mappings) => {
                // Update state with loaded mappings
                state.field_mappings = field_mappings;
                state.prefix_mappings = prefix_mappings;

                // Recompute matches if we have metadata loaded
                if matches!(state.source_metadata, Resource::Success(_))
                    && matches!(state.target_metadata, Resource::Success(_)) {
                    recompute_matches(state);
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
            Subscription::keyboard(KeyCode::Char('5'), "Switch to Entities", Msg::SwitchTab(5)),

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

        Some(Line::from(spans))
    }
}

/// Render the main side-by-side layout with source and target trees
fn render_main_layout(state: &mut State, theme: &Theme) -> Element<Msg> {
    use_constraints!();

    // Build tree items for the active tab from metadata
    let active_tab = state.active_tab;
    let source_items = if let Resource::Success(ref metadata) = state.source_metadata {
        build_tree_items(
            metadata,
            active_tab,
            &state.field_matches,
            &state.relationship_matches,
            &state.entity_matches,
            &state.source_entities,
        )
    } else {
        vec![]
    };

    let target_items = if let Resource::Success(ref metadata) = state.target_metadata {
        // Create reverse matches for target side (target_field -> source_field)
        let reverse_field_matches: HashMap<String, MatchInfo> = state.field_matches.iter()
            .map(|(source_field, match_info)| {
                (match_info.target_field.clone(), MatchInfo {
                    target_field: source_field.clone(),  // Points back to source
                    match_type: match_info.match_type,
                    confidence: match_info.confidence,
                })
            })
            .collect();

        let reverse_relationship_matches: HashMap<String, MatchInfo> = state.relationship_matches.iter()
            .map(|(source_rel, match_info)| {
                (match_info.target_field.clone(), MatchInfo {
                    target_field: source_rel.clone(),  // Points back to source
                    match_type: match_info.match_type,
                    confidence: match_info.confidence,
                })
            })
            .collect();

        let reverse_entity_matches: HashMap<String, MatchInfo> = state.entity_matches.iter()
            .map(|(source_entity, match_info)| {
                (match_info.target_field.clone(), MatchInfo {
                    target_field: source_entity.clone(),  // Points back to source
                    match_type: match_info.match_type,
                    confidence: match_info.confidence,
                })
            })
            .collect();

        build_tree_items(
            metadata,
            active_tab,
            &reverse_field_matches,
            &reverse_relationship_matches,
            &reverse_entity_matches,
            &state.target_entities,
        )
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
        ActiveTab::Entities => (&mut state.source_entities_tree, &mut state.target_entities_tree),
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
        // Flat matching for Fields tab
        let mut all_field_matches = compute_field_matches(
            &source.fields,
            &target.fields,
            &state.field_mappings,
            &state.prefix_mappings,
        );

        // Hierarchical matching for Forms tab
        let forms_matches = compute_hierarchical_field_matches(
            source,
            target,
            &state.field_mappings,
            &state.prefix_mappings,
            "forms",
        );
        all_field_matches.extend(forms_matches);

        // Hierarchical matching for Views tab
        let views_matches = compute_hierarchical_field_matches(
            source,
            target,
            &state.field_mappings,
            &state.prefix_mappings,
            "views",
        );
        all_field_matches.extend(views_matches);

        state.field_matches = all_field_matches;

        // Extract entities from relationships
        state.source_entities = extract_entities(&source.relationships);
        state.target_entities = extract_entities(&target.relationships);

        // Compute entity matches (uses same mappings as fields)
        state.entity_matches = compute_entity_matches(
            &state.source_entities,
            &state.target_entities,
            &state.field_mappings,  // Reuse field mappings for entities
            &state.prefix_mappings,
        );

        // Relationship matching (now entity-aware)
        state.relationship_matches = compute_relationship_matches(
            &source.relationships,
            &target.relationships,
            &state.field_mappings,
            &state.prefix_mappings,
            &state.entity_matches,  // Pass entity matches for entity-aware matching
        );
    }
}
