use ratatui::style::{Color, Modifier, Style};

pub struct AppStyles {
    pub normal: Style,
    pub selected: Style,
    pub highlighted: Style,
    pub hover: Style,
    pub active: Style,
    pub disabled: Style,
    pub error: Style,
    pub success: Style,
    pub warning: Style,
    pub info: Style,
    pub breadcrumb: Style,
    pub breadcrumb_separator: Style,
    pub footer: Style,
    pub footer_key: Style,
    pub modal_border: Style,
    pub modal_title: Style,
}

impl Default for AppStyles {
    fn default() -> Self {
        Self {
            normal: Style::default().fg(Color::White),
            selected: Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
            highlighted: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            hover: Style::default().fg(Color::White).bg(Color::DarkGray),
            active: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            disabled: Style::default().fg(Color::DarkGray),
            error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            success: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            warning: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            info: Style::default().fg(Color::Cyan),
            breadcrumb: Style::default().fg(Color::Gray),
            breadcrumb_separator: Style::default().fg(Color::DarkGray),
            footer: Style::default().fg(Color::Gray),
            footer_key: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            modal_border: Style::default().fg(Color::White),
            modal_title: Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}

pub static STYLES: AppStyles = AppStyles {
    normal: Style {
        fg: Some(Color::White),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
    selected: Style {
        fg: Some(Color::Black),
        bg: Some(Color::White),
        underline_color: None,
        add_modifier: Modifier::BOLD,
        sub_modifier: Modifier::empty(),
    },
    highlighted: Style {
        fg: Some(Color::Yellow),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::BOLD,
        sub_modifier: Modifier::empty(),
    },
    hover: Style {
        fg: Some(Color::White),
        bg: Some(Color::DarkGray),
        underline_color: None,
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
    active: Style {
        fg: Some(Color::Green),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::BOLD,
        sub_modifier: Modifier::empty(),
    },
    disabled: Style {
        fg: Some(Color::DarkGray),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
    error: Style {
        fg: Some(Color::Red),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::BOLD,
        sub_modifier: Modifier::empty(),
    },
    success: Style {
        fg: Some(Color::Green),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::BOLD,
        sub_modifier: Modifier::empty(),
    },
    warning: Style {
        fg: Some(Color::Yellow),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::BOLD,
        sub_modifier: Modifier::empty(),
    },
    info: Style {
        fg: Some(Color::Cyan),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
    breadcrumb: Style {
        fg: Some(Color::Gray),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
    breadcrumb_separator: Style {
        fg: Some(Color::DarkGray),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
    footer: Style {
        fg: Some(Color::Gray),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
    footer_key: Style {
        fg: Some(Color::Yellow),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::BOLD,
        sub_modifier: Modifier::empty(),
    },
    modal_border: Style {
        fg: Some(Color::White),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
    modal_title: Style {
        fg: Some(Color::White),
        bg: None,
        underline_color: None,
        add_modifier: Modifier::BOLD,
        sub_modifier: Modifier::empty(),
    },
};
