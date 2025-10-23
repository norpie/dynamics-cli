use super::models::{State, QuestionnaireSnapshot};
use crate::tui::{Element, Resource, renderer::LayeredView};
use ratatui::{
    text::{Line, Span},
    style::Style,
    prelude::Stylize,
};

pub fn render_view(state: &State) -> LayeredView<super::models::Msg> {
    let theme = &crate::global_runtime_config().theme;

    let content = match &state.snapshot {
        Resource::Success(snapshot) => {
            render_snapshot_summary(state, snapshot, theme)
        }
        Resource::Failure(err) => {
            render_error(err, theme)
        }
        _ => {
            // Loading or NotAsked - LoadingScreen handles this
            Element::text("")
        }
    };

    let panel = Element::panel(content)
        .title("Copy Questionnaire")
        .build();

    LayeredView::new(panel)
}

fn render_snapshot_summary(
    state: &State,
    snapshot: &QuestionnaireSnapshot,
    theme: &crate::tui::Theme,
) -> Element<super::models::Msg> {
    Element::column(vec![
        // Header
        Element::styled_text(Line::from(vec![
            Span::styled("Questionnaire: ", Style::default().fg(theme.text_secondary)),
            Span::styled(state.questionnaire_name.clone(), Style::default().fg(theme.text_primary).bold()),
        ])).build(),
        Element::text(""),

        // Core entities section
        Element::styled_text(Line::from(vec![
            Span::styled("Snapshot Summary", Style::default().fg(theme.accent_primary).bold()),
        ])).build(),
        Element::text(format!("  Pages: {}", snapshot.pages.len())),
        Element::text(format!("  Page Lines: {}", snapshot.page_lines.len())),
        Element::text(format!("  Groups: {}", snapshot.groups.len())),
        Element::text(format!("  Group Lines: {}", snapshot.group_lines.len())),
        Element::text(format!("  Questions: {}", snapshot.questions.len())),
        Element::text(format!("  Template Lines: {}", snapshot.template_lines.len())),
        Element::text(format!("  Conditions: {}", snapshot.conditions.len())),
        Element::text(format!("  Condition Actions: {}", snapshot.condition_actions.len())),
        Element::text(""),

        // Classifications section
        Element::styled_text(Line::from(vec![
            Span::styled("Classifications", Style::default().fg(theme.accent_primary).bold()),
        ])).build(),
        Element::text(format!("  Categories: {}", snapshot.categories.len())),
        Element::text(format!("  Domains: {}", snapshot.domains.len())),
        Element::text(format!("  Funds: {}", snapshot.funds.len())),
        Element::text(format!("  Supports: {}", snapshot.supports.len())),
        Element::text(format!("  Types: {}", snapshot.types.len())),
        Element::text(format!("  Subcategories: {}", snapshot.subcategories.len())),
        Element::text(format!("  Flemish Shares: {}", snapshot.flemish_shares.len())),
        Element::text(""),

        // Total
        Element::styled_text(Line::from(vec![
            Span::styled(format!("Total: {} entities", snapshot.total_entities()), Style::default().fg(theme.accent_success).bold()),
        ])).build(),
        Element::text(""),

        // Footer message
        Element::styled_text(Line::from(vec![
            Span::styled(
                "Copy functionality will be implemented next.",
                Style::default().fg(theme.text_secondary).italic(),
            ),
        ])).build(),
    ])
    .build()
}

fn render_error(err: &str, theme: &crate::tui::Theme) -> Element<super::models::Msg> {
    Element::column(vec![
        Element::styled_text(Line::from(vec![
            Span::styled("Error: ", Style::default().fg(theme.accent_error).bold()),
            Span::styled(err.to_string(), Style::default().fg(theme.text_primary)),
        ])).build(),
    ])
    .build()
}

pub fn render_status(state: &State) -> Option<Line<'static>> {
    let theme = &crate::global_runtime_config().theme;

    match &state.snapshot {
        Resource::Success(snapshot) => {
            Some(Line::from(vec![
                Span::styled(
                    format!("{} ({} entities)", state.questionnaire_name, snapshot.total_entities()),
                    Style::default().fg(theme.text_primary),
                ),
            ]))
        }
        _ => {
            Some(Line::from(vec![
                Span::styled(
                    state.questionnaire_name.clone(),
                    Style::default().fg(theme.text_primary),
                ),
            ]))
        }
    }
}
