use super::models::State;
use super::tree_builder::build_snapshot_tree;
use crate::tui::{Element, Resource, renderer::LayeredView};
use ratatui::{
    text::{Line, Span},
    style::Style,
    prelude::Stylize,
};
use crate::{col, use_constraints};

pub fn render_view(state: &mut State) -> LayeredView<super::models::Msg> {
    let theme = &crate::global_runtime_config().theme;

    let content = if matches!(state.questionnaire, Resource::Success(_)) {
        render_snapshot_summary(state, theme)
    } else if let Resource::Failure(err) = &state.questionnaire {
        render_error(err, theme)
    } else {
        // Loading or NotAsked - LoadingScreen handles this
        Element::text("")
    };

    let panel = Element::panel(content)
        .title("Copy Questionnaire")
        .build();

    LayeredView::new(panel)
}

fn render_snapshot_summary(
    state: &mut State,
    theme: &crate::tui::Theme,
) -> Element<super::models::Msg> {
    use_constraints!();

    // Extract questionnaire data (we already checked it's Success in the caller)
    let questionnaire = if let Resource::Success(ref q) = state.questionnaire {
        q
    } else {
        return Element::text(""); // Should never happen
    };

    // Build tree items from questionnaire
    let tree_items = build_snapshot_tree(questionnaire);

    col![
        // Copy name input wrapped in panel
        Element::panel(
            Element::text_input("copy_name_input", state.copy_name_input.value(), &state.copy_name_input.state)
                .on_event(super::Msg::CopyNameInputEvent)
                .placeholder("Enter name for copy...")
                .build()
        )
        .title("New Questionnaire Name")
        .build() => Length(3),
        Element::text("") => Length(1),

        // Header
        Element::styled_text(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(theme.text_secondary)),
            Span::styled(state.questionnaire_name.clone(), Style::default().fg(theme.text_primary).bold()),
        ])).build() => Length(1),

        // Total
        Element::styled_text(Line::from(vec![
            Span::styled(format!("Total: {} entities", questionnaire.total_entities()), Style::default().fg(theme.accent_success).bold()),
        ])).build() => Length(1),
        Element::text("") => Length(1),

        // Tree widget wrapped in panel
        Element::panel(
            Element::tree("snapshot_tree", &tree_items, &mut state.tree_state, theme)
                .on_event(super::Msg::TreeEvent)
                .on_select(super::Msg::TreeNodeClicked)
                .on_render(super::Msg::ViewportHeight)
                .build()
        )
        .title("Questionnaire Structure")
        .build() => Fill(1),

        Element::text("") => Length(1),

        // Continue button
        Element::button("continue_button", "Continue")
            .on_press(super::Msg::Continue)
            .build() => Length(3),
    ]
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

    match &state.questionnaire {
        Resource::Success(questionnaire) => {
            Some(Line::from(vec![
                Span::styled(
                    format!("{} ({} entities)", state.questionnaire_name, questionnaire.total_entities()),
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
