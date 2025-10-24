use super::models::{State, PushState, CopyProgress, CopyResult, CopyError, CopyPhase};
use crate::tui::{Element, renderer::LayeredView, LayoutConstraint};
use crate::{col, spacer, use_constraints};
use ratatui::{
    text::{Line, Span},
    style::Style,
    prelude::Stylize,
};

pub fn render_view(state: &State) -> LayeredView<super::models::Msg> {
    let theme = &crate::global_runtime_config().theme;

    let content = match &state.push_state {
        PushState::Confirming => render_confirmation_screen(state, theme),
        PushState::Copying(progress) => render_progress_screen(state, progress, theme),
        PushState::Success(result) => render_success_screen(state, result, theme),
        PushState::Failed(error) => render_failure_screen(state, error, theme),
    };

    let panel = Element::panel(content)
        .title("Push Questionnaire")
        .build();

    LayeredView::new(panel)
}

/// Screen 1: Confirmation - show summary before starting
fn render_confirmation_screen(
    state: &State,
    theme: &crate::tui::Theme,
) -> Element<super::models::Msg> {
    use_constraints!();

    // Calculate entity counts
    let total_entities = state.questionnaire.total_entities();
    let pages_count = state.questionnaire.pages.len();
    let page_lines_count = state.questionnaire.page_lines.len();
    let groups_count: usize = state.questionnaire.pages.iter().map(|p| p.groups.len()).sum();
    let group_lines_count = state.questionnaire.group_lines.len();
    let questions_count: usize = state.questionnaire.pages.iter()
        .flat_map(|p| &p.groups)
        .map(|g| g.questions.len())
        .sum();
    let template_lines_count = state.questionnaire.template_lines.len();
    let conditions_count = state.questionnaire.conditions.len();
    let condition_actions_count: usize = state.questionnaire.conditions.iter()
        .map(|c| c.actions.len())
        .sum();
    let classifications_count =
        state.questionnaire.classifications.categories.len() +
        state.questionnaire.classifications.domains.len() +
        state.questionnaire.classifications.funds.len() +
        state.questionnaire.classifications.supports.len() +
        state.questionnaire.classifications.types.len() +
        state.questionnaire.classifications.subcategories.len() +
        state.questionnaire.classifications.flemish_shares.len();

    col![
        Element::column(vec![
            Element::styled_text(Line::from(vec![
                Span::styled("Source: ", Style::default().fg(theme.text_secondary)),
                Span::styled(state.questionnaire.name.clone(), Style::default().fg(theme.text_primary).bold()),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("Copy Name: ", Style::default().fg(theme.text_secondary)),
                Span::styled(state.copy_name.clone(), Style::default().fg(theme.accent_info).bold()),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("Copy Code: ", Style::default().fg(theme.text_secondary)),
                Span::styled(state.copy_code.clone(), Style::default().fg(theme.text_primary)),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("This will create a complete copy including:", Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent_success)),
                Span::styled("1 questionnaire", Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} pages + {} ordering records", pages_count, page_lines_count), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} groups + {} ordering records", groups_count, group_lines_count), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} questions", questions_count), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} template associations", template_lines_count), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} conditions (with ID remapping)", conditions_count), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} condition actions", condition_actions_count), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} classification associations", classifications_count), Style::default().fg(theme.text_primary)),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("Total: ", Style::default().fg(theme.text_secondary)),
                Span::styled(format!("{} entities will be created", total_entities), Style::default().fg(theme.accent_info).bold()),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("References (shared, not copied):", Style::default().fg(theme.text_secondary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  → Question templates", Style::default().fg(theme.text_tertiary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  → Question tags", Style::default().fg(theme.text_tertiary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  → Classifications (categories, domains, funds, etc.)", Style::default().fg(theme.text_tertiary)),
            ])).build(),
        ]).build() => Fill(1),

        Element::button("start_copy_button", "Start Copy")
            .on_press(super::Msg::StartCopy)
            .build() => Length(3),
    ]
}

