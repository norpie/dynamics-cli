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
        let state = State {
            migration_name: params.migration_name,
            source_env: params.source_env,
            target_env: params.target_env,
            source_entity: params.source_entity,
            target_entity: params.target_entity,
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

        // TODO: Load metadata from API
        (state, Command::None)
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
        }
    }

    fn view(state: &mut Self::State, theme: &Theme) -> LayeredView<Self::Msg> {
        use_constraints!();

        // Empty source panel
        let source_panel = Element::panel(
            Element::text("")
        )
        .title(format!("Source: {}", state.source_entity))
        .build();

        // Empty target panel
        let target_panel = Element::panel(
            Element::text("")
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
