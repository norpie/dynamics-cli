//! Settings TUI application for managing options

use crate::config::options::{OptionDefinition, OptionType, OptionValue};
use crate::tui::{
    app::App,
    command::Command,
    element::{Element, FocusId, ColumnBuilder, RowBuilder},
    state::{config::RuntimeConfig, theme::Theme},
    subscription::Subscription,
    widgets::list::{ListItem, ListState},
    LayeredView, LayoutConstraint,
};
use crate::{col, row, use_constraints, spacer};
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;

#[derive(Debug, Clone)]
pub struct State {
    // Navigation
    namespaces: Vec<String>,
    namespace_list_state: ListState,
    selected_namespace: usize,

    // Options for current namespace
    current_options: Vec<OptionDefinition>,
    option_list_state: ListState,
    selected_option: usize,

    // Current values
    values: std::collections::HashMap<String, OptionValue>,

    // Editing state
    editing: Option<EditingState>,
    edit_input_state: crate::tui::widgets::TextInputState,
    edit_select_state: crate::tui::widgets::SelectState,
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum EditingState {
    TextInput {
        key: String,
        input: String,
    },
    Select {
        key: String,
        options: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Navigation
    SelectNamespace(usize),
    SelectOption(usize),
    NamespaceListNavigate(crossterm::event::KeyCode),
    OptionListNavigate(crossterm::event::KeyCode),

    // Value loading
    ValuesLoaded(Result<std::collections::HashMap<String, OptionValue>, String>),

    // Editing
    StartEdit,
    EditInputEvent(crate::tui::widgets::TextInputEvent),
    EditSelectEvent(crate::tui::widgets::SelectEvent),
    SaveValue,
    CancelEdit,
    ValueSaved(Result<(), String>),

    // Toggle boolean
    ToggleBool(String),
    BoolToggled(Result<(), String>),

    // Runtime config reload
    ConfigReloaded(Result<(), String>),
}

impl Default for State {
    fn default() -> Self {
        Self {
            namespaces: Vec::new(),
            namespace_list_state: ListState::with_selection(),
            selected_namespace: 0,
            current_options: Vec::new(),
            option_list_state: ListState::with_selection(),
            selected_option: 0,
            values: std::collections::HashMap::new(),
            editing: None,
            edit_input_state: crate::tui::widgets::TextInputState::new(),
            edit_select_state: crate::tui::widgets::SelectState::new(),
            error: None,
        }
    }
}

impl crate::tui::AppState for State {}

impl ListItem for String {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Msg> {
        let (fg_color, bg_style) = if is_selected {
            (theme.lavender, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, None)
        };

        let mut builder = Element::styled_text(Line::from(vec![
            Span::styled(format!("  {}", self), Style::default().fg(fg_color)),
        ]));

        if let Some(bg) = bg_style {
            builder = builder.background(bg);
        }

        builder.build()
    }
}

// Wrapper to display option with its current value
#[derive(Clone)]
struct OptionWithValue {
    definition: OptionDefinition,
    value: OptionValue,
    max_name_width: usize,
}

impl ListItem for OptionWithValue {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Msg> {
        let (name_color, value_color, bg_style) = if is_selected {
            (theme.lavender, theme.mauve, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, theme.subtext1, None)
        };

        // Format the value based on type
        let value_str = match &self.value {
            OptionValue::Bool(v) => v.to_string(),
            OptionValue::Int(v) => v.to_string(),
            OptionValue::UInt(v) => v.to_string(),
            OptionValue::Float(v) => format!("{:.2}", v),
            OptionValue::String(v) => format!("\"{}\"", v),
        };

        // Pad the name to align values in a column
        let padded_name = format!("  {:width$}", self.definition.display_name, width = self.max_name_width + 2);

        let mut builder = Element::styled_text(Line::from(vec![
            Span::styled(padded_name, Style::default().fg(name_color)),
            Span::styled(value_str, Style::default().fg(value_color)),
        ]));

        if let Some(bg) = bg_style {
            builder = builder.background(bg);
        }

        builder.build()
    }
}

impl App for SettingsApp {
    type State = State;
    type Msg = Msg;
    type InitParams = ();

    fn init(_params: ()) -> (State, Command<Msg>) {
        let mut state = State::default();

        // Load namespaces
        let registry = crate::options_registry();
        state.namespaces = registry.namespaces();

        // Load options for first namespace
        if !state.namespaces.is_empty() {
            state.current_options = registry.list_namespace(&state.namespaces[0]);
        }

        // Load current values
        let cmd = Command::batch(vec![
            Command::perform(
                async {
                    let config = crate::global_config();
                    let mut values = std::collections::HashMap::new();

                    for def in crate::options_registry().list_all() {
                        if let Ok(value) = config.options.get(&def.key).await {
                            values.insert(def.key.clone(), value);
                        }
                    }

                    Ok(values)
                },
                Msg::ValuesLoaded,
            ),
            Command::set_focus(FocusId::new("namespace-list")),
        ]);

        (state, cmd)
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::SelectNamespace(idx) => {
                if idx < state.namespaces.len() {
                    state.selected_namespace = idx;
                    state.selected_option = 0;

                    // Load options for this namespace
                    let registry = crate::options_registry();
                    state.current_options = registry.list_namespace(&state.namespaces[idx]);

                    // Focus the options list after selecting a category
                    return Command::set_focus(FocusId::new("option-list"));
                }
                Command::None
            }

            Msg::SelectOption(idx) => {
                if idx < state.current_options.len() {
                    state.selected_option = idx;

                    // Start editing this option (inline StartEdit logic)
                    if let Some(opt) = state.current_options.get(idx) {
                        match &opt.ty {
                            OptionType::Bool => {
                                // For bools, toggle immediately
                                let value = state
                                    .values
                                    .get(&opt.key)
                                    .unwrap_or(&opt.default)
                                    .as_bool()
                                    .unwrap_or(false);
                                let key = opt.key.clone();
                                return Command::perform(
                                    async move {
                                        crate::global_config()
                                            .options
                                            .set_bool(&key, !value)
                                            .await
                                            .map_err(|e| e.to_string())
                                    },
                                    Msg::BoolToggled,
                                );
                            }
                            OptionType::Enum { variants } => {
                                // Use select widget for enums
                                state.editing = Some(EditingState::Select {
                                    key: opt.key.clone(),
                                    options: variants.clone(),
                                });
                                // Reset the select state and set the current value as selected
                                let current_value = state.values.get(&opt.key).unwrap_or(&opt.default).as_string().unwrap_or_default();
                                if let Some(index) = variants.iter().position(|v| v == &current_value) {
                                    state.edit_select_state = crate::tui::widgets::SelectState::with_selected(index);
                                } else {
                                    state.edit_select_state = crate::tui::widgets::SelectState::new();
                                }
                                return Command::set_focus(FocusId::new("edit-select"));
                            }
                            OptionType::UInt { .. } | OptionType::Int { .. } | OptionType::Float { .. } | OptionType::String { .. } => {
                                // Start text input for other types
                                let current = state
                                    .values
                                    .get(&opt.key)
                                    .unwrap_or(&opt.default)
                                    .clone();
                                let input_str = match current {
                                    OptionValue::UInt(v) => v.to_string(),
                                    OptionValue::Int(v) => v.to_string(),
                                    OptionValue::Float(v) => v.to_string(),
                                    OptionValue::String(v) => v,
                                    _ => String::new(),
                                };
                                state.editing = Some(EditingState::TextInput {
                                    key: opt.key.clone(),
                                    input: input_str,
                                });
                                // Reset the text input state for fresh editing
                                state.edit_input_state = crate::tui::widgets::TextInputState::new();
                                return Command::set_focus(FocusId::new("edit-input"));
                            }
                        }
                    }
                }
                Command::None
            }

            Msg::NamespaceListNavigate(key) => {
                let visible_height = 20;
                state.namespace_list_state.handle_key(key, state.namespaces.len(), visible_height);

                // Sync the selected namespace index with the list state
                if let Some(selected) = state.namespace_list_state.selected() {
                    if selected != state.selected_namespace && selected < state.namespaces.len() {
                        state.selected_namespace = selected;
                        state.selected_option = 0;

                        // Load options for this namespace
                        let registry = crate::options_registry();
                        state.current_options = registry.list_namespace(&state.namespaces[selected]);
                    }
                }

                Command::None
            }

            Msg::OptionListNavigate(key) => {
                let visible_height = 20;
                state.option_list_state.handle_key(key, state.current_options.len(), visible_height);

                // Sync the selected option index with the list state
                if let Some(selected) = state.option_list_state.selected() {
                    state.selected_option = selected;
                }

                Command::None
            }

            Msg::ValuesLoaded(Ok(values)) => {
                state.values = values;
                Command::None
            }

            Msg::ValuesLoaded(Err(e)) => {
                state.error = Some(e);
                Command::None
            }

            Msg::StartEdit => {
                if let Some(opt) = state.current_options.get(state.selected_option) {
                    match &opt.ty {
                        OptionType::Bool => {
                            // For bools, toggle immediately
                            let value = state
                                .values
                                .get(&opt.key)
                                .unwrap_or(&opt.default)
                                .as_bool()
                                .unwrap_or(false);
                            let key = opt.key.clone();
                            return Command::perform(
                                async move {
                                    crate::global_config()
                                        .options
                                        .set_bool(&key, !value)
                                        .await
                                        .map_err(|e| e.to_string())
                                },
                                Msg::BoolToggled,
                            );
                        }
                        OptionType::Enum { variants } => {
                            // Use select widget for enums
                            state.editing = Some(EditingState::Select {
                                key: opt.key.clone(),
                                options: variants.clone(),
                            });
                            // Reset the select state and set the current value as selected
                            let current_value = state.values.get(&opt.key).unwrap_or(&opt.default).as_string().unwrap_or_default();
                            if let Some(index) = variants.iter().position(|v| v == &current_value) {
                                state.edit_select_state = crate::tui::widgets::SelectState::with_selected(index);
                            } else {
                                state.edit_select_state = crate::tui::widgets::SelectState::new();
                            }
                            return Command::set_focus(FocusId::new("edit-select"));
                        }
                        OptionType::UInt { .. } | OptionType::Int { .. } | OptionType::Float { .. } | OptionType::String { .. } => {
                            // Start text input for other types
                            let current = state
                                .values
                                .get(&opt.key)
                                .unwrap_or(&opt.default)
                                .clone();
                            let input_str = match current {
                                OptionValue::UInt(v) => v.to_string(),
                                OptionValue::Int(v) => v.to_string(),
                                OptionValue::Float(v) => v.to_string(),
                                OptionValue::String(v) => v,
                                _ => String::new(),
                            };
                            state.editing = Some(EditingState::TextInput {
                                key: opt.key.clone(),
                                input: input_str,
                            });
                            // Reset the text input state for fresh editing
                            state.edit_input_state = crate::tui::widgets::TextInputState::new();
                            return Command::set_focus(FocusId::new("edit-input"));
                        }
                    }
                }
                Command::None
            }

            Msg::ToggleBool(key) => {
                let value = state
                    .values
                    .get(&key)
                    .and_then(|v| v.as_bool().ok())
                    .unwrap_or(false);

                Command::perform(
                    async move {
                        crate::global_config()
                            .options
                            .set_bool(&key, !value)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Msg::BoolToggled,
                )
            }

            Msg::BoolToggled(Ok(())) => {
                // Reload values and runtime config
                Command::batch(vec![
                    Command::perform(
                        async {
                            let config = crate::global_config();
                            let mut values = std::collections::HashMap::new();

                            for def in crate::options_registry().list_all() {
                                if let Ok(value) = config.options.get(&def.key).await {
                                    values.insert(def.key.clone(), value);
                                }
                            }

                            Ok(values)
                        },
                        Msg::ValuesLoaded,
                    ),
                    Command::perform(
                        async {
                            let new_config = crate::tui::state::RuntimeConfig::load_from_options().await
                                .map_err(|e| e.to_string())?;
                            crate::reload_runtime_config(new_config);
                            Ok(())
                        },
                        Msg::ConfigReloaded,
                    ),
                ])
            }

            Msg::BoolToggled(Err(e)) => {
                state.error = Some(e);
                Command::None
            }

            Msg::EditInputEvent(event) => {
                use crate::tui::widgets::TextInputEvent;

                match event {
                    TextInputEvent::Submit => {
                        // Same as SaveValue
                        if let Some(EditingState::TextInput { key, input }) = &state.editing {
                            let key = key.clone();
                            let input = input.clone();

                            let opt = state.current_options.iter().find(|o| o.key == key).cloned();
                            if let Some(opt) = opt {
                                state.editing = None;

                                return Command::perform(
                                    async move {
                                        let value = match opt.ty {
                                            OptionType::UInt { .. } => {
                                                let parsed = input.parse::<u64>()
                                                    .map_err(|e| format!("Invalid number: {}", e))?;
                                                OptionValue::UInt(parsed)
                                            }
                                            OptionType::Int { .. } => {
                                                let parsed = input.parse::<i64>()
                                                    .map_err(|e| format!("Invalid number: {}", e))?;
                                                OptionValue::Int(parsed)
                                            }
                                            OptionType::Float { .. } => {
                                                let parsed = input.parse::<f64>()
                                                    .map_err(|e| format!("Invalid number: {}", e))?;
                                                OptionValue::Float(parsed)
                                            }
                                            OptionType::String { .. } | OptionType::Enum { .. } => {
                                                OptionValue::String(input)
                                            }
                                            _ => return Err("Unsupported type".to_string()),
                                        };

                                        crate::global_config()
                                            .options
                                            .set(&key, value)
                                            .await
                                            .map_err(|e| e.to_string())
                                    },
                                    Msg::ValueSaved,
                                );
                            }
                        }
                        Command::None
                    }
                    TextInputEvent::Changed(key_code) => {
                        if let Some(EditingState::TextInput { input, .. }) = &mut state.editing {
                            // Use TextInputState's handle_key to process the key and get new value
                            if let Some(new_value) = state.edit_input_state.handle_key(key_code, input, None) {
                                *input = new_value;
                            }
                        }
                        Command::None
                    }
                }
            }

            Msg::EditSelectEvent(event) => {
                use crate::tui::widgets::SelectEvent;
                use crossterm::event::KeyCode;

                // Let SelectState handle the event
                if let Some(EditingState::Select { key, options }) = &state.editing {
                    // Update option count before handling event
                    state.edit_select_state.update_option_count(options.len());

                    // Handle Enter key to toggle when closed
                    match event {
                        SelectEvent::Navigate(KeyCode::Enter) if !state.edit_select_state.is_open() => {
                            // Toggle open when closed
                            state.edit_select_state.toggle();
                            return Command::None;
                        }
                        SelectEvent::Navigate(KeyCode::Enter) if state.edit_select_state.is_open() => {
                            // Select highlighted item when open
                            state.edit_select_state.select_highlighted();
                            let selected_idx = state.edit_select_state.selected();

                            if let Some(selected_value) = options.get(selected_idx) {
                                let key = key.clone();
                                let value = selected_value.clone();
                                state.editing = None;

                                return Command::perform(
                                    async move {
                                        crate::global_config()
                                            .options
                                            .set(&key, OptionValue::String(value))
                                            .await
                                            .map_err(|e| e.to_string())
                                    },
                                    Msg::ValueSaved,
                                );
                            }
                        }
                        SelectEvent::Navigate(KeyCode::Esc) => {
                            // Close dropdown or cancel editing
                            if state.edit_select_state.is_open() {
                                state.edit_select_state.close();
                            } else {
                                state.editing = None;
                            }
                            return Command::None;
                        }
                        _ => {
                            // Handle other navigation events
                            if let Some(selected_idx) = state.edit_select_state.handle_event(event) {
                                // A value was selected via click
                                if let Some(selected_value) = options.get(selected_idx) {
                                    let key = key.clone();
                                    let value = selected_value.clone();
                                    state.editing = None;

                                    return Command::perform(
                                        async move {
                                            crate::global_config()
                                                .options
                                                .set(&key, OptionValue::String(value))
                                                .await
                                                .map_err(|e| e.to_string())
                                        },
                                        Msg::ValueSaved,
                                    );
                                }
                            }
                        }
                    }
                }
                Command::None
            }

            Msg::SaveValue => {
                if let Some(EditingState::TextInput { key, input }) = &state.editing {
                    let key = key.clone();
                    let input = input.clone();

                    let opt = state.current_options.iter().find(|o| o.key == key).cloned();
                    if let Some(opt) = opt {
                        state.editing = None;

                        return Command::perform(
                            async move {
                                let value = match opt.ty {
                                    OptionType::UInt { .. } => {
                                        let parsed = input.parse::<u64>()
                                            .map_err(|e| format!("Invalid number: {}", e))?;
                                        OptionValue::UInt(parsed)
                                    }
                                    OptionType::String { .. } | OptionType::Enum { .. } => {
                                        OptionValue::String(input)
                                    }
                                    _ => return Err("Unsupported type".to_string()),
                                };

                                crate::global_config()
                                    .options
                                    .set(&key, value)
                                    .await
                                    .map_err(|e| e.to_string())
                            },
                            Msg::ValueSaved,
                        );
                    }
                }
                Command::None
            }

            Msg::CancelEdit => {
                state.editing = None;
                Command::None
            }

            Msg::ValueSaved(Ok(())) => {
                state.error = None;
                // Reload values and runtime config
                Command::batch(vec![
                    Command::perform(
                        async {
                            let config = crate::global_config();
                            let mut values = std::collections::HashMap::new();

                            for def in crate::options_registry().list_all() {
                                if let Ok(value) = config.options.get(&def.key).await {
                                    values.insert(def.key.clone(), value);
                                }
                            }

                            Ok(values)
                        },
                        Msg::ValuesLoaded,
                    ),
                    Command::perform(
                        async {
                            let new_config = crate::tui::state::RuntimeConfig::load_from_options().await
                                .map_err(|e| e.to_string())?;
                            crate::reload_runtime_config(new_config);
                            Ok(())
                        },
                        Msg::ConfigReloaded,
                    ),
                ])
            }

            Msg::ValueSaved(Err(e)) => {
                state.error = Some(e);
                state.editing = None;
                Command::None
            }

            Msg::ConfigReloaded(Ok(())) => {
                log::debug!("Runtime config reloaded successfully");
                Command::None
            }

            Msg::ConfigReloaded(Err(e)) => {
                log::error!("Failed to reload runtime config: {}", e);
                state.error = Some(format!("Failed to reload config: {}", e));
                Command::None
            }
        }
    }

    fn view(state: &mut State) -> LayeredView<Msg> {
        use_constraints!();
        let theme = &crate::global_runtime_config().theme;

        // Left sidebar: namespace list
        let namespace_list = Element::list(
            "namespace-list",
            &state.namespaces,
            &state.namespace_list_state,
            theme
        )
        .on_select(Msg::SelectNamespace)
        .on_activate(Msg::SelectNamespace)
        .on_navigate(Msg::NamespaceListNavigate)
        .build();

        let left_panel = Element::panel(namespace_list)
            .title("Categories")
            .build();

        // Right panel: option list with values
        let option_list_content = if state.current_options.is_empty() {
            Element::styled_text(Line::from(vec![
                Span::styled("No options in this category", Style::default().fg(theme.subtext0))
            ])).build()
        } else {
            // Calculate max name width for alignment
            let max_name_width = state.current_options.iter()
                .map(|opt| opt.display_name.len())
                .max()
                .unwrap_or(0);

            // Create wrapped options with values
            let options_with_values: Vec<OptionWithValue> = state.current_options.iter()
                .map(|opt| OptionWithValue {
                    value: state.values.get(&opt.key).unwrap_or(&opt.default).clone(),
                    definition: opt.clone(),
                    max_name_width,
                })
                .collect();

            Element::list(
                "option-list",
                &options_with_values,
                &state.option_list_state,
                theme
            )
            .on_select(Msg::SelectOption)
            .on_activate(Msg::SelectOption)
            .on_navigate(Msg::OptionListNavigate)
            .build()
        };

        let namespace_title = if state.selected_namespace < state.namespaces.len() {
            format!("Options - {}", state.namespaces[state.selected_namespace])
        } else {
            "Options".to_string()
        };

        let option_list_panel = Element::panel(option_list_content)
            .title(&namespace_title)
            .build();

        // Main layout - just the two lists side by side
        let main_ui = if let Some(error) = &state.error {
            // With error display
            let error_section = Element::container(
                Element::styled_text(Line::from(vec![
                    Span::styled(format!("Error: {}", error), Style::default().fg(theme.red))
                ])).build()
            )
            .padding(1)
            .build();

            row![
                left_panel => Length(30),
                col![
                    option_list_panel => Fill(1),
                    error_section => Length(2),
                ] => Fill(1),
            ]
        } else {
            // Without error display
            row![
                left_panel => Length(30),
                option_list_panel => Fill(1),
            ]
        };

        // If editing, add a modal
        let mut view = LayeredView::new(main_ui);

        if state.editing.is_some() {
            if let Some(opt) = state.current_options.get(state.selected_option).cloned() {
                let modal = Self::render_edit_modal(state, &opt, theme);
                view = view.with_app_modal(modal, crate::tui::Alignment::Center);
            }
        }

        view
    }

    fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
        use crossterm::event::KeyCode;

        let mut subs = vec![];

        // If not editing, allow Enter to start editing the selected option
        if state.editing.is_none() && !state.current_options.is_empty() {
            subs.push(Subscription::keyboard(
                KeyCode::Enter,
                "Edit selected option",
                Msg::StartEdit,
            ));
        }

        // If editing, allow Escape to cancel
        if state.editing.is_some() {
            subs.push(Subscription::keyboard(
                KeyCode::Esc,
                "Cancel editing",
                Msg::CancelEdit,
            ));
        }

        subs
    }

    fn title() -> &'static str {
        "Settings"
    }

