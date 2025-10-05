mod app;
mod models;
mod tree_items;
mod fetch;
mod tree_builder;
mod matching;

pub use app::{EntityComparisonApp, EntityComparisonParams, State as EntityComparisonState};
pub use models::*;
pub use fetch::{FetchType, fetch_with_cache, extract_relationships, extract_entities, fetch_example_pair_data};

// Internal message type for the app
#[derive(Clone)]
pub enum Msg {
    Back,
    ConfirmBack,
    CancelBack,
    SwitchTab(usize), // 1-indexed tab number
    ParallelDataLoaded(usize, Result<FetchedData, String>),
    Refresh,
    SourceTreeEvent(crate::tui::widgets::TreeEvent),
    TargetTreeEvent(crate::tui::widgets::TreeEvent),
    SourceViewportHeight(usize),  // Called by renderer with actual area.height
    TargetViewportHeight(usize),  // Called by renderer with actual area.height
    CreateManualMapping,  // Create mapping from selected source to selected target
    DeleteManualMapping,  // Delete mapping from selected field
    ToggleHideMatched,    // Toggle showing/hiding matched items in trees
    MappingsLoaded(std::collections::HashMap<String, String>, std::collections::HashMap<String, String>, Vec<ExamplePair>), // field_mappings, prefix_mappings, example_pairs

    // Examples modal messages
    OpenExamplesModal,
    CloseExamplesModal,
    ExamplesListNavigate(crossterm::event::KeyCode),
    SourceInputEvent(crate::tui::widgets::TextInputEvent),
    TargetInputEvent(crate::tui::widgets::TextInputEvent),
    LabelInputEvent(crate::tui::widgets::TextInputEvent),
    AddExamplePair,
    DeleteExamplePair,
    FetchExampleData,
    ExampleDataFetched(String, Result<(serde_json::Value, serde_json::Value), String>), // pair_id, (source_data, target_data)
    CycleExamplePair,
    ToggleExamples,
}

#[derive(Clone)]
pub enum FetchedData {
    SourceFields(Vec<crate::api::metadata::FieldMetadata>),
    SourceForms(Vec<crate::api::metadata::FormMetadata>),
    SourceViews(Vec<crate::api::metadata::ViewMetadata>),
    TargetFields(Vec<crate::api::metadata::FieldMetadata>),
    TargetForms(Vec<crate::api::metadata::FormMetadata>),
    TargetViews(Vec<crate::api::metadata::ViewMetadata>),
}
