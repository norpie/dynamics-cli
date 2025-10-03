use super::data_models::{ActiveTab, ExamplesState, FocusedSide, LoadingState};
use crate::{
    commands::migration::ui::{screens::comparison_apps::common::ComparisonApp, styles::STYLES},
    config::SavedComparison,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Tabs},
};

/// Helper functions for rendering the unified comparison screen
pub struct RenderHelpers;

impl RenderHelpers {
    /// Render loading screen with progress bar
    pub fn render_loading(
        f: &mut Frame,
        area: Rect,
        comparison: &SavedComparison,
        message: &str,
        progress: f64,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Message
                Constraint::Length(3), // Progress bar
                Constraint::Min(0),    // Spacer
            ])
            .split(area);

        // Loading message
        let loading_text = Paragraph::new(Line::from(vec![
            Span::styled("Loading comparison: ", STYLES.info),
            Span::styled(
                format!(
                    "{} → {}",
                    comparison.source_entity, comparison.target_entity
                ),
                STYLES.highlighted,
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(STYLES.info)
                .title("Loading Comparison"),
        );

        f.render_widget(loading_text, chunks[0]);

        // Progress bar
        let progress_bar = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(STYLES.normal),
            )
            .gauge_style(STYLES.selected)
            .percent((progress * 100.0) as u16)
            .label(message);

        f.render_widget(progress_bar, chunks[1]);
    }

    /// Render error screen
    pub fn render_error(f: &mut Frame, area: Rect, error: &str) {
        let error_text = Paragraph::new(Line::from(vec![
            Span::styled("Error: ", STYLES.error),
            Span::styled(error, STYLES.normal),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(STYLES.error)
                .title("Error Loading Comparison"),
        );

        f.render_widget(error_text, area);
    }

    /// Render the main comparison view with tabs and content
    pub fn render_comparison<FA, RA, VA, FoA>(
        f: &mut Frame,
        area: Rect,
        comparison: &SavedComparison,
        active_tab: &ActiveTab,
        focused_side: FocusedSide,
        fields_app: &mut FA,
        relationships_app: &mut RA,
        views_app: &mut VA,
        forms_app: &mut FoA,
        tab_area: &mut Option<Rect>,
        source_area: &mut Option<Rect>,
        target_area: &mut Option<Rect>,
        examples_state: &ExamplesState,
    ) where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
        VA: ComparisonApp + ?Sized,
        FoA: ComparisonApp + ?Sized,
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content area
            ])
            .split(area);

        // Render tab bar
        let tab_titles = Self::get_tab_titles();
        let tabs = Tabs::new(tab_titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(STYLES.normal)
                    .title({
                        let examples_indicator = if examples_state.examples_mode_enabled {
                            if let Some(active_example) = examples_state.get_active_example() {
                                format!(" | Examples: ON ({})", active_example.display_name())
                            } else {
                                " | Examples: ON (no active example)".to_string()
                            }
                        } else {
                            " | Examples: OFF".to_string()
                        };

                        format!(
                            "Comparison: {} → {}{}",
                            comparison.source_entity,
                            comparison.target_entity,
                            examples_indicator
                        )
                    }),
            )
            .style(STYLES.normal)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .select(Self::get_active_tab_index(active_tab));

        f.render_widget(tabs, chunks[0]);

        // Cache tab area for mouse detection
        *tab_area = Some(chunks[0]);

        // Two-column layout for source and target
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Source column
                Constraint::Percentage(50), // Target column
            ])
            .split(chunks[1]);

        // Cache the areas for mouse detection
        *source_area = Some(content_chunks[0]);
        *target_area = Some(content_chunks[1]);

        // Determine border styles based on focus - use bright cyan for focus
        let focused_border_style = Style::default().fg(Color::Cyan);

        let source_border_style = if matches!(focused_side, FocusedSide::Source) {
            focused_border_style
        } else {
            STYLES.normal
        };

        let target_border_style = if matches!(focused_side, FocusedSide::Target) {
            focused_border_style
        } else {
            STYLES.normal
        };

        // Render source and target lists using Ratatui's List widget
        match *active_tab {
            ActiveTab::Fields => {
                let source_focused = focused_side == FocusedSide::Source;
                let target_focused = focused_side == FocusedSide::Target;

                fields_app.render(
                    f,
                    content_chunks[0], // source area
                    content_chunks[1], // target area
                    source_focused,
                    target_focused,
                    examples_state,
                );
            }
            ActiveTab::Relationships => {
                Self::render_relationships(f, content_chunks, focused_side, relationships_app, examples_state);
            }
            ActiveTab::Views => {
                Self::render_view_lists(f, content_chunks, focused_side, views_app, examples_state);
            }
            ActiveTab::Forms => {
                Self::render_form_lists(f, content_chunks, focused_side, forms_app, examples_state);
            }
        }
    }

    /// Render view lists
    fn render_view_lists<VA>(
        f: &mut Frame,
        content_chunks: std::rc::Rc<[Rect]>,
        focused_side: FocusedSide,
        views_app: &mut VA,
        examples_state: &ExamplesState,
    ) where
        VA: ComparisonApp + ?Sized,
    {
        // Delegate to the views app for rendering
        let source_focused = focused_side == FocusedSide::Source;
        let target_focused = focused_side == FocusedSide::Target;

        views_app.render(
            f,
            content_chunks[0], // source area
            content_chunks[1], // target area
            source_focused,
            target_focused,
            examples_state,
        );
    }

    /// Render form lists
    fn render_form_lists<FoA>(
        f: &mut Frame,
        content_chunks: std::rc::Rc<[Rect]>,
        focused_side: FocusedSide,
        forms_app: &mut FoA,
        examples_state: &ExamplesState,
    ) where
        FoA: ComparisonApp + ?Sized,
    {
        // Delegate to the forms app for rendering
        let source_focused = focused_side == FocusedSide::Source;
        let target_focused = focused_side == FocusedSide::Target;

        forms_app.render(
            f,
            content_chunks[0], // source area
            content_chunks[1], // target area
            source_focused,
            target_focused,
            examples_state,
        );
    }

    /// Render relationships
    fn render_relationships<RA>(
        f: &mut Frame,
        content_chunks: std::rc::Rc<[Rect]>,
        focused_side: FocusedSide,
        relationships_app: &mut RA,
        examples_state: &ExamplesState,
    ) where
        RA: ComparisonApp + ?Sized,
    {
        // Delegate to the relationships app for rendering
        let source_focused = focused_side == FocusedSide::Source;
        let target_focused = focused_side == FocusedSide::Target;

        relationships_app.render(
            f,
            content_chunks[0], // source area
            content_chunks[1], // target area
            source_focused,
            target_focused,
            examples_state,
        );
    }

    /// Get tab titles for UI
    pub fn get_tab_titles() -> Vec<&'static str> {
        vec!["[1] Fields", "[2] Relationships", "[3] Views", "[4] Forms"]
    }

    /// Get active tab index
    pub fn get_active_tab_index(active_tab: &ActiveTab) -> usize {
        match *active_tab {
            ActiveTab::Fields => 0,
            ActiveTab::Relationships => 1,
            ActiveTab::Views => 2,
            ActiveTab::Forms => 3,
        }
    }

    /// Render the main screen based on loading state
    pub fn render_main_screen<FA, RA, VA, FoA>(
        f: &mut Frame,
        area: Rect,
        loading_state: &LoadingState,
        comparison: &SavedComparison,
        active_tab: &ActiveTab,
        focused_side: FocusedSide,
        fields_app: &mut FA,
        relationships_app: &mut RA,
        views_app: &mut VA,
        forms_app: &mut FoA,
        tab_area: &mut Option<Rect>,
        source_area: &mut Option<Rect>,
        target_area: &mut Option<Rect>,
        examples_state: &ExamplesState,
        start_loading_callback: &mut dyn FnMut(),
    ) where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
        VA: ComparisonApp + ?Sized,
        FoA: ComparisonApp + ?Sized,
    {
        let content_area = Rect {
            x: area.x + 1,
            y: area.y,
            width: area.width.saturating_sub(2),
            height: area.height,
        };

        match loading_state {
            LoadingState::NotStarted => {
                start_loading_callback();
                Self::render_loading(f, content_area, comparison, "Initializing...", 0.0);
            }
            LoadingState::LoadingSourceFields => {
                Self::render_loading(f, content_area, comparison, "Loading source fields...", 0.1);
            }
            LoadingState::LoadingTargetFields => {
                Self::render_loading(f, content_area, comparison, "Loading target fields...", 0.3);
            }
            LoadingState::LoadingSourceViews => {
                Self::render_loading(f, content_area, comparison, "Loading source views...", 0.5);
            }
            LoadingState::LoadingTargetViews => {
                Self::render_loading(f, content_area, comparison, "Loading target views...", 0.6);
            }
            LoadingState::LoadingSourceForms => {
                Self::render_loading(f, content_area, comparison, "Loading source forms...", 0.7);
            }
            LoadingState::LoadingTargetForms => {
                Self::render_loading(f, content_area, comparison, "Loading target forms...", 0.8);
            }
            LoadingState::ComputingMatches => {
                Self::render_loading(
                    f,
                    content_area,
                    comparison,
                    "Computing field matches...",
                    0.9,
                );
            }
            LoadingState::Complete => {
                Self::render_comparison(
                    f,
                    content_area,
                    comparison,
                    active_tab,
                    focused_side,
                    fields_app,
                    relationships_app,
                    views_app,
                    forms_app,
                    tab_area,
                    source_area,
                    target_area,
                    examples_state,
                );
            }
            LoadingState::Error(error) => {
                Self::render_error(f, content_area, error);
            }
        }
    }
}
