use crate::tui::Element;

/// Builder for progress bar elements
pub struct ProgressBarBuilder<Msg> {
    pub(crate) current: usize,
    pub(crate) total: usize,
    pub(crate) label: Option<String>,
    pub(crate) show_percentage: bool,
    pub(crate) show_count: bool,
    pub(crate) width: Option<u16>,
    pub(crate) _phantom: std::marker::PhantomData<Msg>,
}

impl<Msg> ProgressBarBuilder<Msg> {
    /// Set an optional label to show before the progress bar
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show percentage (e.g., "42%") - default: true
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Show count (e.g., "23/47") - default: true
    pub fn show_count(mut self, show: bool) -> Self {
        self.show_count = show;
        self
    }

    /// Set a fixed width for the bar portion (default: auto-fill available space)
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::ProgressBar {
            current: self.current,
            total: self.total,
            label: self.label,
            show_percentage: self.show_percentage,
            show_count: self.show_count,
            width: self.width,
        }
    }
}
