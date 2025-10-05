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
                            super::FetchedData::ExampleData(pair_id, source_data, target_data) => {
                                // Store example data in cache
                                if let Some(pair) = state.examples.pairs.iter().find(|p| p.id == pair_id) {
                                    log::info!("Fetched example data for pair {}: source_id={}, target_id={}",
                                        pair_id, pair.source_record_id, pair.target_record_id);
                                    state.examples.cache.insert(pair.source_record_id.clone(), source_data);
                                    state.examples.cache.insert(pair.target_record_id.clone(), target_data);
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

                                // Compute all matches using the extracted function
                                let (field_matches, relationship_matches, entity_matches, source_entities, target_entities) =
                                    recompute_all_matches(
                                        source,
                                        target,
                                        &state.field_mappings,
                                        &state.prefix_mappings,
                                    );

                                state.field_matches = field_matches;
                                state.relationship_matches = relationship_matches;
                                state.entity_matches = entity_matches;
                                state.source_entities = source_entities;
                                state.target_entities = target_entities;

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

                // Check if this is a toggle event before handling
                let is_toggle = matches!(event, crate::tui::widgets::TreeEvent::Toggle);
                let node_id_before_toggle = if is_toggle {
                    tree_state.selected().map(|s| s.to_string())
                } else {
                    None
                };

                tree_state.handle_event(event);

                // Get selected ID before releasing the borrow
                let selected_id = tree_state.selected().map(|s| s.to_string());

                // Check if node is expanded (for toggle mirroring)
                let is_expanded = if let Some(id) = &node_id_before_toggle {
                    tree_state.is_expanded(id)
                } else {
                    false
                };

                // Release the borrow by dropping tree_state reference
                drop(tree_state);

                // Mirrored selection: update target tree when source selection changes
                if let Some(source_id) = selected_id {
                    update_mirrored_selection(state, &source_id);
                }

                // Mirror container expansion/collapse
                if let Some(toggled_id) = node_id_before_toggle {
                    mirror_container_toggle(state, &toggled_id, is_expanded);
                }

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

                // Clear example cache to force re-fetch
                state.examples.cache.clear();

                let mut builder = Command::perform_parallel()
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
                    );

                // Add example data fetching tasks
                for pair in &state.examples.pairs {
                    let pair_id = pair.id.clone();
                    let source_env = state.source_env.clone();
                    let source_entity = state.source_entity.clone();
                    let source_record_id = pair.source_record_id.clone();
                    let target_env = state.target_env.clone();
                    let target_entity = state.target_entity.clone();
                    let target_record_id = pair.target_record_id.clone();

                    builder = builder.add_task(
                        format!("Refreshing example: {}", pair.display_name()),
                        async move {
                            super::fetch_example_pair_data(
                                &source_env,
                                &source_entity,
                                &source_record_id,
                                &target_env,
                                &target_entity,
                                &target_record_id,
                            ).await
                            .map(|(source, target)| super::FetchedData::ExampleData(pair_id, source, target))
                            .map_err(|e| e.to_string())
                        }
                    );
                }

                builder
                    .with_title("Refreshing Entity Comparison")
                    .on_complete(AppId::EntityComparison)
                    .build(|_task_idx, result| {
                        let data = result.downcast::<Result<super::FetchedData, String>>().unwrap();
                        Msg::ParallelDataLoaded(0, *data)
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
                    if let (Resource::Success(source), Resource::Success(target)) =
                        (&state.source_metadata, &state.target_metadata)
                    {
                        let (field_matches, relationship_matches, entity_matches, source_entities, target_entities) =
                            recompute_all_matches(
                                source,
                                target,
                                &state.field_mappings,
                                &state.prefix_mappings,
                            );
                        state.field_matches = field_matches;
                        state.relationship_matches = relationship_matches;
                        state.entity_matches = entity_matches;
                        state.source_entities = source_entities;
                        state.target_entities = target_entities;
                    }

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
                        if let (Resource::Success(source), Resource::Success(target)) =
                            (&state.source_metadata, &state.target_metadata)
                        {
                            let (field_matches, relationship_matches, entity_matches, source_entities, target_entities) =
                                recompute_all_matches(
                                    source,
                                    target,
                                    &state.field_mappings,
                                    &state.prefix_mappings,
                                );
                            state.field_matches = field_matches;
                            state.relationship_matches = relationship_matches;
                            state.entity_matches = entity_matches;
                            state.source_entities = source_entities;
                            state.target_entities = target_entities;
                        }

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
            Msg::ToggleHideMatched => {
                state.hide_matched = !state.hide_matched;
                Command::None
            }
            Msg::MappingsLoaded(field_mappings, prefix_mappings, example_pairs) => {
                // Update state with loaded mappings and examples
                state.field_mappings = field_mappings;
                state.prefix_mappings = prefix_mappings;
                state.examples.pairs = example_pairs.clone();

                // Set first pair as active if any exist
                if !state.examples.pairs.is_empty() {
                    state.examples.active_pair_id = Some(state.examples.pairs[0].id.clone());
                }

                // Now load metadata + example data in one parallel batch
                let mut builder = Command::perform_parallel()
                    // Source entity fetches
                    .add_task(
                        format!("Loading {} fields ({})", state.source_entity, state.source_env),
                        {
                            let env = state.source_env.clone();
                            let entity = state.source_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::SourceFields, true).await
                            }
                        }
                    )
                    .add_task(
                        format!("Loading {} forms ({})", state.source_entity, state.source_env),
                        {
                            let env = state.source_env.clone();
                            let entity = state.source_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::SourceForms, true).await
                            }
                        }
                    )
                    .add_task(
                        format!("Loading {} views ({})", state.source_entity, state.source_env),
                        {
                            let env = state.source_env.clone();
                            let entity = state.source_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::SourceViews, true).await
                            }
                        }
                    )
                    // Target entity fetches
                    .add_task(
                        format!("Loading {} fields ({})", state.target_entity, state.target_env),
                        {
                            let env = state.target_env.clone();
                            let entity = state.target_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::TargetFields, true).await
                            }
                        }
                    )
                    .add_task(
                        format!("Loading {} forms ({})", state.target_entity, state.target_env),
                        {
                            let env = state.target_env.clone();
                            let entity = state.target_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::TargetForms, true).await
                            }
                        }
                    )
                    .add_task(
                        format!("Loading {} views ({})", state.target_entity, state.target_env),
                        {
                            let env = state.target_env.clone();
                            let entity = state.target_entity.clone();
                            async move {
                                fetch_with_cache(&env, &entity, FetchType::TargetViews, true).await
                            }
                        }
                    );

                // Add example data fetching tasks
                for pair in example_pairs {
                    let pair_id = pair.id.clone();
                    let source_env = state.source_env.clone();
                    let source_entity = state.source_entity.clone();
                    let source_record_id = pair.source_record_id.clone();
                    let target_env = state.target_env.clone();
                    let target_entity = state.target_entity.clone();
                    let target_record_id = pair.target_record_id.clone();

                    builder = builder.add_task(
                        format!("Loading example: {}", pair.display_name()),
                        async move {
                            super::fetch_example_pair_data(
                                &source_env,
                                &source_entity,
                                &source_record_id,
                                &target_env,
                                &target_entity,
                                &target_record_id,
                            ).await
                            .map(|(source, target)| super::FetchedData::ExampleData(pair_id, source, target))
                            .map_err(|e| e.to_string())
                        }
                    );
                }

                builder
                    .with_title("Loading Entity Comparison")
                    .on_complete(AppId::EntityComparison)
                    .build(|_task_idx, result| {
                        let data = result.downcast::<Result<super::FetchedData, String>>().unwrap();
                        Msg::ParallelDataLoaded(0, *data)
                    })
            }

            // Examples modal messages
            Msg::OpenExamplesModal => {
                state.show_examples_modal = true;
                Command::None
            }
            Msg::CloseExamplesModal => {
                state.show_examples_modal = false;
                Command::None
            }
            Msg::ExamplesListNavigate(key) => {
                state.examples_list_state.handle_key(key, state.examples.pairs.len(), 10);
                Command::None
            }
            Msg::SourceInputEvent(event) => {
                state.examples_source_input.handle_event(event, None);
                Command::None
            }
            Msg::TargetInputEvent(event) => {
                state.examples_target_input.handle_event(event, None);
                Command::None
            }
            Msg::LabelInputEvent(event) => {
                state.examples_label_input.handle_event(event, None);
                Command::None
            }
            Msg::AddExamplePair => {
                // Create new example pair from inputs
                let source_id = state.examples_source_input.value().trim().to_string();
                let target_id = state.examples_target_input.value().trim().to_string();
                let label = state.examples_label_input.value().trim().to_string();

                if !source_id.is_empty() && !target_id.is_empty() {
                    let mut pair = ExamplePair::new(source_id, target_id);
                    if !label.is_empty() {
                        pair = pair.with_label(label);
                    }

                    let pair_id = pair.id.clone();
                    let source_record_id = pair.source_record_id.clone();
                    let target_record_id = pair.target_record_id.clone();

                    state.examples.pairs.push(pair.clone());

                    // Clear inputs
                    state.examples_source_input.set_value(String::new());
                    state.examples_target_input.set_value(String::new());
                    state.examples_label_input.set_value(String::new());

                    // Persist to database
                    let source_entity = state.source_entity.clone();
                    let target_entity = state.target_entity.clone();
                    tokio::spawn(async move {
                        let config = crate::global_config();
                        if let Err(e) = config.save_example_pair(&source_entity, &target_entity, &pair).await {
                            log::error!("Failed to save example pair: {}", e);
                        }
                    });

                    // Auto-fetch data for new pair
                    let source_env = state.source_env.clone();
                    let source_entity = state.source_entity.clone();
                    let target_env = state.target_env.clone();
                    let target_entity = state.target_entity.clone();

                    return Command::perform(
                        async move {
                            super::fetch_example_pair_data(
                                &source_env,
                                &source_entity,
                                &source_record_id,
                                &target_env,
                                &target_entity,
                                &target_record_id,
                            ).await.map(|(source, target)| (pair_id, source, target))
                        },
                        |result| match result {
                            Ok((pair_id, source, target)) => Msg::ExampleDataFetched(pair_id, Ok((source, target))),
                            Err(e) => Msg::ExampleDataFetched(String::new(), Err(e)),
                        }
                    );
                }

                Command::None
            }
            Msg::DeleteExamplePair => {
                // Delete selected pair from list
                if let Some(selected_idx) = state.examples_list_state.selected() {
                    if selected_idx < state.examples.pairs.len() {
                        let pair = state.examples.pairs.remove(selected_idx);

                        // Persist to database
                        let pair_id = pair.id.clone();
                        tokio::spawn(async move {
                            let config = crate::global_config();
                            if let Err(e) = config.delete_example_pair(&pair_id).await {
                                log::error!("Failed to delete example pair: {}", e);
                            }
                        });
                    }
                }
                Command::None
            }
            Msg::ExampleDataFetched(pair_id, result) => {
                // Store fetched data in cache
                match result {
                    Ok((source_data, target_data)) => {
                        // Find the pair and store its record IDs as cache keys
                        if let Some(pair) = state.examples.pairs.iter().find(|p| p.id == pair_id) {
                            log::info!("Fetched example data for pair {}: source_id={}, target_id={}",
                                pair_id, pair.source_record_id, pair.target_record_id);
                            log::debug!("Source data keys: {:?}", source_data.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                            log::debug!("Target data keys: {:?}", target_data.as_object().map(|o| o.keys().collect::<Vec<_>>()));

                            state.examples.cache.insert(pair.source_record_id.clone(), source_data);
                            state.examples.cache.insert(pair.target_record_id.clone(), target_data);
                            log::info!("Cached example data for pair {}", pair_id);
                        } else {
                            log::error!("Pair {} not found in examples.pairs", pair_id);
                        }
                    }
                    Err(err) => {
                        log::error!("Failed to fetch example data: {}", err);
                        // TODO: Show error to user
                    }
                }
                Command::None
            }
            Msg::CycleExamplePair => {
                // Cycle through pairs, or toggle off if at end
                if state.examples.pairs.is_empty() {
                    // No pairs, just toggle
                    state.examples.enabled = !state.examples.enabled;
                    state.examples.active_pair_id = None;
                } else if !state.examples.enabled {
                    // Not enabled, enable and select first
                    state.examples.enabled = true;
                    state.examples.active_pair_id = state.examples.pairs.first().map(|p| p.id.clone());
                } else if let Some(active_id) = &state.examples.active_pair_id {
                    // Find current pair index
                    let current_idx = state.examples.pairs.iter()
                        .position(|p| &p.id == active_id);

                    if let Some(idx) = current_idx {
                        // Move to next, or toggle off if at end
                        let next_idx = idx + 1;
                        if next_idx >= state.examples.pairs.len() {
                            // At end, toggle off
                            state.examples.enabled = false;
                            state.examples.active_pair_id = None;
                        } else {
                            // Move to next
                            state.examples.active_pair_id = Some(state.examples.pairs[next_idx].id.clone());
                        }
                    } else {
                        // Active ID not found, select first
                        state.examples.active_pair_id = state.examples.pairs.first().map(|p| p.id.clone());
                    }
                } else {
                    // Enabled but no active pair, select first
                    state.examples.active_pair_id = state.examples.pairs.first().map(|p| p.id.clone());
                }
                Command::None
            }
            Msg::ToggleExamples => {
                state.examples.toggle();
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

        if state.show_examples_modal {
            view = view.with_app_modal(render_examples_modal(state, theme), LayerAlignment::Center);
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

            // Examples management
            Subscription::keyboard(KeyCode::Char('e'), "Cycle example pairs", Msg::CycleExamplePair),
            Subscription::keyboard(KeyCode::Char('x'), "Open examples modal", Msg::OpenExamplesModal),
            Subscription::keyboard(KeyCode::Char('X'), "Open examples modal", Msg::OpenExamplesModal),
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

