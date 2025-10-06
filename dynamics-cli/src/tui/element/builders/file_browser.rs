use crate::tui::Element;
use crate::tui::element::FocusId;
use crate::tui::widgets::FileBrowserEvent;
use std::path::PathBuf;

/// Builder for file browser elements
pub struct FileBrowserBuilder<Msg> {
    pub(crate) id: FocusId,
    pub(crate) current_path: PathBuf,
    pub(crate) entries: Vec<Element<Msg>>,
    pub(crate) selected: Option<usize>,
    pub(crate) scroll_offset: usize,
    pub(crate) on_file_selected: Option<fn(PathBuf) -> Msg>,
    pub(crate) on_directory_changed: Option<fn(PathBuf) -> Msg>,
    pub(crate) on_directory_entered: Option<fn(PathBuf) -> Msg>,
    pub(crate) on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
    pub(crate) on_event: Option<fn(FileBrowserEvent) -> Msg>,
    pub(crate) on_focus: Option<Msg>,
    pub(crate) on_blur: Option<Msg>,
    pub(crate) on_render: Option<fn(usize) -> Msg>,
}

impl<Msg> FileBrowserBuilder<Msg> {
    /// Set callback when file is selected
    pub fn on_file_selected(mut self, msg: fn(PathBuf) -> Msg) -> Self {
        self.on_file_selected = Some(msg);
        self
    }

    /// Set callback when directory path changes
    pub fn on_directory_changed(mut self, msg: fn(PathBuf) -> Msg) -> Self {
        self.on_directory_changed = Some(msg);
        self
    }

    /// Set callback when directory is entered
    pub fn on_directory_entered(mut self, msg: fn(PathBuf) -> Msg) -> Self {
        self.on_directory_entered = Some(msg);
        self
    }

    /// Set callback for navigation keys
    pub fn on_navigate(mut self, msg: fn(crossterm::event::KeyCode) -> Msg) -> Self {
        self.on_navigate = Some(msg);
        self
    }

    /// Set unified event callback
    pub fn on_event(mut self, msg: fn(FileBrowserEvent) -> Msg) -> Self {
        self.on_event = Some(msg);
        self
    }

    pub fn on_focus(mut self, msg: Msg) -> Self {
        self.on_focus = Some(msg);
        self
    }

    pub fn on_blur(mut self, msg: Msg) -> Self {
        self.on_blur = Some(msg);
        self
    }

    pub fn on_render(mut self, msg: fn(usize) -> Msg) -> Self {
        self.on_render = Some(msg);
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::FileBrowser {
            id: self.id,
            current_path: self.current_path,
            entries: self.entries,
            selected: self.selected,
            scroll_offset: self.scroll_offset,
            on_file_selected: self.on_file_selected,
            on_directory_changed: self.on_directory_changed,
            on_directory_entered: self.on_directory_entered,
            on_navigate: self.on_navigate,
            on_event: self.on_event,
            on_focus: self.on_focus,
            on_blur: self.on_blur,
            on_render: self.on_render,
        }
    }
}
