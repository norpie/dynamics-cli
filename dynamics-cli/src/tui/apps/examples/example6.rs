use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, Command, Element, Subscription, Theme, LayoutConstraint, FocusId};
use crate::tui::widgets::SelectField;
use dynamics_lib_macros::AppState;

pub struct Example6;

#[derive(Clone)]
pub enum Msg {
    // No event routing messages needed - using auto-routing!
}

#[derive(AppState)]
pub struct State {
    #[widget("sort_select", options = "self.sort_options")]
    sort_select: SelectField,

    #[widget("export_select", options = "self.export_options")]
    export_select: SelectField,

    #[widget("filter_select", options = "self.filter_options")]
    filter_select: SelectField,

    sort_options: Vec<String>,
    export_options: Vec<String>,
    filter_options: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sort_select: SelectField::new(),
            export_select: SelectField::new(),
            filter_select: SelectField::new(),
            sort_options: vec![
                "Alphabetical".to_string(),
                "By Type".to_string(),
                "By Size".to_string(),
                "Recently Modified".to_string(),
            ],
            export_options: vec![
                "JSON".to_string(),
                "CSV".to_string(),
                "Excel (XLSX)".to_string(),
                "XML".to_string(),
                "YAML".to_string(),
            ],
            filter_options: vec![
                "Show All".to_string(),
                "Show Matched".to_string(),
                "Show Unmatched".to_string(),
                "Show Modified".to_string(),
            ],
        }
    }
}

impl App for Example6 {
    type State = State;
    type Msg = Msg;

    fn update(_state: &mut State, _msg: Msg) -> Command<Msg> {
        // All widget events are auto-routed - no manual handling needed!
        Command::None
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        // Build selects - NO .on_event() needed, events are auto-routed!
        let sort_select = Element::select(
            FocusId::new("sort_select"),
            state.sort_options.clone(),
            &mut state.sort_select.state,
        )
        .build();

        let export_select = Element::select(
            FocusId::new("export_select"),
            state.export_options.clone(),
            &mut state.export_select.state,
        )
        .build();

        let filter_select = Element::select(
            FocusId::new("filter_select"),
            state.filter_options.clone(),
            &mut state.filter_select.state,
        )
        .build();

        // Wrap in panels with labels
        let sort_panel = Element::panel(sort_select)
            .title("Sort Mode")
            .build();

        let export_panel = Element::panel(export_select)
            .title("Export Format")
            .build();

        let filter_panel = Element::panel(filter_select)
            .title("Filter")
            .build();

        // Layout in a column
        let content = Element::column(vec![
            Element::text("Select/Dropdown Widget Demo"),
            Element::text(""),
            sort_panel,
            Element::text(""),
            export_panel,
            Element::text(""),
            filter_panel,
        ]).build();

        Element::panel(
            Element::container(content)
                .padding(2)
                .build()
        )
        .title("Example 6 - Select Widget")
        .build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        // All keyboard navigation is handled via on_navigate callbacks
        vec![]
    }

    fn title() -> &'static str {
        "Example 6"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        // Get selected values from SelectField and convert to owned strings
        let sort_name = state.sort_select.value()
            .unwrap_or("(none)")
            .to_string();
        let export_name = state.export_select.value()
            .unwrap_or("(none)")
            .to_string();
        let filter_name = state.filter_select.value()
            .unwrap_or("(none)")
            .to_string();

        Some(Line::from(vec![
            Span::styled("Sort: ".to_string(), Style::default().fg(theme.overlay1)),
            Span::styled(sort_name, Style::default().fg(theme.blue)),
            Span::raw(" | ".to_string()),
            Span::styled("Export: ".to_string(), Style::default().fg(theme.overlay1)),
            Span::styled(export_name, Style::default().fg(theme.green)),
            Span::raw(" | ".to_string()),
            Span::styled("Filter: ".to_string(), Style::default().fg(theme.overlay1)),
            Span::styled(filter_name, Style::default().fg(theme.peach)),
            Span::raw(" | ".to_string()),
            Span::styled("Auto-routed with #[derive(AppState)]".to_string(), Style::default().fg(theme.mauve).bold()),
        ]))
    }
}
