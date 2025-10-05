pub mod navigation;
pub mod tree_events;
pub mod mappings;
pub mod examples;
pub mod data_loading;

use crate::tui::command::Command;
use super::Msg;
use super::app::State;

pub fn update(state: &mut State, msg: Msg) -> Command<Msg> {
    match msg {
        // Navigation
        Msg::Back => navigation::handle_back(state),
        Msg::ConfirmBack => navigation::handle_confirm_back(),
        Msg::CancelBack => navigation::handle_cancel_back(state),
        Msg::SwitchTab(n) => navigation::handle_switch_tab(state, n),

        // Tree events
        Msg::SourceTreeEvent(event) => tree_events::handle_source_tree_event(state, event),
        Msg::TargetTreeEvent(event) => tree_events::handle_target_tree_event(state, event),
        Msg::SourceViewportHeight(h) => tree_events::handle_source_viewport_height(state, h),
        Msg::TargetViewportHeight(h) => tree_events::handle_target_viewport_height(state, h),

        // Data loading
        Msg::ParallelDataLoaded(idx, result) => data_loading::handle_parallel_data_loaded(state, idx, result),
        Msg::MappingsLoaded(fm, pm, ep) => data_loading::handle_mappings_loaded(state, fm, pm, ep),
        Msg::Refresh => data_loading::handle_refresh(state),

        // Mappings
        Msg::CreateManualMapping => mappings::handle_create_manual_mapping(state),
        Msg::DeleteManualMapping => mappings::handle_delete_manual_mapping(state),
        Msg::ToggleHideMatched => mappings::handle_toggle_hide_matched(state),
        Msg::ToggleSortMode => mappings::handle_toggle_sort_mode(state),
        Msg::ToggleTechnicalNames => mappings::handle_toggle_technical_names(state),

        // Examples
        Msg::OpenExamplesModal => examples::handle_open_modal(state),
        Msg::CloseExamplesModal => examples::handle_close_modal(state),
        Msg::ExamplesListNavigate(key) => examples::handle_list_navigate(state, key),
        Msg::SourceInputEvent(event) => examples::handle_source_input_event(state, event),
        Msg::TargetInputEvent(event) => examples::handle_target_input_event(state, event),
        Msg::LabelInputEvent(event) => examples::handle_label_input_event(state, event),
        Msg::AddExamplePair => examples::handle_add_example_pair(state),
        Msg::DeleteExamplePair => examples::handle_delete_example_pair(state),
        Msg::ExampleDataFetched(id, result) => examples::handle_example_data_fetched(state, id, result),
        Msg::CycleExamplePair => examples::handle_cycle_example_pair(state),
        Msg::ToggleExamples => examples::handle_toggle_examples(state),

        // Export
        Msg::ExportToExcel => mappings::handle_export_to_excel(state),
    }
}
