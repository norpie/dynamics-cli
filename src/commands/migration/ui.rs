use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::app::{CompareApp, ViewCompareApp, FocusedPanel, HideMode, TreeNode, MatchStatus};

impl CompareApp {
    pub fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(2), // Status line
                Constraint::Length(3), // Footer
            ])
            .split(f.area());

        // Header
        let header_text = if self.source_entity_name == self.target_entity_name {
            format!("{} | {} → {}", self.source_entity_name, self.source_env, self.target_env)
        } else {
            format!("{} vs {} | {} → {}", self.source_entity_name, self.target_entity_name, self.source_env, self.target_env)
        };

        let hide_status = match self.hide_mode {
            HideMode::ShowAll => "",
            HideMode::HideMatches => " (Hiding matches)",
        };

        // Add mapping progress
        let (mapped, total) = self.calculate_mapping_progress();
        let progress_text = format!(" | Progress: {}/{} ({:.1}%)", mapped, total, (mapped as f64 / total as f64) * 100.0);

        // Add search mode indicator
        let search_indicator = if self.search_mode {
            format!(" | Search: \"{}\"", self.search_query)
        } else {
            String::new()
        };

        let full_header = format!("{}{}{}{}", header_text, hide_status, progress_text, search_indicator);
        let header = Paragraph::new(full_header)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(header, chunks[0]);

        // Main content - split into two columns
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        // Store areas for mouse handling
        self.source_area = main_chunks[0];
        self.target_area = main_chunks[1];

        // Source fields
        self.render_source_panel(f, main_chunks[0]);

        // Target fields
        self.render_target_panel(f, main_chunks[1]);

        // Status line
        let status_text = self.get_status_line_text();
        let status_line = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status_line, chunks[2]);

        // Footer
        let footer_text = if self.search_mode {
            "Type to search | Enter/Esc: exit search | ↑↓: navigate matches"
        } else {
            "Arrow keys: navigate | Tab: switch panels | h: toggle hide matches | m: create mapping | /: search | f: fuzzy match | u: undo | e: export | F1: copy | q: quit"
        };
        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(footer, chunks[3]);

        // Render popups if active
        if self.show_mapping_popup {
            self.render_mapping_popup(f);
        }

        if self.show_prefix_popup {
            self.render_prefix_popup(f);
        }

        if self.show_prefix_input {
            self.render_prefix_input_dialog(f);
        }

        if self.show_copy_mappings_popup {
            self.render_copy_mappings_popup(f);
        }

        if self.show_fuzzy_popup {
            self.render_fuzzy_popup(f);
        }
    }

    fn render_source_panel(&mut self, f: &mut Frame, area: Rect) {
        let source_fields = self.get_filtered_source_fields();
        let source_count = source_fields.len();
        let total_count = self.source_fields.len();

        // Get current index (1-based) or 0 if no selection
        let current_index = self.source_list_state.selected().map(|i| i + 1).unwrap_or(0);

        let title = if source_count == total_count {
            if current_index > 0 {
                format!("{} ({}:{}/{})", self.source_entity_name, current_index, source_count, total_count)
            } else {
                format!("{} ({})", self.source_entity_name, source_count)
            }
        } else {
            if current_index > 0 {
                format!("{} ({}:{}/{})", self.source_entity_name, current_index, source_count, total_count)
            } else {
                format!("{} ({}/{})", self.source_entity_name, source_count, total_count)
            }
        };

        let source_style = if self.focused_panel == FocusedPanel::Source {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let source_items: Vec<ListItem> = source_fields
            .iter()
            .map(|field| {
                let mut spans = vec![];

                // Check if field is mapped or not
                let has_exact_match = self.target_fields.iter().any(|target| target.name == field.name);
                let has_manual_mapping = self.field_mappings.contains_key(&field.name);
                let has_prefix_match = self.get_prefix_matched_target(&field.name).is_some();
                let is_mapped = has_exact_match || has_manual_mapping || has_prefix_match;


                // Field name with appropriate styling
                let field_name_style = if !is_mapped {
                    // Unmapped fields are highlighted
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else if field.is_required {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if field.is_custom {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                spans.push(Span::styled(
                    format!("{}{}", field.name, if field.is_required { "*" } else { "" }),
                    field_name_style,
                ));

                // Check for matches and add indicators
                let mut match_indicators = vec![];

                // Check for exact match
                if has_exact_match {
                    match_indicators.push("exact");
                }

                // Check for manual mapping
                if has_manual_mapping {
                    match_indicators.push("manual");
                    // Check for type mismatch in manual mapping
                    if let Some(target_field) = self.field_mappings.get(&field.name) {
                        if self.has_type_mismatch(&field.name, target_field) {
                            match_indicators.push("TYPE-MISMATCH");
                        }
                    }
                }

                // Check for prefix match
                if has_prefix_match {
                    match_indicators.push("prefix");
                }

                if !match_indicators.is_empty() {
                    spans.push(Span::raw(" "));
                    let indicator_style = if match_indicators.iter().any(|m| m.contains("TYPE-MISMATCH")) {
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Yellow)
                    };
                    spans.push(Span::styled(
                        format!("[{}]", match_indicators.join(",")),
                        indicator_style,
                    ));
                }

                // Field type
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("({})", field.field_type),
                    Style::default().fg(Color::Gray),
                ));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let source_list = List::new(source_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(source_style),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(if self.focused_panel == FocusedPanel::Source {
                        Color::Yellow
                    } else {
                        Color::Gray
                    }),
            );

        f.render_stateful_widget(source_list, area, &mut self.source_list_state);
    }

    fn render_target_panel(&mut self, f: &mut Frame, area: Rect) {
        let target_fields = self.get_filtered_target_fields();
        let target_count = target_fields.len();
        let total_count = self.target_fields.len();

        // Get current index (1-based) or 0 if no selection
        let current_index = self.target_list_state.selected().map(|i| i + 1).unwrap_or(0);

        let title = if target_count == total_count {
            if current_index > 0 {
                format!("{} ({}:{}/{})", self.target_entity_name, current_index, target_count, total_count)
            } else {
                format!("{} ({})", self.target_entity_name, target_count)
            }
        } else {
            if current_index > 0 {
                format!("{} ({}:{}/{})", self.target_entity_name, current_index, target_count, total_count)
            } else {
                format!("{} ({}/{})", self.target_entity_name, target_count, total_count)
            }
        };

        let target_style = if self.focused_panel == FocusedPanel::Target {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let target_items: Vec<ListItem> = target_fields
            .iter()
            .map(|field| {
                let mut spans = vec![];

                // Check if field is mapped or not
                let has_exact_match = self.source_fields.iter().any(|source| source.name == field.name);
                let has_manual_mapping = self.field_mappings.values().any(|v| v == &field.name);
                let has_prefix_match = self.get_prefix_matched_source(&field.name).is_some();
                let is_mapped = has_exact_match || has_manual_mapping || has_prefix_match;


                // Field name with appropriate styling
                let field_name_style = if !is_mapped {
                    // Unmapped fields are highlighted
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else if field.is_required {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if field.is_custom {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                spans.push(Span::styled(
                    format!("{}{}", field.name, if field.is_required { "*" } else { "" }),
                    field_name_style,
                ));

                // Check for matches and add indicators
                let mut match_indicators = vec![];

                // Check for exact match
                if has_exact_match {
                    match_indicators.push("exact");
                }

                // Check for manual mapping (reverse lookup)
                if has_manual_mapping {
                    match_indicators.push("manual");
                    // Check for type mismatch in manual mapping
                    if let Some((source_field, _)) = self.field_mappings.iter().find(|(_, v)| *v == &field.name) {
                        if self.has_type_mismatch(source_field, &field.name) {
                            match_indicators.push("TYPE-MISMATCH");
                        }
                    }
                }

                // Check for prefix match
                if has_prefix_match {
                    match_indicators.push("prefix");
                }

                if !match_indicators.is_empty() {
                    spans.push(Span::raw(" "));
                    let indicator_style = if match_indicators.iter().any(|m| m.contains("TYPE-MISMATCH")) {
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Yellow)
                    };
                    spans.push(Span::styled(
                        format!("[{}]", match_indicators.join(",")),
                        indicator_style,
                    ));
                }

                // Field type
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("({})", field.field_type),
                    Style::default().fg(Color::Gray),
                ));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let target_list = List::new(target_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(target_style),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(if self.focused_panel == FocusedPanel::Target {
                        Color::Yellow
                    } else {
                        Color::Gray
                    }),
            );

        f.render_stateful_widget(target_list, area, &mut self.target_list_state);
    }

    fn render_copy_mappings_popup(&mut self, f: &mut Frame) {
        let popup_area = self.centered_rect(60, 70, f.area());

        // Clear the background
        f.render_widget(ratatui::widgets::Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Copy Mappings From...")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Instructions
                Constraint::Min(1),    // List
                Constraint::Length(2), // Controls
            ])
            .split(inner_area);

        // Instructions
        let instructions = Paragraph::new("Select an entity comparison to copy mappings from:")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(instructions, chunks[0]);

        // List of available comparisons
        if self.available_comparisons.is_empty() {
            let no_mappings = Paragraph::new("No other entity comparisons with mappings found.")
                .style(Style::default().fg(Color::Gray))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(no_mappings, chunks[1]);
        } else {
            let items: Vec<ListItem> = self.available_comparisons
                .iter()
                .map(|comparison| {
                    let icon = if comparison.contains("field + prefix") {
                        "🔀" // Both types
                    } else if comparison.contains("field") {
                        "🗂️" // Field mappings
                    } else {
                        "🏷️" // Prefix mappings
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("{} ", icon), Style::default().fg(Color::Blue)),
                        Span::styled(comparison, Style::default().fg(Color::White)),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Available Comparisons")
                        .borders(Borders::ALL),
                )
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .fg(Color::Yellow),
                );

            f.render_stateful_widget(list, chunks[1], &mut self.copy_mappings_state);
        }

        // Controls
        let controls = Paragraph::new("Enter: copy mappings | ↑↓: navigate | Esc: cancel")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(controls, chunks[2]);
    }

    fn render_fuzzy_popup(&mut self, f: &mut Frame) {
        let popup_area = self.centered_rect(60, 70, f.area());

        // Clear the background
        f.render_widget(ratatui::widgets::Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Fuzzy Match Suggestions")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Instructions
                Constraint::Min(1),    // List
                Constraint::Length(2), // Controls
            ])
            .split(inner_area);

        // Instructions
        let instruction_text = if let Some(source_field) = &self.selected_field_for_mapping {
            format!("Select a target field to map \"{}\" to:", source_field)
        } else {
            "Select a target field:".to_string()
        };
        let instructions = Paragraph::new(instruction_text)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(instructions, chunks[0]);

        // List of suggestions
        if self.fuzzy_suggestions.is_empty() {
            let no_suggestions = Paragraph::new("No similar fields found.")
                .style(Style::default().fg(Color::Gray))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(no_suggestions, chunks[1]);
        } else {
            let items: Vec<ListItem> = self.fuzzy_suggestions
                .iter()
                .map(|suggestion| {
                    // Find the target field to get its type and info
                    let field_info = self.target_fields.iter().find(|f| f.name == *suggestion);

                    // Calculate match percentage
                    let match_percentage = if let Some(source_field) = &self.selected_field_for_mapping {
                        self.get_fuzzy_match_percentage(source_field, suggestion)
                    } else {
                        0.0
                    };

                    let display_text = if let Some(field) = field_info {
                        format!("{} ({}){}",
                               suggestion,
                               field.field_type,
                               if field.is_required { " *" } else { "" })
                    } else {
                        suggestion.clone()
                    };

                    // Create spans with match percentage
                    let spans = vec![
                        Span::styled(display_text, Style::default().fg(Color::White)),
                        Span::styled(" ", Style::default()),
                        Span::styled(
                            format!("{:.1}%", match_percentage),
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        ),
                    ];

                    ListItem::new(Line::from(spans))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Suggestions")
                        .borders(Borders::ALL),
                )
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .fg(Color::Yellow),
                );

            f.render_stateful_widget(list, chunks[1], &mut self.fuzzy_popup_state);
        }

        // Controls
        let controls = Paragraph::new("Enter: apply mapping | ↑↓: navigate | Esc: cancel")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(controls, chunks[2]);
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

// UI implementation for ViewCompareApp
impl ViewCompareApp {
    pub fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(2), // Status line
                Constraint::Length(3), // Footer
            ])
            .split(f.area());

        // Header
        let header_text = format!("{} ↔ {} | {} → {}",
            self.source_view_name, self.target_view_name, self.source_env, self.target_env);

        let hide_status = match self.hide_mode {
            HideMode::ShowAll => "",
            HideMode::HideMatches => " (Hiding matches)",
        };

        // Add comparison progress
        let (exact, different, missing, total) = self.calculate_comparison_progress();
        let progress_text = format!(" | ✅{} ⚠️{} ❌{} | {:.1}%",
            exact, different, missing,
            if total > 0 { (exact as f64 / total as f64) * 100.0 } else { 0.0 });

        // Add search mode indicator
        let search_indicator = if self.search_mode {
            format!(" | Search: \"{}\"", self.search_query)
        } else {
            String::new()
        };

        let full_header = format!("{}{}{}{}", header_text, hide_status, progress_text, search_indicator);
        let header = Paragraph::new(full_header)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(header, chunks[0]);

        // Main content - split into two columns
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        // Store areas for mouse handling
        self.source_area = main_chunks[0];
        self.target_area = main_chunks[1];

        // Source tree
        self.render_source_tree_panel(f, main_chunks[0]);

        // Target tree
        self.render_target_tree_panel(f, main_chunks[1]);

        // Status line
        let status_text = self.get_status_line_text();
        let status_line = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(status_line, chunks[2]);

        // Footer
        let footer_text = if self.search_mode {
            "Type to search | Enter/Esc: exit search | ↑↓: navigate matches"
        } else {
            "↑↓: navigate | Tab: switch panels | Space: expand/collapse | h: hide matches | /: search | b/Esc: back to view selection | q: quit"
        };
        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(footer, chunks[3]);
    }

    fn render_source_tree_panel(&mut self, f: &mut Frame, area: Rect) {
        // Extract necessary data first to avoid borrowing conflicts
        let source_view_name = self.source_view_name.clone();
        let is_focused = self.focused_panel == FocusedPanel::Source;

        // Create the items using the static method to avoid borrowing conflicts
        let items = ViewCompareApp::create_tree_items_for(&self.source_tree);
        let visible_count = items.len();
        let selected_idx = self.source_list_state.selected().map(|i| i + 1).unwrap_or(0);

        let title = format!("Source: {} ({}/{})",
                           source_view_name,
                           selected_idx,
                           visible_count);

        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );

        f.render_stateful_widget(list, area, &mut self.source_list_state);
    }

    fn render_target_tree_panel(&mut self, f: &mut Frame, area: Rect) {
        // Extract necessary data first to avoid borrowing conflicts
        let target_view_name = self.target_view_name.clone();
        let is_focused = self.focused_panel == FocusedPanel::Target;

        // Create the items using the static method to avoid borrowing conflicts
        let items = ViewCompareApp::create_tree_items_for(&self.target_tree);
        let visible_count = items.len();
        let selected_idx = self.target_list_state.selected().map(|i| i + 1).unwrap_or(0);

        let title = format!("Target: {} ({}/{})",
                           target_view_name,
                           selected_idx,
                           visible_count);

        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );

        f.render_stateful_widget(list, area, &mut self.target_list_state);
    }

}

