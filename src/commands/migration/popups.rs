use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::config::Config;
use super::app::CompareApp;

impl CompareApp {
    pub fn toggle_mapping_popup(&mut self) {
        self.show_mapping_popup = !self.show_mapping_popup;
        if self.show_mapping_popup && !self.field_mappings.is_empty() {
            self.mapping_popup_state.select(Some(0));
        }
    }

    pub fn delete_selected_mapping(&mut self) {
        if let Some(selected) = self.mapping_popup_state.selected() {
            // Convert HashMap to sorted vector for consistent indexing
            let mut mappings: Vec<(&String, &String)> = self.field_mappings.iter().collect();
            mappings.sort_by_key(|(k, _)| *k);

            if let Some((source_field, _)) = mappings.get(selected) {
                let source_field = (*source_field).clone();

                // Remove from local state
                self.field_mappings.remove(&source_field);

                // Remove from config file
                if let Ok(mut config) = Config::load() {
                    match config.remove_field_mapping(
                        &self.source_entity_name,
                        &self.target_entity_name,
                        &source_field,
                    ) {
                        Ok(_) => {
                            // Successfully removed
                        }
                        Err(e) => {
                            eprintln!("Failed to remove mapping from config: {}", e);
                        }
                    }
                }

                // Adjust selection after deletion
                if self.field_mappings.is_empty() {
                    self.mapping_popup_state.select(None);
                    self.show_mapping_popup = false; // Auto-close if empty
                } else {
                    let new_len = self.field_mappings.len();
                    if selected >= new_len {
                        self.mapping_popup_state.select(Some(new_len - 1));
                    }
                }
            }
        }
    }

    pub fn popup_previous(&mut self) {
        if let Some(selected) = self.mapping_popup_state.selected() {
            if selected > 0 {
                self.mapping_popup_state.select(Some(selected - 1));
            } else {
                let len = self.field_mappings.len();
                if len > 0 {
                    self.mapping_popup_state.select(Some(len - 1));
                }
            }
        }
    }

    pub fn popup_next(&mut self) {
        if let Some(selected) = self.mapping_popup_state.selected() {
            let len = self.field_mappings.len();
            if selected + 1 < len {
                self.mapping_popup_state.select(Some(selected + 1));
            } else {
                self.mapping_popup_state.select(Some(0));
            }
        }
    }

    pub fn toggle_prefix_popup(&mut self) {
        self.show_prefix_popup = !self.show_prefix_popup;
        if self.show_prefix_popup && !self.prefix_mappings.is_empty() {
            self.prefix_popup_state.select(Some(0));
        }
    }

    pub fn delete_selected_prefix_mapping(&mut self) {
        if let Some(selected) = self.prefix_popup_state.selected() {
            // Convert HashMap to sorted vector for consistent indexing
            let mut mappings: Vec<(&String, &String)> = self.prefix_mappings.iter().collect();
            mappings.sort_by_key(|(k, _)| *k);

            if let Some((source_prefix, _)) = mappings.get(selected) {
                let source_prefix = (*source_prefix).clone();

                // Remove from local state
                self.prefix_mappings.remove(&source_prefix);

                // Remove from config file
                if let Ok(mut config) = Config::load() {
                    match config.remove_prefix_mapping(
                        &self.source_entity_name,
                        &self.target_entity_name,
                        &source_prefix,
                    ) {
                        Ok(_) => {
                            // Successfully removed
                        }
                        Err(e) => {
                            eprintln!("Failed to remove prefix mapping from config: {}", e);
                        }
                    }
                }

                // Adjust selection after deletion
                if self.prefix_mappings.is_empty() {
                    self.prefix_popup_state.select(None);
                    self.show_prefix_popup = false; // Auto-close if empty
                } else {
                    let new_len = self.prefix_mappings.len();
                    if selected >= new_len {
                        self.prefix_popup_state.select(Some(new_len - 1));
                    }
                }
            }
        }
    }

    pub fn prefix_popup_previous(&mut self) {
        if let Some(selected) = self.prefix_popup_state.selected() {
            if selected > 0 {
                self.prefix_popup_state.select(Some(selected - 1));
            } else {
                let len = self.prefix_mappings.len();
                if len > 0 {
                    self.prefix_popup_state.select(Some(len - 1));
                }
            }
        }
    }

    pub fn prefix_popup_next(&mut self) {
        if let Some(selected) = self.prefix_popup_state.selected() {
            let len = self.prefix_mappings.len();
            if selected + 1 < len {
                self.prefix_popup_state.select(Some(selected + 1));
            } else {
                self.prefix_popup_state.select(Some(0));
            }
        }
    }

    pub fn show_prefix_input_dialog(&mut self) {
        self.show_prefix_input = true;
        self.prefix_input_source.clear();
        self.prefix_input_target.clear();
        self.prefix_input_field = 0;
    }