/// Screen 2: Progress - show real-time progress
fn render_progress_screen(
    state: &State,
    progress: &CopyProgress,
    theme: &crate::tui::Theme,
) -> Element<super::models::Msg> {
    Element::column(vec![
        Element::styled_text(Line::from(vec![
            Span::styled("Step ", Style::default().fg(theme.text_secondary)),
            Span::styled(format!("{}/10", progress.step), Style::default().fg(theme.accent_info).bold()),
            Span::styled(": ", Style::default().fg(theme.text_secondary)),
            Span::styled(progress.phase.name(), Style::default().fg(theme.text_primary).bold()),
        ])).build(),

        spacer!(),

        // Overall progress bar
        Element::progress_bar(progress.total_created, progress.total_entities)
            .build(),

        spacer!(),

        Element::styled_text(Line::from(vec![
            Span::styled("Progress Detail:", Style::default().fg(theme.text_primary).bold()),
        ])).build(),

        spacer!(),

        // Individual entity progress lines
        render_entity_progress("Questionnaire", progress.questionnaire, theme, matches!(progress.phase, CopyPhase::CreatingQuestionnaire)),
        render_entity_progress("Pages", progress.pages, theme, matches!(progress.phase, CopyPhase::CreatingPages)),
        render_entity_progress("Page Lines", progress.page_lines, theme, matches!(progress.phase, CopyPhase::CreatingPageLines)),
        render_entity_progress("Groups", progress.groups, theme, matches!(progress.phase, CopyPhase::CreatingGroups)),
        render_entity_progress("Group Lines", progress.group_lines, theme, matches!(progress.phase, CopyPhase::CreatingGroupLines)),
        render_entity_progress("Questions", progress.questions, theme, matches!(progress.phase, CopyPhase::CreatingQuestions)),
        render_entity_progress("Template Lines", progress.template_lines, theme, matches!(progress.phase, CopyPhase::CreatingTemplateLines)),
        render_entity_progress("Conditions", progress.conditions, theme, matches!(progress.phase, CopyPhase::CreatingConditions)),
        render_entity_progress("Condition Actions", progress.condition_actions, theme, matches!(progress.phase, CopyPhase::CreatingConditionActions)),
        render_entity_progress("Classifications", progress.classifications, theme, matches!(progress.phase, CopyPhase::CreatingClassifications)),

        spacer!(),

        Element::styled_text(Line::from(vec![
            Span::styled("Overall: ", Style::default().fg(theme.text_secondary)),
            Span::styled(
                format!("{}/{} entities created ({}%)",
                    progress.total_created,
                    progress.total_entities,
                    progress.percentage()
                ),
                Style::default().fg(theme.text_primary)
            ),
        ])).build(),

        spacer!(),

        Element::styled_text(Line::from(vec![
            Span::styled("⚠ This may take 10-30 seconds for large questionnaires",
                Style::default().fg(theme.accent_warning)),
        ])).build(),
    ]).build()
}

/// Helper to render a single entity progress line
fn render_entity_progress(
    label: &str,
    (done, total): (usize, usize),
    theme: &crate::tui::Theme,
    is_active: bool,
) -> Element<super::models::Msg> {
    let (status, color) = if done == total && total > 0 {
        ("✓", theme.accent_success)
    } else if is_active {
        ("→", theme.accent_info)
    } else if done > 0 {
        ("→", theme.accent_info)
    } else {
        (" ", theme.text_tertiary)
    };

    Element::styled_text(Line::from(vec![
        Span::styled(format!("{} ", status), Style::default().fg(color).bold()),
        Span::styled(format!("{:<30}", label), Style::default().fg(theme.text_primary)),
        Span::styled(format!("{:>6}", format!("{}/{}", done, total)), Style::default().fg(theme.text_secondary)),
    ])).build()
}

/// Screen 3a: Success - show results
fn render_success_screen(
    state: &State,
    result: &CopyResult,
    theme: &crate::tui::Theme,
) -> Element<super::models::Msg> {
    use_constraints!();

    col![
        Element::column(vec![
            Element::styled_text(Line::from(vec![
                Span::styled("✓ ", Style::default().fg(theme.accent_success).bold()),
                Span::styled("Copy Completed Successfully", Style::default().fg(theme.accent_success).bold()),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("New Questionnaire: ", Style::default().fg(theme.text_secondary)),
                Span::styled(result.new_questionnaire_name.clone(), Style::default().fg(theme.text_primary).bold()),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("ID: ", Style::default().fg(theme.text_secondary)),
                Span::styled(result.new_questionnaire_id.clone(), Style::default().fg(theme.text_tertiary)),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("Created Entities:", Style::default().fg(theme.text_primary).bold()),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} questionnaire", result.entities_created.get("questionnaire").unwrap_or(&0)), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} pages + {} page lines",
                    result.entities_created.get("pages").unwrap_or(&0),
                    result.entities_created.get("page_lines").unwrap_or(&0)
                ), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} groups + {} group lines",
                    result.entities_created.get("groups").unwrap_or(&0),
                    result.entities_created.get("group_lines").unwrap_or(&0)
                ), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} questions", result.entities_created.get("questions").unwrap_or(&0)), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} template associations", result.entities_created.get("template_lines").unwrap_or(&0)), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} conditions (IDs remapped)", result.entities_created.get("conditions").unwrap_or(&0)), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} condition actions", result.entities_created.get("condition_actions").unwrap_or(&0)), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✓ ", Style::default().fg(theme.accent_success)),
                Span::styled(format!("{} classification associations", result.entities_created.get("classifications").unwrap_or(&0)), Style::default().fg(theme.text_primary)),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("Total: ", Style::default().fg(theme.text_secondary)),
                Span::styled(format!("{} entities created in {:.1} seconds",
                    result.total_entities,
                    result.duration.as_secs_f64()
                ), Style::default().fg(theme.accent_success).bold()),
            ])).build(),
        ]).build() => Fill(1),

        Element::button("done_button", "Done")
            .on_press(super::Msg::Done)
            .build() => Length(3),
    ]
}

