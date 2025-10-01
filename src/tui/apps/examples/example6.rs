use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, Command, Element, Subscription, Theme, LayoutConstraint, FocusId};
use crate::tui::widgets::SelectState;

pub struct Example6;

#[derive(Clone)]
pub enum Msg {
    // Sort select
    SortModeSelected(usize),
    ToggleSortDropdown,
    SortNavigate(KeyCode),

    // Export format select
    ExportFormatSelected(usize),
    ToggleExportDropdown,
    ExportNavigate(KeyCode),

    // Filter select
    FilterSelected(usize),
    ToggleFilterDropdown,
    FilterNavigate(KeyCode),
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
            // Sort select handlers
            Msg::ToggleSortDropdown => {
                state.sort_select.toggle();
                Command::None
            }
            Msg::SortNavigate(key) => {
                match key {
                    KeyCode::Up => state.sort_select.navigate_prev(),
                    KeyCode::Down => state.sort_select.navigate_next(),
                    KeyCode::Enter => state.sort_select.select_highlighted(),
                    KeyCode::Esc => state.sort_select.close(),
                    _ => {}
                }
                Command::None
            }
            Msg::SortModeSelected(idx) => {
                state.sort_select.select(idx);
                Command::None
            }

            // Export format handlers
            Msg::ToggleExportDropdown => {
                state.export_select.toggle();
                Command::None
            }
            Msg::ExportNavigate(key) => {
                match key {
                    KeyCode::Up => state.export_select.navigate_prev(),
                    KeyCode::Down => state.export_select.navigate_next(),
                    KeyCode::Enter => state.export_select.select_highlighted(),
                    KeyCode::Esc => state.export_select.close(),
                    _ => {}
                }
                Command::None
            }
            Msg::ExportFormatSelected(idx) => {
                state.export_select.select(idx);
                Command::None
            }

            // Filter handlers
            Msg::ToggleFilterDropdown => {
                state.filter_select.toggle();
                Command::None
            }
            Msg::FilterNavigate(key) => {
                match key {
                    KeyCode::Up => state.filter_select.navigate_prev(),
                    KeyCode::Down => state.filter_select.navigate_next(),
                    KeyCode::Enter => state.filter_select.select_highlighted(),
                    KeyCode::Esc => state.filter_select.close(),
                    _ => {}
                }
                Command::None
            }
            Msg::FilterSelected(idx) => {
                state.filter_select.select(idx);
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
        .on_select(Msg::SortModeSelected)
        .on_toggle(Msg::ToggleSortDropdown)
        .on_navigate(Msg::SortNavigate)
        .build();

        let export_select = Element::select(
            FocusId::new("export_select"),
            export_options,
            &mut state.export_select,
        )
        .on_select(Msg::ExportFormatSelected)
        .on_toggle(Msg::ToggleExportDropdown)
        .on_navigate(Msg::ExportNavigate)
        .build();

        let filter_select = Element::select(
            FocusId::new("filter_select"),
            filter_options,
            &mut state.filter_select,
        )
        .on_select(Msg::FilterSelected)
        .on_toggle(Msg::ToggleFilterDropdown)
        .on_navigate(Msg::FilterNavigate)
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
