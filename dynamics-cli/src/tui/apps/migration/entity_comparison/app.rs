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
use super::{Msg, Side, ExamplesState, ActiveTab};

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
    field_mappings: HashMap<String, String>,  // source -> target
    prefix_mappings: HashMap<String, String>, // source_prefix -> target_prefix
    hide_matched: bool,

    // Tree UI state
    source_tree_state: TreeState,
    target_tree_state: TreeState,
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
            source_tree_state: TreeState::new(),
            target_tree_state: TreeState::new(),
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

                        // Focus tree when both fully loaded
                        if let (Resource::Success(source), Resource::Success(target)) =
                            (&state.source_metadata, &state.target_metadata)
                        {
                            if !source.fields.is_empty() && !target.fields.is_empty() {
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
        }
    }

    fn view(state: &mut Self::State, theme: &Theme) -> LayeredView<Self::Msg> {
        use_constraints!();

        // Source panel with tree
        let source_items: Vec<super::tree_items::FieldNode> = vec![];
        let source_panel = Element::panel(
            Element::tree("source_tree", &source_items, &mut state.source_tree_state, theme)
                .build()
        )
        .title(format!("Source: {}", state.source_entity))
        .build();

        // Target panel with tree
        let target_items: Vec<super::tree_items::FieldNode> = vec![];
        let target_panel = Element::panel(
            Element::tree("target_tree", &target_items, &mut state.target_tree_state, theme)
                .build()
        )
        .title(format!("Target: {}", state.target_entity))
        .build();

        // Side-by-side layout
        let main_ui = row![
            source_panel => Fill(1),
            target_panel => Fill(1),
        ];

        let mut view = LayeredView::new(main_ui);

        // Add confirmation modal if showing
        if state.show_back_confirmation {
            let modal = ConfirmationModal::new("Go Back?")
                .message("Are you sure you want to go back to the comparison list?")
                .confirm_text("Yes")
                .cancel_text("No")
                .on_confirm(Msg::ConfirmBack)
                .on_cancel(Msg::CancelBack)
                .width(60)
                .height(10)
                .build(theme);

            view = view.with_app_modal(modal, LayerAlignment::Center);
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

/// Type of data to fetch
enum FetchType {
    SourceFields,
    SourceForms,
    SourceViews,
    TargetFields,
    TargetForms,
    TargetViews,
}

/// Fetch specific metadata type with optional 12-hour caching
async fn fetch_with_cache(
    environment_name: &str,
    entity_name: &str,
    fetch_type: FetchType,
    use_cache: bool,
) -> Result<super::FetchedData, String> {
    let config = crate::global_config();
    let manager = crate::client_manager();

    // Check cache first (12 hours) - use full metadata cache, only if use_cache is true
    if use_cache {
        let cached_metadata = config.get_entity_metadata_cache(environment_name, entity_name, 12).await
            .ok()
            .flatten();

        // If we have cached metadata, extract the requested type
        if let Some(cached) = cached_metadata {
            return match fetch_type {
                FetchType::SourceFields => Ok(super::FetchedData::SourceFields(cached.fields)),
                FetchType::SourceForms => Ok(super::FetchedData::SourceForms(cached.forms)),
                FetchType::SourceViews => Ok(super::FetchedData::SourceViews(cached.views)),
                FetchType::TargetFields => Ok(super::FetchedData::TargetFields(cached.fields)),
                FetchType::TargetForms => Ok(super::FetchedData::TargetForms(cached.forms)),
                FetchType::TargetViews => Ok(super::FetchedData::TargetViews(cached.views)),
            };
        }
    }

    // Fetch from API
    let client = manager.get_client(environment_name).await
        .map_err(|e| e.to_string())?;

    match fetch_type {
        FetchType::SourceFields => {
            let fields = client.fetch_entity_fields(entity_name).await.map_err(|e| e.to_string())?;
            Ok(super::FetchedData::SourceFields(fields))
        }
        FetchType::SourceForms => {
            let forms = client.fetch_entity_forms(entity_name).await.map_err(|e| e.to_string())?;
            Ok(super::FetchedData::SourceForms(forms))
        }
        FetchType::SourceViews => {
            let views = client.fetch_entity_views(entity_name).await.map_err(|e| e.to_string())?;
            Ok(super::FetchedData::SourceViews(views))
        }
        FetchType::TargetFields => {
            let fields = client.fetch_entity_fields(entity_name).await.map_err(|e| e.to_string())?;
            Ok(super::FetchedData::TargetFields(fields))
        }
        FetchType::TargetForms => {
            let forms = client.fetch_entity_forms(entity_name).await.map_err(|e| e.to_string())?;
            Ok(super::FetchedData::TargetForms(forms))
        }
        FetchType::TargetViews => {
            let views = client.fetch_entity_views(entity_name).await.map_err(|e| e.to_string())?;
            Ok(super::FetchedData::TargetViews(views))
        }
    }
}

/// Extract relationships from field list
fn extract_relationships(fields: &[crate::api::metadata::FieldMetadata]) -> Vec<crate::api::metadata::RelationshipMetadata> {
    fields.iter()
        .filter(|f| matches!(f.field_type, crate::api::metadata::FieldType::Lookup))
        .map(|f| crate::api::metadata::RelationshipMetadata {
            name: f.logical_name.clone(),
            relationship_type: crate::api::metadata::RelationshipType::ManyToOne,
            related_entity: f.related_entity.clone().unwrap_or_default(),
            related_attribute: f.logical_name.clone(),
        })
        .collect()
}
