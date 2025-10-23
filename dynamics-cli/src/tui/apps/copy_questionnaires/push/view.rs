use super::models::State;
use crate::tui::{Element, renderer::LayeredView};
use ratatui::{
    text::{Line, Span},
    style::Style,
    prelude::Stylize,
};

pub fn render_view(state: &State) -> LayeredView<super::models::Msg> {
    let theme = &crate::global_runtime_config().theme;

    let content = Element::column(vec![
        Element::styled_text(Line::from(vec![
            Span::styled("Push Questionnaire (Stub)", Style::default().fg(theme.text_primary).bold()),
        ])).build(),
        Element::text(""),
        Element::styled_text(Line::from(vec![
            Span::styled("Questionnaire ID: ", Style::default().fg(theme.text_secondary)),
            Span::styled(state.questionnaire_id.clone(), Style::default().fg(theme.text_primary)),
        ])).build(),
        Element::styled_text(Line::from(vec![
            Span::styled("Copy Name: ", Style::default().fg(theme.text_secondary)),
            Span::styled(state.copy_name.clone(), Style::default().fg(theme.text_primary)),
        ])).build(),
        Element::text(""),
        Element::text("This is a stub. Copy implementation will go here."),
    ])
    .build();

    let panel = Element::panel(content)
        .title("Push Questionnaire")
        .build();

    LayeredView::new(panel)
}

pub fn render_status(state: &State) -> Option<Line<'static>> {
    let theme = &crate::global_runtime_config().theme;
    Some(Line::from(vec![
        Span::styled(
            format!("Push: {}", state.copy_name),
            Style::default().fg(theme.text_primary),
        ),
    ]))
}
