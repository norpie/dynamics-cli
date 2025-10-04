mod app;
mod models;
mod tree_items;

pub use app::{EntityComparisonApp, EntityComparisonParams, State as EntityComparisonState};
pub use models::*;

// Internal message type for the app
#[derive(Clone)]
pub enum Msg {
    Back,
    ConfirmBack,
    CancelBack,
    SwitchTab(usize), // 1-indexed tab number
    ParallelDataLoaded(usize, Result<FetchedData, String>),
    Refresh,
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