    pub fn hide_prefix_input_dialog(&mut self) {
        self.show_prefix_input = false;
        self.prefix_input_source.clear();
        self.prefix_input_target.clear();
        self.prefix_input_field = 0;
    }

    pub fn save_prefix_input(&mut self) {
        if !self.prefix_input_source.is_empty() && !self.prefix_input_target.is_empty() {
            // Add to local state
            self.prefix_mappings.insert(
                self.prefix_input_source.clone(),
                self.prefix_input_target.clone(),
            );

            // Save to config
            if let Ok(mut config) = Config::load() {
                let _ = config.add_prefix_mapping(
                    &self.source_entity_name,
                    &self.target_entity_name,
                    &self.prefix_input_source,
                    &self.prefix_input_target,
                );
            }

            // Update prefix popup state to show the new mapping
            if !self.show_prefix_popup {
                self.show_prefix_popup = true;
            }
            self.prefix_popup_state.select(Some(self.prefix_mappings.len() - 1));

            self.hide_prefix_input_dialog();
        }
    }

    pub fn render_mapping_popup(&mut self, f: &mut Frame) {
        let popup_area = centered_rect(60, 50, f.area());

        // Clear the background
        f.render_widget(Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Manual Field Mappings (delete with 'd', ESC to close)")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        // Get the inner area for the list
        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        if self.field_mappings.is_empty() {
            let no_mappings = Paragraph::new("No manual mappings found.")
                .style(Style::default().fg(Color::Gray));
            f.render_widget(no_mappings, inner_area);
        } else {
            // Convert to sorted vector for consistent display
            let mut mappings: Vec<(&String, &String)> = self.field_mappings.iter().collect();
            mappings.sort_by_key(|(k, _)| *k);

            let items: Vec<ListItem> = mappings
                .iter()
                .map(|(source, target)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(source.as_str(), Style::default().fg(Color::Cyan)),
                        Span::raw(" → "),
                        Span::styled(target.as_str(), Style::default().fg(Color::Green)),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .fg(Color::Yellow),
                );

            f.render_stateful_widget(list, inner_area, &mut self.mapping_popup_state);
        }
    }

    pub fn render_prefix_popup(&mut self, f: &mut Frame) {
        let popup_area = centered_rect(60, 50, f.area());

        // Clear the background
        f.render_widget(Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Prefix Mappings (delete with 'd', add with 'a', ESC to close)")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        // Get the inner area for the list
        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        if self.prefix_mappings.is_empty() {
            let no_mappings = Paragraph::new("No prefix mappings found. Press 'a' to add one.")
                .style(Style::default().fg(Color::Gray));
            f.render_widget(no_mappings, inner_area);
        } else {
            // Convert to sorted vector for consistent display
            let mut mappings: Vec<(&String, &String)> = self.prefix_mappings.iter().collect();
            mappings.sort_by_key(|(k, _)| *k);

            let items: Vec<ListItem> = mappings
                .iter()
                .map(|(source_prefix, target_prefix)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("'{}'", source_prefix), Style::default().fg(Color::Cyan)),
                        Span::raw(" → "),
                        Span::styled(format!("'{}'", target_prefix), Style::default().fg(Color::Green)),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .fg(Color::Yellow),
                );

            f.render_stateful_widget(list, inner_area, &mut self.prefix_popup_state);
        }
    }

    pub fn render_prefix_input_dialog(&mut self, f: &mut Frame) {
        let popup_area = centered_rect(60, 30, f.area());

        // Clear the background
        f.render_widget(Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Add Prefix Mapping")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        // Split the inner area into sections
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
                Constraint::Length(3), // Source prefix input
                Constraint::Length(3), // Target prefix input
                Constraint::Min(1),    // Controls
            ])
            .split(inner_area);

        // Instructions
        let instructions = Paragraph::new("Tab: next field, Enter: save, ESC: cancel")
            .style(Style::default().fg(Color::Gray));
        f.render_widget(instructions, chunks[0]);

        // Source prefix input
        let source_style = if self.prefix_input_field == 0 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let source_input = Paragraph::new(self.prefix_input_source.as_str())
            .block(Block::default().title("Source Prefix").borders(Borders::ALL))
            .style(source_style);
        f.render_widget(source_input, chunks[1]);

        // Target prefix input
        let target_style = if self.prefix_input_field == 1 {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let target_input = Paragraph::new(self.prefix_input_target.as_str())
            .block(Block::default().title("Target Prefix").borders(Borders::ALL))
            .style(target_style);
        f.render_widget(target_input, chunks[2]);

        // Example
        if !self.prefix_input_source.is_empty() && !self.prefix_input_target.is_empty() {
            let example = format!(
                "Example: '{}fieldname' ↔ '{}fieldname'",
                self.prefix_input_source, self.prefix_input_target
            );
            let example_widget = Paragraph::new(example)
                .style(Style::default().fg(Color::Green));
            f.render_widget(example_widget, chunks[3]);
        }
    }
}

// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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