use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayoutConstraint, Resource};
use crate::tui::element::FocusId;

pub struct Example1;

#[derive(Clone)]
pub enum Msg {
    NavigateToExample2,
    LoadData,
    DataLoaded(Result<String, String>),
    LoadParallel,
    ParallelDataLoaded(usize, Result<String, String>),
}

#[derive(Default)]
pub struct State {
    data: Resource<String>,
    parallel_data1: Resource<String>,
    parallel_data2: Resource<String>,
}

impl App for Example1 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::NavigateToExample2 => Command::navigate_to(AppId::Example2),
            Msg::LoadData => {
                state.data = Resource::Loading;
                // Simulate an async API call
                Command::perform(
                    async {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        Ok("Data loaded from async operation!".to_string())
                    },
                    Msg::DataLoaded,
                )
            }
            Msg::DataLoaded(result) => {
                state.data = Resource::from_result(result);
                Command::None
            }
            Msg::LoadParallel => {
                state.parallel_data1 = Resource::Loading;
                state.parallel_data2 = Resource::Loading;

                // NEW API: Load two things in parallel with automatic LoadingScreen
                Command::perform_parallel()
                    .add_task("Loading dataset 1", async {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        Ok::<String, String>("Dataset 1 loaded!".to_string())
                    })
                    .add_task("Loading dataset 2", async {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        Ok::<String, String>("Dataset 2 loaded!".to_string())
                    })
                    .with_title("Loading Data in Parallel")
                    .on_complete(AppId::Example1)
                    .build(|task_idx, result| {
                        // Downcast the result to the expected type
                        let data = result.downcast::<Result<String, String>>().unwrap();
                        Msg::ParallelDataLoaded(task_idx, *data)
                    })
            }
            Msg::ParallelDataLoaded(task_idx, result) => {
                match task_idx {
                    0 => state.parallel_data1 = Resource::from_result(result),
                    1 => state.parallel_data2 = Resource::from_result(result),
                    _ => {}
                }
                Command::None
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        let data_display = match state.data.as_ref() {
            Resource::NotAsked => "No data loaded yet",
            Resource::Loading => "Loading...",
            Resource::Success(data) => data,
            Resource::Failure(err) => err.as_str(),
        };

        let parallel1_display = match state.parallel_data1.as_ref() {
            Resource::NotAsked => "Not loaded",
            Resource::Loading => "Loading...",
            Resource::Success(data) => data.as_str(),
            Resource::Failure(err) => err.as_str(),
        };

        let parallel2_display = match state.parallel_data2.as_ref() {
            Resource::NotAsked => "Not loaded",
            Resource::Loading => "Loading...",
            Resource::Success(data) => data.as_str(),
            Resource::Failure(err) => err.as_str(),
        };

        // Demonstrate new constraint-based layout system
        use crate::tui::element::ColumnBuilder;

        ColumnBuilder::new()
            // Fixed header (3 lines)
            .add(
                Element::text("Example 1 - New Constraint Layout System + Parallel Loading"),
                LayoutConstraint::Length(3),
            )
            // Navigation button (3 lines)
            .add(
                Element::button(FocusId::new("nav-button"), "[ Press 2 or click to go to Example 2 ]")
                    .on_press(Msg::NavigateToExample2)
                    .build(),
                LayoutConstraint::Length(3),
            )
            // Load button (3 lines)
            .add(
                Element::button(FocusId::new("load-button"), "[ Press L to load data async ]")
                    .on_press(Msg::LoadData)
                    .build(),
                LayoutConstraint::Length(3),
            )
            // Parallel load button (3 lines)
            .add(
                Element::button(FocusId::new("parallel-button"), "[ Press P to load parallel data ]")
                    .on_press(Msg::LoadParallel)
                    .build(),
                LayoutConstraint::Length(3),
            )
            // Flexible content area - fills remaining space
            .add(
                Element::column(vec![
                    Element::text(""),
                    Element::text(format!("Single async: {}", data_display)),
                    Element::text(""),
                    Element::text(format!("Parallel 1: {}", parallel1_display)),
                    Element::text(format!("Parallel 2: {}", parallel2_display)),
                    Element::text(""),
                    Element::text("Layout Features:"),
                    Element::text("✓ Fixed-size header and buttons"),
                    Element::text("✓ This content area fills remaining space"),
                    Element::text("✓ Automatic space distribution"),
                    Element::text("✓ LayoutConstraint::Length(n) for fixed"),
                    Element::text("✓ LayoutConstraint::Fill(weight) for flexible"),
                    Element::text("✓ LayoutConstraint::Min(n) for minimum"),
                    Element::text(""),
                    Element::text("Parallel Loading:"),
                    Element::text("✓ Command::perform_parallel() API"),
                    Element::text("✓ Automatic LoadingScreen management"),
                    Element::text("✓ Progress tracking per task"),
                    Element::text("✓ Auto-navigation on completion"),
                ])
                .build(),
                LayoutConstraint::Fill(1),
            )
            // Fixed footer (1 line)
            .add(
                Element::text("Footer: Constraint-based layouts make real apps possible!"),
                LayoutConstraint::Length(1),
            )
            .spacing(0)
            .build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Char('2'), "Navigate to Example 2", Msg::NavigateToExample2),
            Subscription::keyboard(KeyCode::Char('l'), "Load data asynchronously", Msg::LoadData),
            Subscription::keyboard(KeyCode::Char('L'), "Load data asynchronously", Msg::LoadData),
            Subscription::keyboard(KeyCode::Char('p'), "Load parallel data", Msg::LoadParallel),
            Subscription::keyboard(KeyCode::Char('P'), "Load parallel data", Msg::LoadParallel),
        ]
    }

    fn title() -> &'static str {
        "Example 1"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        if state.data.is_loading() {
            Some(Line::from(Span::styled("[Loading...]", Style::default().fg(theme.yellow))))
        } else {
            None
        }
    }
}