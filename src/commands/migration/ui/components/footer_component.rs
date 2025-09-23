use crate::commands::migration::ui::styles::STYLES;
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

pub struct FooterComponent {
    actions: Vec<FooterAction>,
    style: FooterStyle,
}

pub struct FooterAction {
    pub key: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Clone)]
pub enum FooterStyle {
    Standard,
    Compact,
    Help,
}

impl FooterComponent {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            style: FooterStyle::Standard,
        }
    }

    pub fn add_action(mut self, key: &str, description: &str) -> Self {
        self.actions.push(FooterAction {
            key: key.to_string(),
            description: description.to_string(),
            enabled: true,
        });
        self
    }

    pub fn add_action_enabled(mut self, key: &str, description: &str, enabled: bool) -> Self {
        self.actions.push(FooterAction {
            key: key.to_string(),
            description: description.to_string(),
            enabled,
        });
        self
    }

    pub fn add_navigation_actions(mut self) -> Self {
        self.actions.extend([
            FooterAction {
                key: "↑↓".to_string(),
                description: "Navigate".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "Enter".to_string(),
                description: "Select".to_string(),
                enabled: true,
            },
            FooterAction {
                key: "Esc".to_string(),
                description: "Back".to_string(),
                enabled: true,
            },
        ]);
        self
    }

    pub fn add_quit_action(mut self) -> Self {
        self.actions.push(FooterAction {
            key: "Ctrl+Q".to_string(),
            description: "Quit".to_string(),
            enabled: true,
        });
        self
    }

    pub fn with_style(mut self, style: FooterStyle) -> Self {
        self.style = style;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let text = self.build_footer_text();

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(STYLES.footer)
                    .title("Actions"),
            )
            .style(STYLES.footer);

        f.render_widget(paragraph, area);
    }

    fn build_footer_text(&self) -> Text {
        let mut spans = Vec::new();

        for (i, action) in self.actions.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" | ", STYLES.footer));
            }

            let key_style = if action.enabled {
                STYLES.footer_key
            } else {
                STYLES.disabled
            };

            let desc_style = if action.enabled {
                STYLES.footer
            } else {
                STYLES.disabled
            };

            spans.push(Span::styled(&action.key, key_style));
            spans.push(Span::styled(" ", STYLES.footer));
            spans.push(Span::styled(&action.description, desc_style));
        }

        Text::from(Line::from(spans))
    }
}

impl Default for FooterComponent {
    fn default() -> Self {
        Self::new()
    }
}
