mod app;
mod models;
mod tree_items;
mod fetch;
mod tree_builder;
mod matching;

pub use app::{EntityComparisonApp, EntityComparisonParams, State as EntityComparisonState};
pub use models::*;
pub use fetch::{FetchType, fetch_with_cache, extract_relationships};

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
    MappingsLoaded(std::collections::HashMap<String, String>, std::collections::HashMap<String, String>), // field_mappings, prefix_mappings
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