pub fn render_tree_node(node: &TreeNode, depth: usize) -> ListItem {
    let indent = "  ".repeat(depth);
    let mut spans = vec![];

    // Add indentation
    if !indent.is_empty() {
        spans.push(Span::styled(indent, Style::default()));
    }

    // Add expand/collapse indicator for expandable nodes
    if node.can_expand() {
        let expand_icon = if node.is_expanded() { "▼ " } else { "▶ " };
        spans.push(Span::styled(expand_icon, Style::default().fg(Color::Gray)));
    } else {
        spans.push(Span::styled("  ", Style::default()));
    }

    // Get base display text
    let display_text = node.get_display_text();

    // Color based on match status
    let (text_color, status_text) = match node.get_match_status() {
        Some(MatchStatus::Exact) => (Color::Green, " ✅"),
        Some(MatchStatus::Different) => (Color::Yellow, " ⚠️"),
        Some(MatchStatus::Missing) => (Color::Red, " ❌"),
        Some(MatchStatus::Added) => (Color::Cyan, " 🆕"),
        None => (Color::White, ""),
    };

    spans.push(Span::styled(display_text, Style::default().fg(text_color)));

    if !status_text.is_empty() {
        spans.push(Span::styled(status_text, Style::default().fg(text_color)));
    }

    ListItem::new(Line::from(spans))
}