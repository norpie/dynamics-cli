use crate::commands::migration::ui::components::{
    list_component::{ListAction, ListComponent},
    modal_component::{ModalContent, ModalContentAction},
};
use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ManualMappingAction {
    Delete(String), // source field name to delete
}

pub struct ManualMappingModal {
    field_mappings: HashMap<String, String>, // source -> target field name
    list: ListComponent<String>,
    current_action: Option<ManualMappingAction>,
}

impl ManualMappingModal {
    pub fn new(field_mappings: HashMap<String, String>) -> Self {
        let items: Vec<String> = field_mappings
            .iter()
            .map(|(source, target)| format!("{} → {}", source, target))
            .collect();

        let mut list = ListComponent::new(items);
        list = list.with_title("Manual Field Mappings".to_string());

        Self {
            field_mappings,
            list,
            current_action: None,
        }
    }

    pub fn take_action(&mut self) -> Option<ManualMappingAction> {
        self.current_action.take()
    }

    fn delete_selected(&mut self) {
        if let Some(selected_index) = self.list.selected()
            && let Some(mapping_entry) = self.list.items().get(selected_index)
        {
            // Parse the "source → target" format to get source field name
            if let Some(arrow_pos) = mapping_entry.find(" → ") {
                let source_field = mapping_entry[..arrow_pos].to_string();

                // Remove from internal mappings
                self.field_mappings.remove(&source_field);

                // Update the list items
                self.refresh_list();

                // Set the action for the parent to handle persistence
                self.current_action = Some(ManualMappingAction::Delete(source_field));
            }
        }
    }

    fn refresh_list(&mut self) {
        let items: Vec<String> = self
            .field_mappings
            .iter()
            .map(|(source, target)| format!("{} → {}", source, target))
            .collect();

        self.list.update_items(items);
    }

    fn get_help_text(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(vec![
                Span::styled("Delete: ", Style::default().fg(Color::Yellow)),
                Span::styled("d", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Close: ", Style::default().fg(Color::Yellow)),
                Span::styled("Esc", Style::default().fg(Color::Cyan)),
            ]),
        ]
    }
}

impl ModalContent for ManualMappingModal {
    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
            .split(area);

        // Render the list
        self.list.render(f, chunks[0]);

        // Render help text
        let help_text = self.get_help_text();
        let help_paragraph = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::TOP).title("Help"))
            .wrap(Wrap { trim: true });
        f.render_widget(help_paragraph, chunks[1]);
    }

    fn handle_key(&mut self, key: KeyCode) -> ModalContentAction {
        match key {
            KeyCode::Esc => ModalContentAction::Close,
            KeyCode::Char('d') | KeyCode::Delete => {
                self.delete_selected();
                ModalContentAction::Custom("delete".to_string())
            }
            _ => {
                // Handle list navigation
                match self.list.handle_key(key) {
                    ListAction::None => ModalContentAction::Custom("continue".to_string()),
                    _ => ModalContentAction::Custom("continue".to_string()),
                }
            }
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> ModalContentAction {
        match self.list.handle_mouse(event, area) {
            ListAction::None => ModalContentAction::Custom("continue".to_string()),
            _ => ModalContentAction::Custom("continue".to_string()),
        }
    }
}
