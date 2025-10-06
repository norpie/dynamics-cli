use crossterm::event::KeyCode;
use std::collections::HashMap;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayeredView, Resource};
use crate::tui::element::LayoutConstraint::*;
use crate::tui::widgets::ListState;
use crate::{col, spacer};
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Stylize};

use super::models::{InspectionParams, TransformedDeadline};

pub struct DeadlinesInspectionApp;

#[derive(Clone)]
pub struct State {
    environment_name: String,
    entity_type: String,
    transformed_records: Vec<TransformedDeadline>,
}

impl State {
    fn new(environment_name: String, entity_type: String, transformed_records: Vec<TransformedDeadline>) -> Self {
        Self {
            environment_name,
            entity_type,
            transformed_records,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new(String::new(), String::new(), Vec::new())
    }
}

#[derive(Clone)]
pub enum Msg {
    Back,
    PrintAndPanic,
}

impl crate::tui::AppState for State {}

impl App for DeadlinesInspectionApp {
    type State = State;
    type Msg = Msg;
    type InitParams = InspectionParams;

    fn init(params: Self::InitParams) -> (State, Command<Msg>) {
        let state = State::new(
            params.environment_name,
            params.entity_type,
            params.transformed_records,
        );

        (state, Command::None)
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Back => Command::start_app(
                AppId::DeadlinesMapping,
                super::models::MappingParams {
                    environment_name: state.environment_name.clone(),
                    file_path: std::path::PathBuf::new(), // TODO: preserve original path
                    sheet_name: String::new(), // TODO: preserve original sheet
                },
            ),
            Msg::PrintAndPanic => {
                // Print all transformed records to terminal in a nice format
                println!("\n");
                println!("═══════════════════════════════════════════════════════════════");
                println!("  DEADLINES INSPECTION - TRANSFORMED RECORDS");
                println!("═══════════════════════════════════════════════════════════════");
                println!();
                println!("Environment: {}", state.environment_name);
                println!("Entity Type: {}", state.entity_type);
                println!("Total Records: {}", state.transformed_records.len());
                let records_with_warnings = state.transformed_records.iter()
                    .filter(|r| r.has_warnings())
                    .count();
                println!("Records with Warnings: {}", records_with_warnings);
                println!();

                for (idx, record) in state.transformed_records.iter().enumerate() {
                    println!("───────────────────────────────────────────────────────────────");
                    println!("Record {} (Excel Row {})", idx + 1, record.source_row);
                    println!("───────────────────────────────────────────────────────────────");
                    println!();

                    // Direct fields
                    if !record.direct_fields.is_empty() {
                        println!("  📝 Direct Fields:");
                        for (key, value) in &record.direct_fields {
                            println!("      {}: {}", key, value);
                        }
                        println!();
                    }

                    // Lookup fields
                    if !record.lookup_fields.is_empty() {
                        println!("  🔗 Lookup Fields (Resolved IDs):");
                        for (key, value) in &record.lookup_fields {
                            // Truncate GUID for readability
                            let truncated = if value.len() > 12 {
                                format!("{}...", &value[..12])
                            } else {
                                value.clone()
                            };
                            println!("      {}: {}", key, truncated);
                        }
                        println!();
                    }

                    // Dates
                    if record.deadline_date.is_some() || record.commission_date.is_some() {
                        println!("  📅 Dates:");
                        if let Some(date) = record.deadline_date {
                            println!("      Deadline Date: {}", date.format("%Y-%m-%d"));
                        }
                        if let Some(time) = record.deadline_time {
                            println!("      Deadline Time: {}", time.format("%H:%M:%S"));
                        }
                        if let Some(date) = record.commission_date {
                            println!("      Commission Date: {}", date.format("%Y-%m-%d"));
                        }
                        println!();
                    }

                    // Checkbox relationships (N:N)
                    if !record.checkbox_relationships.is_empty() {
                        println!("  ☑️  Checkbox Relationships (N:N):");
                        for (relationship, ids) in &record.checkbox_relationships {
                            println!("      {}: {} items", relationship, ids.len());
                            for id in ids {
                                let truncated = if id.len() > 12 {
                                    format!("{}...", &id[..12])
                                } else {
                                    id.clone()
                                };
                                println!("        - {}", truncated);
                            }
                        }
                        println!();
                    }

                    // Warnings
                    if !record.warnings.is_empty() {
                        println!("  ⚠️  Warnings ({}):", record.warnings.len());
                        for warning in &record.warnings {
                            println!("      - {}", warning);
                        }
                        println!();
                    } else {
                        println!("  ✅ No warnings");
                        println!();
                    }
                }

                println!("═══════════════════════════════════════════════════════════════");
                println!("  END OF INSPECTION");
                println!("═══════════════════════════════════════════════════════════════");
                println!();

                panic!("Inspection complete - application terminated as requested");
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> LayeredView<Msg> {
        let total_records = state.transformed_records.len();
        let records_with_warnings = state.transformed_records.iter()
            .filter(|r| r.has_warnings())
            .count();

        let content = col![
            Element::styled_text(Line::from(vec![
                Span::styled("Environment: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    state.environment_name.clone(),
                    Style::default().fg(theme.lavender)
                ),
            ]))
            .build(),
            Element::styled_text(Line::from(vec![
                Span::styled("Entity Type: ", Style::default().fg(theme.subtext0)),
                Span::styled(state.entity_type.clone(), Style::default().fg(theme.text)),
            ]))
            .build(),
            spacer!(),
            Element::styled_text(Line::from(vec![
                Span::styled("Total Records: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    total_records.to_string(),
                    Style::default().fg(theme.green).bold()
                ),
            ]))
            .build(),
            Element::styled_text(Line::from(vec![
                Span::styled("Records with Warnings: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    records_with_warnings.to_string(),
                    Style::default().fg(if records_with_warnings > 0 { theme.yellow } else { theme.green }).bold()
                ),
            ]))
            .build(),
            spacer!(),
            spacer!(),
            Element::styled_text(Line::from(vec![Span::styled(
                "Press the 'Inspect' button to print all records to terminal and exit",
                Style::default().fg(theme.subtext0).italic()
            )]))
            .build(),
            spacer!(),
            crate::row![
                Element::button("back-button", "Back")
                    .on_press(Msg::Back)
                    .build(),
                spacer!(),
                Element::button("inspect-button", "Inspect (Print & Exit)")
                    .on_press(Msg::PrintAndPanic)
                    .build(),
            ],
        ];

        let outer_panel = Element::panel(content)
            .title("Deadlines - Inspection")
            .build();

        LayeredView::new(outer_panel)
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![]
    }

    fn title() -> &'static str {
        "Deadlines - Inspection"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        Some(Line::from(vec![
            Span::styled("Records: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                state.transformed_records.len().to_string(),
                Style::default().fg(theme.lavender),
            ),
        ]))
    }
}