    fn status(_state: &State) -> Option<Line<'static>> {
        Some(Line::from(vec![
            Span::raw("Configure application options")
        ]))
    }
}

impl SettingsApp {
    fn render_edit_modal(
        state: &mut State,
        opt: &OptionDefinition,
        theme: &Theme,
    ) -> Element<Msg> {
        use_constraints!();

        // Build the edit widget with proper panel wrapping (matching other modals)
        let (value_panel, hint) = if let Some(EditingState::TextInput { key, input }) = &state.editing {
            if key == &opt.key {
                let input_widget = Element::text_input(
                    FocusId::new("edit-input"),
                    input.as_str(),
                    &state.edit_input_state
                )
                .on_event(Msg::EditInputEvent)
                .build();

                (
                    Element::panel(input_widget).title("Value").build(),
                    "Press Enter to save, Esc to cancel"
                )
            } else {
                return Element::text("Invalid edit state"); // Should not happen
            }
        } else if let Some(EditingState::Select { key, options }) = &state.editing {
            if key == &opt.key {
                let select_widget = Element::select(
                    FocusId::new("edit-select"),
                    options.clone(),
                    &mut state.edit_select_state
                )
                .on_event(Msg::EditSelectEvent)
                .build();

                let hint_text = if state.edit_select_state.is_open() {
                    "Up/Down to navigate, Enter to select, Esc to close"
                } else {
                    "Press Enter to open dropdown, Esc to cancel"
                };

                (
                    Element::panel(select_widget).title("Value").build(),
                    hint_text
                )
            } else {
                return Element::text("Invalid edit state"); // Should not happen
            }
        } else {
            return Element::text("No edit state"); // Should not happen
        };

        // Build modal content following the same pattern as other modals
        // Height calculation: 1 + 1 + 1 + 3 + 1 + 1 + 10 = 18 lines
        // + padding(2) = 4 lines (top 2, bottom 2)
        // + outer panel borders = 2 lines
        // Total = 24 lines minimum
        let modal_body = col![
            // Option name and description
            Element::styled_text(Line::from(vec![
                Span::styled(opt.display_name.clone(), Style::default().fg(theme.lavender).bold())
            ])).build() => Length(1),
            Element::styled_text(Line::from(vec![
                Span::styled(opt.description.clone(), Style::default().fg(theme.subtext0))
            ])).build() => Length(1),
            spacer!() => Length(1),
            // Value panel (nested panel with input/select inside)
            value_panel => Length(3),
            spacer!() => Length(1),
            // Hint
            Element::styled_text(Line::from(vec![
                Span::styled(hint.to_string(), Style::default().fg(theme.subtext0))
            ])).build() => Length(1),
            // Extra space for dropdown overlay
            spacer!() => Length(12),
        ];

        // Wrap in outer panel with title, width, and height
        Element::panel(
            Element::container(modal_body)
                .padding(2)
                .build()
        )
        .title("Edit Option")
        .width(60)
        .height(26)
        .build()
    }
}

pub struct SettingsApp;
