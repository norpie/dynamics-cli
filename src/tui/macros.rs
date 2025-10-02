/// UI layout and pattern macros for ergonomic view construction
///
/// This module provides declarative macros to reduce boilerplate in view functions.
/// See layout.md for design rationale and usage examples.

/// Create a spacer element for vertical/horizontal gaps
///
/// # Examples
/// ```
/// spacer!()     // 1 line gap (Element::text(""))
/// spacer!(3)    // 3 line gap (column of empty text)
/// ```
#[macro_export]
macro_rules! spacer {
    () => {
        $crate::tui::Element::text("")
    };
    ($height:expr) => {{
        let items: Vec<_> = (0..$height)
            .map(|_| ($crate::tui::LayoutConstraint::Length(1), $crate::tui::Element::text("")))
            .collect();
        $crate::tui::element::ColumnBuilder::from_items(items).spacing(0).build()
    }};
}

/// Create a vertical column layout
///
/// # Examples
/// ```
/// // Simple: all children get Fill(1) constraint
/// col![
///     Element::text("Header"),
///     Element::text("Body"),
///     Element::text("Footer"),
/// ]
///
/// // With explicit constraints using => syntax
/// col![
///     Element::text("Header") => Length(1),
///     list => Fill(1),
///     Element::text("Footer") => Length(1),
/// ]
/// ```
#[macro_export]
macro_rules! col {
    // Without constraints - use Fill(1) default
    [ $($child:expr),* $(,)? ] => {{
        let mut builder = $crate::tui::element::ColumnBuilder::new();
        $(
            builder = builder.add($child, $crate::tui::LayoutConstraint::Fill(1));
        )*
        builder.build()
    }};

    // With explicit constraints using => syntax
    [ $($child:expr => $constraint:expr),* $(,)? ] => {{
        let mut builder = $crate::tui::element::ColumnBuilder::new();
        $(
            builder = builder.add($child, $constraint);
        )*
        builder.build()
    }};
}

/// Create a horizontal row layout
///
/// # Examples
/// ```
/// // Simple: all children get Fill(1) constraint
/// row![
///     Element::button("cancel", "Cancel"),
///     Element::button("confirm", "Confirm"),
/// ]
///
/// // With explicit constraints using => syntax
/// row![
///     sidebar => Length(20),
///     content => Fill(1),
///     details => Length(30),
/// ]
/// ```
#[macro_export]
macro_rules! row {
    // Without constraints - use Fill(1) default
    [ $($child:expr),* $(,)? ] => {{
        let mut builder = $crate::tui::element::RowBuilder::new();
        $(
            builder = builder.add($child, $crate::tui::LayoutConstraint::Fill(1));
        )*
        builder.build()
    }};

    // With explicit constraints using => syntax
    [ $($child:expr => $constraint:expr),* $(,)? ] => {{
        let mut builder = $crate::tui::element::RowBuilder::new();
        $(
            builder = builder.add($child, $constraint);
        )*
        builder.build()
    }};
}

/// Import all layout constraint types for shorter syntax
///
/// # Example
/// ```
/// use_constraints!();
/// col![
///     thing @ Length(3),  // no need for LayoutConstraint::Length
///     thing @ Fill(1),
/// ]
/// ```
#[macro_export]
macro_rules! use_constraints {
    () => {
        use $crate::tui::LayoutConstraint::{Fill, Length, Min};
    };
}

/// Create a button row with consistent spacing
///
/// # Example
/// ```
/// button_row![
///     ("cancel", "Cancel", Msg::Cancel),
///     ("confirm", "Confirm", Msg::Confirm),
/// ]
/// ```
#[macro_export]
macro_rules! button_row {
    [ $(($id:expr, $label:expr, $msg:expr)),* $(,)? ] => {{
        let mut builder = $crate::tui::element::RowBuilder::new();
        let mut idx = 0;
        $(
            if idx > 0 {
                builder = builder.add(
                    $crate::tui::Element::text("  "),
                    $crate::tui::LayoutConstraint::Length(2)
                );
            }
            builder = builder.add(
                $crate::tui::Element::button($id, $label)
                    .on_press($msg)
                    .build(),
                $crate::tui::LayoutConstraint::Fill(1)
            );
            idx += 1;
        )*
        builder.spacing(0).build()
    }};
}

/// Create a modal overlay (dimmed background with centered content)
///
/// # Examples
/// ```
/// modal!(main_ui, modal_content)
/// modal!(main_ui, modal_content, Alignment::TopRight)
/// ```
#[macro_export]
macro_rules! modal {
    ($base:expr, $overlay:expr) => {
        $crate::tui::Element::stack(vec![
            $crate::tui::Layer::new($base),
            $crate::tui::Layer::new($overlay).center().dim(true),
        ])
    };
    ($base:expr, $overlay:expr, $align:expr) => {
        $crate::tui::Element::stack(vec![
            $crate::tui::Layer::new($base),
            $crate::tui::Layer::new($overlay).align($align).dim(true),
        ])
    };
}

/// Display validation error with warning icon
///
/// # Example
/// ```
/// error_display!(state.form.validation_error, theme)
/// ```
#[macro_export]
macro_rules! error_display {
    ($error_opt:expr, $theme:expr) => {
        if let Some(ref err) = $error_opt {
            $crate::col![
                $crate::tui::Element::styled_text(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(
                        format!("⚠ {}", err),
                        ratatui::style::Style::default().fg($theme.red)
                    )
                ]))
                .build() => $crate::tui::LayoutConstraint::Length(1),
                $crate::spacer!() => $crate::tui::LayoutConstraint::Length(1),
            ]
        } else {
            $crate::tui::Element::text("")
        }
    };
}

/// Create a labeled text input field in a panel
///
/// # Example
/// ```
/// labeled_input!(
///     "Name",
///     "create-name-input",
///     &state.create_form.name,
///     &state.create_form.name_input_state,
///     Msg::CreateFormNameChanged
/// )
/// ```
#[macro_export]
macro_rules! labeled_input {
    ($title:expr, $id:expr, $value:expr, $state:expr, $on_change:expr) => {
        $crate::tui::Element::panel(
            $crate::tui::Element::text_input($id, $value, $state)
                .on_change($on_change)
                .build(),
        )
        .title($title)
        .build()
    };
    // With placeholder
    ($title:expr, $id:expr, $value:expr, $state:expr, $on_change:expr, $placeholder:expr) => {
        $crate::tui::Element::panel(
            $crate::tui::Element::text_input($id, $value, $state)
                .placeholder($placeholder)
                .on_change($on_change)
                .build(),
        )
        .title($title)
        .build()
    };
}
