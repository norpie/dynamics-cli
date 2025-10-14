mod app;
mod models;
mod tree_items;
mod fetch;
mod tree_builder;
mod matching;
mod view;
mod tree_sync;
mod update;
mod export;

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
    SourceTreeNodeClicked(String), // Node clicked in source tree
    TargetTreeNodeClicked(String), // Node clicked in target tree
    CreateManualMapping,  // Create mapping from selected source to selected target
    DeleteManualMapping,  // Delete mapping from selected field
    ToggleHideMatched,    // Toggle showing/hiding matched items in trees
    ToggleSortMode,       // Toggle between Alphabetical and MatchesFirst sorting
    ToggleTechnicalNames, // Toggle between technical (logical) and display names
    MappingsLoaded(std::collections::HashMap<String, String>, std::collections::HashMap<String, String>, std::collections::HashMap<String, String>, Option<String>, Vec<ExamplePair>), // field_mappings, prefix_mappings, imported_mappings, import_source_file, example_pairs

    // Examples modal messages
    OpenExamplesModal,
    CloseExamplesModal,
    ExamplesListNavigate(crossterm::event::KeyCode),
    ExamplesListSelect(usize),
    SourceInputEvent(crate::tui::widgets::TextInputEvent),
    TargetInputEvent(crate::tui::widgets::TextInputEvent),
    LabelInputEvent(crate::tui::widgets::TextInputEvent),
    AddExamplePair,
    DeleteExamplePair,
    ExampleDataFetched(String, Result<(serde_json::Value, serde_json::Value), String>), // pair_id, (source_data, target_data)
    CycleExamplePair,
    ToggleExamples,

    // Prefix mappings modal messages
    OpenPrefixMappingsModal,
    ClosePrefixMappingsModal,
    PrefixMappingsListNavigate(crossterm::event::KeyCode),
    PrefixMappingsListSelect(usize),
    PrefixSourceInputEvent(crate::tui::widgets::TextInputEvent),
    PrefixTargetInputEvent(crate::tui::widgets::TextInputEvent),
    AddPrefixMapping,
    DeletePrefixMapping,

    // Manual mappings modal messages
    OpenManualMappingsModal,
    CloseManualMappingsModal,
    ManualMappingsListNavigate(crossterm::event::KeyCode),
    ManualMappingsListSelect(usize),
    DeleteManualMappingFromModal,

    // Export
    ExportToExcel,

    // Import from C# file
    OpenImportModal,
    CloseImportModal,
    ImportFileSelected(std::path::PathBuf),
    ImportMappingsLoaded(std::collections::HashMap<String, String>, String), // mappings, filename
    ClearImportedMappings,
    ImportNavigate(crossterm::event::KeyCode),
    ImportSetViewportHeight(usize),
    CloseImportResultsModal,
    ImportResultsNavigate(crossterm::event::KeyCode),
    ImportResultsSelect(usize),
    ImportResultsSetViewportHeight(usize),
}

#[derive(Clone)]
pub enum FetchedData {
    SourceFields(Vec<crate::api::metadata::FieldMetadata>),
    SourceForms(Vec<crate::api::metadata::FormMetadata>),
    SourceViews(Vec<crate::api::metadata::ViewMetadata>),
    TargetFields(Vec<crate::api::metadata::FieldMetadata>),
    TargetForms(Vec<crate::api::metadata::FormMetadata>),
    TargetViews(Vec<crate::api::metadata::ViewMetadata>),
    ExampleData(String, serde_json::Value, serde_json::Value), // pair_id, source_data, target_data
}
