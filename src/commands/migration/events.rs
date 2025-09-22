use anyhow::Result;
use crossterm::event::{Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::{backend::Backend, Terminal};

use super::app::{CompareApp, ViewCompareApp};

impl CompareApp {
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = crossterm::event::read()? {
                if self.show_prefix_input {
                    self.handle_prefix_input_events(key.code)?;
                } else if self.show_copy_mappings_popup {
                    self.handle_copy_mappings_popup_events(key.code)?;
                } else if self.show_fuzzy_popup {
                    self.handle_fuzzy_popup_events(key.code)?;
                } else if self.show_mapping_popup {
                    self.handle_mapping_popup_events(key.code)?;
                } else if self.show_prefix_popup {
                    self.handle_prefix_popup_events(key.code)?;
                } else {
                    self.handle_main_events(key.code)?;
                }
            } else if let Event::Mouse(mouse) = crossterm::event::read()? {
                if !self.show_mapping_popup && !self.show_prefix_popup && !self.show_prefix_input && !self.show_copy_mappings_popup && !self.show_fuzzy_popup {
                    self.handle_mouse_events(mouse)?;
                }
            }

            if self.quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_main_events(&mut self, key_code: KeyCode) -> Result<()> {
        // Handle search mode input first
        if self.search_mode {
            match key_code {
                KeyCode::Char(c) => {
                    self.add_to_search_query(c);
                    return Ok(());
                }
                KeyCode::Backspace => {
                    self.remove_from_search_query();
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.toggle_search_mode(); // Exit search mode
                    return Ok(());
                }
                KeyCode::Enter => {
                    self.toggle_search_mode(); // Exit search mode
                    return Ok(());
                }
                KeyCode::Down => self.next(),
                KeyCode::Up => self.previous(),
                _ => {}
            }
            return Ok(());
        }

        // Normal mode input handling
        match key_code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
            KeyCode::Tab => {
                self.focused_panel = match self.focused_panel {
                    super::app::FocusedPanel::Source => super::app::FocusedPanel::Target,
                    super::app::FocusedPanel::Target => super::app::FocusedPanel::Source,
                };
            }
            KeyCode::Char('h') => self.cycle_hide_mode(),
            KeyCode::Char('m') => self.create_manual_mapping(),
            KeyCode::Char('M') => self.toggle_mapping_popup(),
            KeyCode::Char('P') => self.toggle_prefix_popup(),
            KeyCode::Char('c') => self.toggle_copy_mappings_popup(),
            // New keybindings
            KeyCode::Char('/') => self.toggle_search_mode(),
            KeyCode::Char('u') => self.undo_last_mapping(),
            KeyCode::Char('f') => self.start_fuzzy_mapping(),
            KeyCode::Char('e') => self.export_to_excel_tui(),
            KeyCode::F(1) => self.copy_field_name_to_clipboard(),
            _ => {}
        }
        Ok(())
    }

    fn handle_mapping_popup_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => self.toggle_mapping_popup(),
            KeyCode::Down => self.popup_next(),
            KeyCode::Up => self.popup_previous(),
            KeyCode::Char('d') => self.delete_selected_mapping(),
            _ => {}
        }
        Ok(())
    }

    fn handle_prefix_popup_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => self.toggle_prefix_popup(),
            KeyCode::Down => self.prefix_popup_next(),
            KeyCode::Up => self.prefix_popup_previous(),
            KeyCode::Char('d') => self.delete_selected_prefix_mapping(),
            KeyCode::Char('a') => self.show_prefix_input_dialog(),
            _ => {}
        }
        Ok(())
    }

    fn handle_prefix_input_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => self.hide_prefix_input_dialog(),
            KeyCode::Enter => self.save_prefix_input(),
            KeyCode::Tab => {
                self.prefix_input_field = (self.prefix_input_field + 1) % 2;
            }
            KeyCode::Backspace => {
                if self.prefix_input_field == 0 {
                    self.prefix_input_source.pop();
                } else {
                    self.prefix_input_target.pop();
                }
            }
            KeyCode::Char(c) => {
                if self.prefix_input_field == 0 {
                    self.prefix_input_source.push(c);
                } else {
                    self.prefix_input_target.push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_mouse_events(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is in source area
                if mouse.column >= self.source_area.x
                    && mouse.column < self.source_area.x + self.source_area.width
                    && mouse.row >= self.source_area.y
                    && mouse.row < self.source_area.y + self.source_area.height
                {
                    self.focused_panel = super::app::FocusedPanel::Source;
                    // Calculate which item was clicked (accounting for header)
                    if mouse.row >= self.source_area.y + 2 {
                        let item_index = (mouse.row - self.source_area.y - 2) as usize;
                        let source_fields = self.get_filtered_source_fields();
                        if item_index < source_fields.len() {
                            self.source_list_state.select(Some(item_index));
                        }
                    }
                }
                // Check if click is in target area
                else if mouse.column >= self.target_area.x
                    && mouse.column < self.target_area.x + self.target_area.width
                    && mouse.row >= self.target_area.y
                    && mouse.row < self.target_area.y + self.target_area.height
                {
                    self.focused_panel = super::app::FocusedPanel::Target;
                    // Calculate which item was clicked (accounting for header)
                    if mouse.row >= self.target_area.y + 2 {
                        let item_index = (mouse.row - self.target_area.y - 2) as usize;
                        let target_fields = self.get_filtered_target_fields();
                        if item_index < target_fields.len() {
                            self.target_list_state.select(Some(item_index));
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if mouse.column >= self.source_area.x
                    && mouse.column < self.source_area.x + self.source_area.width
                    && mouse.row >= self.source_area.y
                    && mouse.row < self.source_area.y + self.source_area.height
                {
                    self.focused_panel = super::app::FocusedPanel::Source;
                    self.previous();
                } else if mouse.column >= self.target_area.x
                    && mouse.column < self.target_area.x + self.target_area.width
                    && mouse.row >= self.target_area.y
                    && mouse.row < self.target_area.y + self.target_area.height
                {
                    self.focused_panel = super::app::FocusedPanel::Target;
                    self.previous();
                }
            }
            MouseEventKind::ScrollDown => {
                if mouse.column >= self.source_area.x
                    && mouse.column < self.source_area.x + self.source_area.width
                    && mouse.row >= self.source_area.y
                    && mouse.row < self.source_area.y + self.source_area.height
                {
                    self.focused_panel = super::app::FocusedPanel::Source;
                    self.next();
                } else if mouse.column >= self.target_area.x
                    && mouse.column < self.target_area.x + self.target_area.width
                    && mouse.row >= self.target_area.y
                    && mouse.row < self.target_area.y + self.target_area.height
                {
                    self.focused_panel = super::app::FocusedPanel::Target;
                    self.next();
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn next(&mut self) {
        match self.focused_panel {
            super::app::FocusedPanel::Source => {
                let source_fields = self.get_filtered_source_fields();
                if !source_fields.is_empty() {
                    let i = match self.source_list_state.selected() {
                        Some(i) => {
                            if i >= source_fields.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.source_list_state.select(Some(i));
                }
            }
            super::app::FocusedPanel::Target => {
                let target_fields = self.get_filtered_target_fields();
                if !target_fields.is_empty() {
                    let i = match self.target_list_state.selected() {
                        Some(i) => {
                            if i >= target_fields.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.target_list_state.select(Some(i));
                }
            }
        }
    }

    pub fn previous(&mut self) {
        match self.focused_panel {
            super::app::FocusedPanel::Source => {
                let source_fields = self.get_filtered_source_fields();
                if !source_fields.is_empty() {
                    let i = match self.source_list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                source_fields.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.source_list_state.select(Some(i));
                }
            }
            super::app::FocusedPanel::Target => {
                let target_fields = self.get_filtered_target_fields();
                if !target_fields.is_empty() {
                    let i = match self.target_list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                target_fields.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.target_list_state.select(Some(i));
                }
            }
        }
    }

    fn toggle_copy_mappings_popup(&mut self) {
        self.show_copy_mappings_popup = !self.show_copy_mappings_popup;
        if self.show_copy_mappings_popup {
            self.copy_mappings_state.select(if self.available_comparisons.is_empty() { None } else { Some(0) });
        }
    }

    fn handle_copy_mappings_popup_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => {
                self.show_copy_mappings_popup = false;
            }
            KeyCode::Down => {
                if !self.available_comparisons.is_empty() {
                    let i = match self.copy_mappings_state.selected() {
                        Some(i) => {
                            if i >= self.available_comparisons.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.copy_mappings_state.select(Some(i));
                }
            }
            KeyCode::Up => {
                if !self.available_comparisons.is_empty() {
                    let i = match self.copy_mappings_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.available_comparisons.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.copy_mappings_state.select(Some(i));
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.copy_mappings_state.selected() {
                    if selected < self.available_comparisons.len() {
                        let source_comparison = self.available_comparisons[selected].clone();
                        // We need to handle the config loading here since we need a mutable reference
                        if let Ok(mut config) = crate::config::Config::load() {
                            if let Ok((_field_count, _prefix_count)) = self.copy_mappings_from(&source_comparison, &mut config) {
                                // Successfully copied - close popup
                                self.show_copy_mappings_popup = false;
                                // Note: In a real implementation, you might want to show a success message
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_fuzzy_popup_events(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => self.cancel_fuzzy_mapping(),
            KeyCode::Down => {
                if !self.fuzzy_suggestions.is_empty() {
                    let i = match self.fuzzy_popup_state.selected() {
                        Some(i) => {
                            if i >= self.fuzzy_suggestions.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.fuzzy_popup_state.select(Some(i));
                }
            }
            KeyCode::Up => {
                if !self.fuzzy_suggestions.is_empty() {
                    let i = match self.fuzzy_popup_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                self.fuzzy_suggestions.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.fuzzy_popup_state.select(Some(i));
                }
            }
            KeyCode::Enter => self.apply_fuzzy_mapping(),
            _ => {}
        }
        Ok(())
    }
}

// Event handling for ViewCompareApp
impl ViewCompareApp {
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<super::app::ViewCompareResult> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = crossterm::event::read()? {
                self.handle_main_events(key.code)?;
            } else if let Event::Mouse(mouse) = crossterm::event::read()? {
                self.handle_mouse_events(mouse)?;
            }

            if self.quit {
                break;
            }
        }

        // Return the result based on what the user chose
        Ok(self.result.take().unwrap_or(super::app::ViewCompareResult::Exit))
    }

    fn handle_main_events(&mut self, key_code: KeyCode) -> Result<()> {
        // Handle search mode input first
        if self.search_mode {
            match key_code {
                KeyCode::Char(c) => {
                    self.add_to_search_query(c);
                    return Ok(());
                }
                KeyCode::Backspace => {
                    self.remove_from_search_query();
                    return Ok(());
                }
                KeyCode::Esc => {
                    self.toggle_search_mode(); // Exit search mode
                    return Ok(());
                }
                KeyCode::Enter => {
                    self.toggle_search_mode(); // Exit search mode
                    return Ok(());
                }
                KeyCode::Down => self.next(),
                KeyCode::Up => self.previous(),
                _ => {}
            }
            return Ok(());
        }

        // Normal mode input handling
        match key_code {
            KeyCode::Char('q') => {
                self.result = Some(super::app::ViewCompareResult::Exit);
                self.quit = true;
            }
            KeyCode::Char('b') | KeyCode::Esc => {
                self.result = Some(super::app::ViewCompareResult::BackToViewSelection);
                self.quit = true;
            }
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
            KeyCode::Tab => {
                self.focused_panel = match self.focused_panel {
                    super::app::FocusedPanel::Source => super::app::FocusedPanel::Target,
                    super::app::FocusedPanel::Target => super::app::FocusedPanel::Source,
                };
            }
            KeyCode::Char('h') => self.cycle_hide_mode(),
            KeyCode::Char(' ') => self.toggle_expand(),
            KeyCode::Char('/') => self.toggle_search_mode(),
            _ => {}
        }
        Ok(())
    }

    fn handle_mouse_events(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is in source area
                if mouse.column >= self.source_area.x
                    && mouse.column < self.source_area.x + self.source_area.width
                    && mouse.row >= self.source_area.y + 1 // +1 to account for border
                    && mouse.row < self.source_area.y + self.source_area.height - 1
                {
                    // Switch to source panel
                    self.focused_panel = super::app::FocusedPanel::Source;

                    // Calculate which view was clicked
                    let clicked_row = (mouse.row - self.source_area.y - 1) as usize;
                    let source_nodes = self.get_visible_source_nodes();
                    if clicked_row < source_nodes.len() {
                        self.source_list_state.select(Some(clicked_row));
                    }
                }
                // Check if click is in target area
                else if mouse.column >= self.target_area.x
                    && mouse.column < self.target_area.x + self.target_area.width
                    && mouse.row >= self.target_area.y + 1 // +1 to account for border
                    && mouse.row < self.target_area.y + self.target_area.height - 1
                {
                    // Switch to target panel
                    self.focused_panel = super::app::FocusedPanel::Target;

                    // Calculate which view was clicked
                    let clicked_row = (mouse.row - self.target_area.y - 1) as usize;
                    let target_nodes = self.get_visible_target_nodes();
                    if clicked_row < target_nodes.len() {
                        self.target_list_state.select(Some(clicked_row));
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                self.previous();
            }
            MouseEventKind::ScrollDown => {
                self.next();
            }
            _ => {}
        }
        Ok(())
    }
}