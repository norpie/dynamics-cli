use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayoutConstraint, FocusId};
use crate::tui::element::ColumnBuilder;

pub struct Example4;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Overview,
    Details,
}

impl TabId {
    fn name(&self) -> &'static str {
        match self {
            TabId::Overview => "Overview",
            TabId::Details => "Details",
        }
    }

    fn index(&self) -> usize {
        match self {
            TabId::Overview => 0,
            TabId::Details => 1,
        }
    }

    fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(TabId::Overview),
            1 => Some(TabId::Details),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub enum Msg {
    SwitchTab(TabId),
    GoBack,
}

pub struct State {
    active_tab: TabId,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_tab: TabId::Overview,
        }
    }
}

impl App for Example4 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::SwitchTab(tab) => {
                state.active_tab = tab;
                Command::None
            }
            Msg::GoBack => {
                Command::navigate_to(AppId::AppLauncher)
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        // Render different content based on active tab
        let content = match state.active_tab {
            TabId::Overview => {
                Element::panel(
                    Element::container(
                        ColumnBuilder::new()
                            .add(
                                Element::text("Welcome to the Tab Pattern Example!"),
                                LayoutConstraint::Length(1),
                            )
                            .add(Element::text(""), LayoutConstraint::Length(1))
                            .add(
                                Element::text("This demonstrates how tabs work in the TUI framework."),
                                LayoutConstraint::Length(1),
                            )
                            .add(Element::text(""), LayoutConstraint::Length(1))
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("• Tabs are just app state (active_tab: TabId)", Style::default().fg(theme.green)),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("• Content is conditionally rendered via match", Style::default().fg(theme.green)),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("• Tab state shows in panel title & app status", Style::default().fg(theme.green)),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .add(Element::text(""), LayoutConstraint::Length(1))
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("Press 2 to see the Details tab!", Style::default().fg(theme.overlay1).italic()),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .build()
                    )
                    .padding(2)
                    .build()
                )
                .title("Overview Tab")
                .build()
            }
            TabId::Details => {
                Element::panel(
                    Element::container(
                        ColumnBuilder::new()
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("Technical Details", Style::default().fg(theme.blue).bold()),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .add(Element::text(""), LayoutConstraint::Length(1))
                            .add(
                                Element::text("State Structure:"),
                                LayoutConstraint::Length(1),
                            )
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("  active_tab: TabId", Style::default().fg(theme.mauve)),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .add(Element::text(""), LayoutConstraint::Length(1))
                            .add(
                                Element::text("Tab Switching:"),
                                LayoutConstraint::Length(1),
                            )
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("  Subscription::keyboard(KeyCode::Char('1'), ...)", Style::default().fg(theme.mauve)),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("  Subscription::keyboard(KeyCode::Char('2'), ...)", Style::default().fg(theme.mauve)),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .add(Element::text(""), LayoutConstraint::Length(1))
                            .add(
                                Element::styled_text(Line::from(vec![
                                    Span::styled("Press 1 to go back to Overview!", Style::default().fg(theme.overlay1).italic()),
                                ])).build(),
                                LayoutConstraint::Length(1),
                            )
                            .build()
                    )
                    .padding(2)
                    .build()
                )
                .title("Details Tab")
                .build()
            }
        };

        ColumnBuilder::new()
            .add(
                Element::styled_text(Line::from(vec![
                    Span::styled("Tab Pattern Demo", Style::default().fg(theme.blue).bold()),
                ])).build(),
                LayoutConstraint::Length(1),
            )
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(content, LayoutConstraint::Fill(1))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(
                Element::button(FocusId::new("back"), "[ Go Back ]")
                    .on_press(Msg::GoBack)
                    .build(),
                LayoutConstraint::Length(3),
            )
            .build()
    }

    fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Char('1'), "Switch to Overview", Msg::SwitchTab(TabId::Overview)),
            Subscription::keyboard(KeyCode::Char('2'), "Switch to Details", Msg::SwitchTab(TabId::Details)),
            Subscription::keyboard(KeyCode::Esc, "Go back", Msg::GoBack),
        ]
    }

    fn title() -> &'static str {
        "Example 4 - Tab Pattern"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        // Tab state shown in status line
        let tab_name = state.active_tab.name();
        let indicator = format!("[{} Tab - Press 1/2 to switch]", tab_name);
        Some(Line::from(Span::styled(indicator, Style::default().fg(theme.lavender))))
    }
}
