use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, Command, Element, Subscription, Theme, LayoutConstraint, FocusId};
use crate::tui::widgets::{SelectState, SelectEvent};

pub struct Example6;

#[derive(Clone)]
pub enum Msg {
    SortEvent(SelectEvent),
    ExportEvent(SelectEvent),
    FilterEvent(SelectEvent),
}

pub struct State {
    sort_select: SelectState,
    export_select: SelectState,
    filter_select: SelectState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sort_select: SelectState::new(),
            export_select: SelectState::new(),
            filter_select: SelectState::new(),
        }
    }
}

impl App for Example6 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::SortEvent(event) => {
                state.sort_select.handle_event(event);
                Command::None
            }
            Msg::ExportEvent(event) => {
                state.export_select.handle_event(event);
                Command::None
            }
            Msg::FilterEvent(event) => {
                state.filter_select.handle_event(event);
                Command::None
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        // Sort mode options
        let sort_options = vec![
            "Alphabetical".to_string(),
            "By Type".to_string(),
            "By Size".to_string(),
            "Recently Modified".to_string(),
        ];

        // Export format options
        let export_options = vec![
            "JSON".to_string(),
            "CSV".to_string(),
            "Excel (XLSX)".to_string(),
            "XML".to_string(),
            "YAML".to_string(),
        ];

        // Filter options
        let filter_options = vec![
            "Show All".to_string(),
            "Show Matched".to_string(),
            "Show Unmatched".to_string(),
            "Show Modified".to_string(),
        ];

        // Build selects
        let sort_select = Element::select(
            FocusId::new("sort_select"),
            sort_options,
            &mut state.sort_select,
        )
        .on_event(Msg::SortEvent)
        .build();

        let export_select = Element::select(
            FocusId::new("export_select"),
            export_options,
            &mut state.export_select,
        )
        .on_event(Msg::ExportEvent)
        .build();

        let filter_select = Element::select(
            FocusId::new("filter_select"),
            filter_options,
            &mut state.filter_select,
        )
        .on_event(Msg::FilterEvent)
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
        let sort_names = ["Alphabetical", "By Type", "By Size", "Recently Modified"];
        let export_names = ["JSON", "CSV", "Excel (XLSX)", "XML", "YAML"];
        let filter_names = ["Show All", "Show Matched", "Show Unmatched", "Show Modified"];

        let sort_name = sort_names.get(state.sort_select.selected()).unwrap_or(&"Unknown");
        let export_name = export_names.get(state.export_select.selected()).unwrap_or(&"Unknown");
        let filter_name = filter_names.get(state.filter_select.selected()).unwrap_or(&"Unknown");

        Some(Line::from(vec![
            Span::styled("Sort: ", Style::default().fg(theme.overlay1)),
            Span::styled(sort_name.to_string(), Style::default().fg(theme.blue)),
            Span::raw(" | "),
            Span::styled("Export: ", Style::default().fg(theme.overlay1)),
            Span::styled(export_name.to_string(), Style::default().fg(theme.green)),
            Span::raw(" | "),
            Span::styled("Filter: ", Style::default().fg(theme.overlay1)),
            Span::styled(filter_name.to_string(), Style::default().fg(theme.peach)),
            Span::raw(" | "),
            Span::styled("Tab: focus | Click: toggle | ↑↓Enter when open", Style::default().fg(theme.overlay1)),
        ]))
    }
}
