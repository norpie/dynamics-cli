use crossterm::event::KeyCode;
use std::collections::HashMap;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayeredView, FocusId};
use crate::tui::element::LayoutConstraint::*;
use crate::tui::widgets::list::{ListItem, ListState};
use crate::{col, row, spacer, use_constraints};
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Stylize};

use super::models::{InspectionParams, TransformedDeadline};
use crate::tui::apps::queue::{QueueItem, QueueMetadata};
use crate::api::operations::Operations;

pub struct DeadlinesInspectionApp;

/// Wrapper for TransformedDeadline to implement ListItem trait
#[derive(Clone)]
struct RecordListItem {
    record: TransformedDeadline,
    entity_type: String, // For determining field prefix
}

impl ListItem for RecordListItem {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Msg> {
        let (fg_color, bg_style) = if is_selected {
            (theme.lavender, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, None)
        };

        // Extract name from direct fields
        let name_field = if self.entity_type == "cgk_deadline" { "cgk_name" } else { "nrq_name" };
        let name = self.record.direct_fields.get(name_field)
            .map(|s| s.as_str())
            .unwrap_or("<No Name>");

        // Truncate name if too long
        let display_name = if name.len() > 35 {
            format!("{}...", &name[..32])
        } else {
            name.to_string()
        };

        // Warning indicator
        let warning_indicator = if self.record.has_warnings() {
            Span::styled("⚠ ", Style::default().fg(theme.yellow))
        } else {
            Span::styled("  ", Style::default())
        };

        let mut builder = Element::styled_text(Line::from(vec![
            warning_indicator,
            Span::styled(format!("Row {}: ", self.record.source_row), Style::default().fg(theme.subtext0)),
            Span::styled(display_name, Style::default().fg(fg_color)),
        ]));

        if let Some(bg) = bg_style {
            builder = builder.background(bg);
        }

        builder.build()
    }
}

#[derive(Clone)]
pub struct State {
    environment_name: String,
    entity_type: String,
    transformed_records: Vec<TransformedDeadline>,
    list_state: ListState,
    selected_record_idx: usize,
}