/// Screen 3b: Failure - show error and partial progress
fn render_failure_screen(
    state: &State,
    error: &CopyError,
    theme: &crate::tui::Theme,
) -> Element<super::models::Msg> {
    use_constraints!();

    col![
        Element::column(vec![
            Element::styled_text(Line::from(vec![
                Span::styled("✗ ", Style::default().fg(theme.accent_error).bold()),
                Span::styled("Copy Failed", Style::default().fg(theme.accent_error).bold()),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("Failed at: ", Style::default().fg(theme.text_secondary)),
                Span::styled(format!("Step {}/10 - {}", error.step, error.phase.name()), Style::default().fg(theme.text_primary).bold()),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("Error:", Style::default().fg(theme.accent_error).bold()),
            ])).build(),

            Element::panel(
                Element::styled_text(Line::from(vec![
                    Span::styled(error.error_message.clone(), Style::default().fg(theme.text_primary)),
                ])).build()
            ).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("Partial Progress (before failure):", Style::default().fg(theme.text_primary).bold()),
            ])).build(),

            spacer!(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("✓ {} questionnaire", error.partial_counts.get("questionnaire").unwrap_or(&0)), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("✓ {} pages + {} page lines",
                    error.partial_counts.get("pages").unwrap_or(&0),
                    error.partial_counts.get("page_lines").unwrap_or(&0)
                ), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("✓ {} groups + {} group lines",
                    error.partial_counts.get("groups").unwrap_or(&0),
                    error.partial_counts.get("group_lines").unwrap_or(&0)
                ), Style::default().fg(theme.text_primary)),
            ])).build(),

            Element::styled_text(Line::from(vec![
                Span::styled("  ✗ ", Style::default().fg(theme.accent_error)),
                Span::styled(format!("Failed during {}", error.phase.name()), Style::default().fg(theme.accent_error)),
            ])).build(),

            spacer!(),

            if error.rollback_complete {
                Element::styled_text(Line::from(vec![
                    Span::styled("⚠ Rollback: ", Style::default().fg(theme.accent_warning)),
                    Span::styled("Deleted all partially created entities", Style::default().fg(theme.text_primary)),
                ])).build()
            } else {
                Element::styled_text(Line::from(vec![
                    Span::styled("⚠ Warning: ", Style::default().fg(theme.accent_warning)),
                    Span::styled("Rollback incomplete - some entities may remain", Style::default().fg(theme.accent_error)),
                ])).build()
            },
        ]).build() => Fill(1),

        Element::button("retry_button", "Retry")
            .on_press(super::Msg::Retry)
            .build() => Length(3),
    ]
}

pub fn render_status(state: &State) -> Option<Line<'static>> {
    let theme = &crate::global_runtime_config().theme;

    match &state.push_state {
        PushState::Confirming => {
            Some(Line::from(vec![
                Span::styled(
                    format!("Ready to copy: {}", state.copy_name),
                    Style::default().fg(theme.text_primary),
                ),
            ]))
        }
        PushState::Copying(progress) => {
            Some(Line::from(vec![
                Span::styled(
                    format!("Copying... {}% ({}/{})",
                        progress.percentage(),
                        progress.total_created,
                        progress.total_entities
                    ),
                    Style::default().fg(theme.accent_info),
                ),
            ]))
        }
        PushState::Success(_) => {
            Some(Line::from(vec![
                Span::styled(
                    "✓ Copy complete",
                    Style::default().fg(theme.accent_success),
                ),
            ]))
        }
        PushState::Failed(_) => {
            Some(Line::from(vec![
                Span::styled(
                    "✗ Copy failed",
                    Style::default().fg(theme.accent_error),
                ),
            ]))
        }
    }
}