impl State {
    fn new(environment_name: String, entity_type: String, transformed_records: Vec<TransformedDeadline>) -> Self {
        let mut list_state = ListState::default();
        // Auto-select first record if any exist
        if !transformed_records.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            environment_name,
            entity_type,
            transformed_records,
            list_state,
            selected_record_idx: 0,
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
    SelectRecord(usize),
    ListNavigate(KeyCode),
    SetViewportHeight(usize),
    Back,
    AddToQueueAndView,
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

        (state, Command::set_focus(FocusId::new("record-list")))
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::SelectRecord(idx) => {
                if idx < state.transformed_records.len() {
                    state.selected_record_idx = idx;
                    state.list_state.select(Some(idx));
                }
                Command::None
            }
            Msg::ListNavigate(key) => {
                // ListState will use its stored viewport_height from on_render, fallback to 20
                state.list_state.handle_key(key, state.transformed_records.len(), 20);

                // Sync selected_record_idx with list_state
                if let Some(selected) = state.list_state.selected() {
                    if selected != state.selected_record_idx && selected < state.transformed_records.len() {
                        state.selected_record_idx = selected;
                    }
                }

                Command::None
            }
            Msg::SetViewportHeight(height) => {
                let item_count = state.transformed_records.len();
                state.list_state.set_viewport_height(height);
                state.list_state.update_scroll(height, item_count);
                Command::None
            }
            Msg::Back => Command::start_app(
                AppId::DeadlinesMapping,
                super::models::MappingParams {
                    environment_name: state.environment_name.clone(),
                    file_path: std::path::PathBuf::new(), // TODO: preserve original path
                    sheet_name: String::new(), // TODO: preserve original sheet
                },
            ),
            Msg::AddToQueueAndView => {
                // Convert all transformed records (without warnings) to queue items
                let mut queue_items = Vec::new();

                for record in &state.transformed_records {
                    // Skip records with warnings
                    if record.has_warnings() {
                        continue;
                    }

                    // Get name for description
                    let name_field = if state.entity_type == "cgk_deadline" { "cgk_name" } else { "nrq_name" };
                    let name = record.direct_fields.get(name_field)
                        .map(|s| s.as_str())
                        .unwrap_or("<No Name>");

                    // Convert to operations
                    let operations_vec = record.to_operations(&state.entity_type);
                    let operations = Operations::from_operations(operations_vec);

                    // Create metadata
                    let metadata = QueueMetadata {
                        source: "Deadlines Excel".to_string(),
                        entity_type: state.entity_type.clone(),
                        description: format!("Row {}: {}", record.source_row, name),
                        row_number: Some(record.source_row),
                        environment_name: state.environment_name.clone(),
                    };

                    // Create queue item (priority based on row number - earlier rows = higher priority)
                    let priority = (record.source_row.min(255)) as u8;
                    let queue_item = QueueItem::new(operations, metadata, priority);
                    queue_items.push(queue_item);
                }

                // Serialize queue items to JSON for pub/sub
                let queue_items_json = match serde_json::to_value(&queue_items) {
                    Ok(json) => json,
                    Err(e) => {
                        log::error!("Failed to serialize queue items: {}", e);
                        return Command::None;
                    }
                };

                // Publish to queue and navigate
                Command::Batch(vec![
                    Command::Publish {
                        topic: "queue:add_items".to_string(),
                        data: queue_items_json,
                    },
                    Command::navigate_to(AppId::OperationQueue),
                ])
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> LayeredView<Msg> {
        use_constraints!();

        // Convert records to list items
        let list_items: Vec<RecordListItem> = state.transformed_records.iter()
            .map(|r| RecordListItem {
                record: r.clone(),
                entity_type: state.entity_type.clone(),
            })
            .collect();

        // Left panel: record list
        let record_list = Element::list(
            "record-list",
            &list_items,
            &state.list_state,
            theme
        )
        .on_select(Msg::SelectRecord)
        .on_activate(Msg::SelectRecord)
        .on_navigate(Msg::ListNavigate)
        .on_render(Msg::SetViewportHeight)
        .build();

        let records_with_warnings = state.transformed_records.iter()
            .filter(|r| r.has_warnings())
            .count();

        let list_title = format!("Records ({} total, {} warnings)",
            state.transformed_records.len(),
            records_with_warnings
        );

        let left_panel = Element::panel(record_list)
            .title(&list_title)
            .build();

        // Right panel: details for selected record
        let detail_content = if let Some(record) = state.transformed_records.get(state.selected_record_idx) {
            build_detail_panel(record, &state.entity_type, theme)
        } else {
            col![
                Element::styled_text(Line::from(vec![
                    Span::styled("No record selected", Style::default().fg(theme.subtext0))
                ])).build()
            ]
        };

        let detail_title = if let Some(record) = state.transformed_records.get(state.selected_record_idx) {
            let name_field = if state.entity_type == "cgk_deadline" { "cgk_name" } else { "nrq_name" };
            let name = record.direct_fields.get(name_field)
                .map(|s| s.as_str())
                .unwrap_or("<No Name>");
            format!("Row {} - {}", record.source_row, name)
        } else {
            "Record Details".to_string()
        };

        let right_panel = Element::panel(detail_content)
            .title(&detail_title)
            .build();

        // Main layout - two panels side by side with buttons at bottom
        let main_content = col![
            row![
                left_panel => Length(45),
                right_panel => Fill(1),
            ] => Fill(1),
            spacer!() => Length(1),
            row![
                Element::button("back-button", "Back")
                    .on_press(Msg::Back)
                    .build(),
                spacer!(),
                Element::button("queue-button", "Add to Queue & View")
                    .on_press(Msg::AddToQueueAndView)
                    .build(),
            ] => Length(3),
        ];

        let outer_panel = Element::panel(main_content)
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
        let records_with_warnings = state.transformed_records.iter()
            .filter(|r| r.has_warnings())
            .count();

        Some(Line::from(vec![
            Span::styled("Records: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                state.transformed_records.len().to_string(),
                Style::default().fg(theme.lavender),
            ),
            Span::styled(" | Warnings: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                records_with_warnings.to_string(),
                Style::default().fg(if records_with_warnings > 0 { theme.yellow } else { theme.green }),
            ),
        ]))
    }
}

/// Build the detail panel for a selected record
fn build_detail_panel(record: &TransformedDeadline, entity_type: &str, theme: &Theme) -> Element<Msg> {
    use crate::tui::element::ColumnBuilder;

    let mut builder = ColumnBuilder::new();

    // Direct fields section
    if !record.direct_fields.is_empty() {
        builder = builder.add(Element::styled_text(Line::from(vec![
            Span::styled("📝 Direct Fields", Style::default().fg(theme.blue).bold())
        ])).build(), Length(1));

        for (key, value) in &record.direct_fields {
            builder = builder.add(Element::styled_text(Line::from(vec![
                Span::styled(format!("  {}: ", key), Style::default().fg(theme.subtext0)),
                Span::styled(value.clone(), Style::default().fg(theme.text)),
            ])).build(), Length(1));
        }
        builder = builder.add(spacer!(), Length(1));
    }

    // Lookup fields section
    if !record.lookup_fields.is_empty() {
        builder = builder.add(Element::styled_text(Line::from(vec![
            Span::styled("🔗 Lookup Fields (Resolved IDs)", Style::default().fg(theme.blue).bold())
        ])).build(), Length(1));

        for (key, value) in &record.lookup_fields {
            let truncated = if value.len() > 20 {
                format!("{}...", &value[..20])
            } else {
                value.clone()
            };

            builder = builder.add(Element::styled_text(Line::from(vec![
                Span::styled(format!("  {}: ", key), Style::default().fg(theme.subtext0)),
                Span::styled(truncated, Style::default().fg(theme.green)),
            ])).build(), Length(1));
        }
        builder = builder.add(spacer!(), Length(1));
    }

    // Dates section
    if record.deadline_date.is_some() || record.commission_date.is_some() {
        builder = builder.add(Element::styled_text(Line::from(vec![
            Span::styled("📅 Dates", Style::default().fg(theme.blue).bold())
        ])).build(), Length(1));

        if let Some(date) = record.deadline_date {
            let mut line = vec![
                Span::styled("  Deadline Date: ", Style::default().fg(theme.subtext0)),
                Span::styled(date.format("%Y-%m-%d").to_string(), Style::default().fg(theme.text)),
            ];

            if let Some(time) = record.deadline_time {
                line.push(Span::styled(" at ", Style::default().fg(theme.subtext0)));
                line.push(Span::styled(time.format("%H:%M:%S").to_string(), Style::default().fg(theme.text)));
            }

            builder = builder.add(Element::styled_text(Line::from(line)).build(), Length(1));
        }

        if let Some(date) = record.commission_date {
            builder = builder.add(Element::styled_text(Line::from(vec![
                Span::styled("  Commission Date: ", Style::default().fg(theme.subtext0)),
                Span::styled(date.format("%Y-%m-%d").to_string(), Style::default().fg(theme.text)),
            ])).build(), Length(1));
        }
        builder = builder.add(spacer!(), Length(1));
    }

    // Checkbox relationships section
    if !record.checkbox_relationships.is_empty() {
        builder = builder.add(Element::styled_text(Line::from(vec![
            Span::styled("☑️  Checkbox Relationships (N:N)", Style::default().fg(theme.blue).bold())
        ])).build(), Length(1));

        for (relationship, ids) in &record.checkbox_relationships {
            builder = builder.add(Element::styled_text(Line::from(vec![
                Span::styled(format!("  {}: ", relationship), Style::default().fg(theme.subtext0)),
                Span::styled(format!("{} items", ids.len()), Style::default().fg(theme.peach)),
            ])).build(), Length(1));

            // Show first few IDs
            for (idx, id) in ids.iter().take(3).enumerate() {
                let truncated = if id.len() > 16 {
                    format!("{}...", &id[..16])
                } else {
                    id.clone()
                };

                builder = builder.add(Element::styled_text(Line::from(vec![
                    Span::styled(format!("    {}: ", idx + 1), Style::default().fg(theme.overlay0)),
                    Span::styled(truncated, Style::default().fg(theme.green)),
                ])).build(), Length(1));
            }

            if ids.len() > 3 {
                builder = builder.add(Element::styled_text(Line::from(vec![
                    Span::styled(format!("    ... and {} more", ids.len() - 3), Style::default().fg(theme.overlay0).italic()),
                ])).build(), Length(1));
            }
        }
        builder = builder.add(spacer!(), Length(1));
    }

    // Warnings section
    builder = builder.add(Element::styled_text(Line::from(vec![
        Span::styled(
            if record.has_warnings() { "⚠️  Warnings" } else { "✅ Status" },
            Style::default().fg(if record.has_warnings() { theme.yellow } else { theme.green }).bold()
        )
    ])).build(), Length(1));

    if !record.warnings.is_empty() {
        for warning in &record.warnings {
            builder = builder.add(Element::styled_text(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.yellow)),
                Span::styled(warning.clone(), Style::default().fg(theme.red)),
            ])).build(), Length(1));
        }
    } else {
        builder = builder.add(Element::styled_text(Line::from(vec![
            Span::styled("  No warnings - record is ready for upload", Style::default().fg(theme.green))
        ])).build(), Length(1));
    }

    builder.build()
}
