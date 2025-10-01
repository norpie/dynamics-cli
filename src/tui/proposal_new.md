# TUI Framework - Missing Features & Improvements

This document outlines improvements to make based on real-world usage of the current implementation.

## Status: What We Have vs Need

**✅ Implemented**: App trait, Commands, Runtime, Multi-app navigation, Basic elements (Text/Button), Layout constraints, Mouse events, Async support, Pub/sub, Theme system

**❌ Missing**: List widget, TextInput, Ergonomic macros, Focus management, Scrollable containers

---

## 1. Core Widgets (HIGH PRIORITY)

### List Widget

**Critical for**: Contact lists, menus, file browsers, any selection UI

```rust
pub enum Element<Msg> {
    // ... existing variants ...

    List {
        items: Vec<Element<Msg>>,
        selected: Option<usize>,
        on_select: fn(usize) -> Msg,
        on_activate: Option<fn(usize) -> Msg>,  // Enter key
        scrollable: bool,
        scroll_offset: usize,
    },
}

// Builder API
Element::list(items)
    .on_select(Msg::ItemSelected)
    .on_activate(Msg::ItemActivated)  // Optional: Enter to open
    .build()

// Keyboard support:
// - Up/Down: Navigate items
// - PageUp/PageDown: Jump by page
// - Home/End: First/last item
// - Enter: Activate selected (if on_activate provided)

// Mouse support:
// - Click: Select item
// - Double-click: Activate item
// - Wheel: Scroll list

// Rendering:
// - Highlight selected item with theme.lavender background
// - Show scroll indicator when content overflows
// - Virtual scrolling for 1000+ items (render only visible)
```

### TextInput Widget

**Critical for**: Search bars, forms, CLI arguments, filters

```rust
pub enum Element<Msg> {
    // ... existing variants ...

    TextInput {
        value: String,
        placeholder: String,
        on_change: fn(String) -> Msg,
        on_submit: Option<Msg>,
        cursor_pos: usize,
        password: bool,  // Render as ***
        max_length: Option<usize>,
    },
}

// Builder API
Element::text_input(current_value)
    .placeholder("Search contacts...")
    .on_change(Msg::SearchChanged)
    .on_submit(Msg::SearchSubmitted)
    .max_length(100)
    .build()

// Keyboard support:
// - Type: Insert character
// - Backspace: Delete char before cursor
// - Delete: Delete char after cursor
// - Left/Right arrows: Move cursor
// - Ctrl+A / Home: Jump to start
// - Ctrl+E / End: Jump to end
// - Ctrl+U: Clear line
// - Enter: Submit (if on_submit provided)

// Mouse support:
// - Click: Position cursor
// - Drag: Select text (future)

// Rendering:
// - Show cursor as inverted color block
// - Placeholder text in theme.overlay1 when empty
// - Password mode: render value as "•••"
```

### Checkbox Widget

**Useful for**: Settings, multi-select lists, toggle options

```rust
pub enum Element<Msg> {
    // ... existing variants ...

    Checkbox {
        label: String,
        checked: bool,
        on_toggle: fn(bool) -> Msg,
        disabled: bool,
    },
}

// Builder API
Element::checkbox("Enable dark mode", is_dark_mode)
    .on_toggle(Msg::ToggleDarkMode)
    .build()

// Keyboard: Space to toggle (when focused)
// Mouse: Click to toggle
// Rendering: [✓] Checked  [ ] Unchecked
```

### RadioGroup Widget

**Useful for**: Exclusive selections (theme picker, sort order)

```rust
pub enum Element<Msg> {
    // ... existing variants ...

    RadioGroup {
        options: Vec<String>,
        selected: usize,
        on_select: fn(usize) -> Msg,
    },
}

// Builder API
Element::radio_group(vec!["Mocha", "Latte", "Frappé"])
    .selected(0)
    .on_select(Msg::ThemeSelected)
    .build()

// Keyboard: Up/Down to change selection
// Mouse: Click option
// Rendering: (•) Selected  ( ) Not selected
```

---

## 2. Ergonomic Macros (HIGH PRIORITY)

### Problem: Current API is verbose

```rust
// CURRENT (too verbose):
ColumnBuilder::new()
    .add(Element::text("Hello"), LayoutConstraint::Length(1))
    .add(
        Element::button("Click me")
            .on_press(Msg::Clicked)
            .build(),
        LayoutConstraint::Length(3),
    )
    .add(Element::text("Footer"), LayoutConstraint::Length(1))
    .spacing(1)
    .build()
```

### Solution: Declarative macros

```rust
// PROPOSED (clean):
column![
    text("Hello"),
    button("Click me").on_press(Msg::Clicked),
    text("Footer"),
]

// With explicit constraints:
column![
    text("Header") @ Length(1),
    list(items).on_select(Msg::Selected) @ Fill(1),
    text("Footer") @ Length(1),
]

// Row macro:
row![
    button("Cancel").on_press(Msg::Cancel) @ Fill(1),
    spacer() @ Length(2),  // Dedicated spacer element
    button("Confirm").on_press(Msg::Confirm) @ Fill(1),
]
```

### Implementation

```rust
#[macro_export]
macro_rules! column {
    // Without constraints - use element's default_constraint()
    [ $($child:expr),* $(,)? ] => {
        {
            let mut builder = $crate::tui::element::ColumnBuilder::new();
            $(
                let child = $child;
                let constraint = child.default_constraint();
                builder = builder.add(child, constraint);
            )*
            builder.build()
        }
    };

    // With explicit constraints using @ syntax
    [ $($child:expr @ $constraint:expr),* $(,)? ] => {
        {
            let mut builder = $crate::tui::element::ColumnBuilder::new();
            $(
                builder = builder.add($child, $constraint);
            )*
            builder.build()
        }
    };
}

// Similar for row![], stack![], etc.
```

---

## 3. Focus Management (MEDIUM PRIORITY)

### Problem: No Tab navigation between elements

**User expectation**: Tab moves focus, Enter activates, Escape cancels

```rust
// Runtime tracks focused element
pub struct Runtime<A: App> {
    // ... existing fields ...

    focused_id: Option<ElementId>,
    focusable_elements: Vec<ElementId>,
}

// New element field
pub struct FocusableElement {
    id: ElementId,
    on_focus: Option<Msg>,
    on_blur: Option<Msg>,
    can_focus: bool,
}

// Example usage
view(state) {
    column![
        text_input(state.search)
            .on_change(Msg::SearchChanged)
            .focusable(true),  // Can receive focus

        list(state.items)
            .on_select(Msg::ItemSelected)
            .focusable(true),

        button("Submit")
            .on_press(Msg::Submit)
            .focusable(true),
    ]
}

// Global keyboard shortcuts in Runtime:
// - Tab: Focus next element
// - Shift+Tab: Focus previous element
// - Visual: Focused element gets highlighted border (theme.blue)
```

---

## 4. Scrollable Containers (MEDIUM PRIORITY)

**Useful for**: Long content that doesn't fit on screen

```rust
pub enum Element<Msg> {
    // ... existing variants ...

    Scrollable {
        child: Box<Element<Msg>>,
        vertical: bool,
        horizontal: bool,
        scroll_offset_y: usize,
        scroll_offset_x: usize,
        on_scroll: Option<fn(usize, usize) -> Msg>,
    },
}

// Builder API
Element::scrollable(
    column![/* many items */]
)
.vertical(true)
.build()

// Keyboard: PageUp/Down, Home/End
// Mouse: Wheel to scroll
// Rendering: Show scrollbar indicator when content overflows
```

---

## 5. Additional Useful Widgets

### ProgressBar

```rust
Element::ProgressBar {
    progress: f32,  // 0.0 to 1.0
    label: Option<String>,
}

// Usage
Element::progress_bar(0.65)
    .label("Migrating contacts...")
    .build()

// Rendering: [████████░░░░] 65%
```

### Spinner

```rust
Element::Spinner {
    label: Option<String>,
    frame: usize,  // Animation frame
}

// Extract from LoadingScreen as reusable widget
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// Usage
Element::spinner()
    .label("Loading...")
    .frame(state.animation_frame)
    .build()

// Auto-animate with timer subscription
```

### Table

```rust
Element::Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    selected_row: Option<usize>,
    column_widths: Vec<LayoutConstraint>,
    on_select: Option<fn(usize) -> Msg>,
    sortable: bool,
}

// Usage
Element::table()
    .headers(vec!["Name", "Email", "Company"])
    .rows(contact_rows)
    .column_widths(vec![Fill(2), Fill(2), Fill(1)])
    .on_select(Msg::RowSelected)
    .sortable(true)
    .build()
```

### Tabs

```rust
Element::Tabs {
    tabs: Vec<String>,
    selected: usize,
    on_select: fn(usize) -> Msg,
    content: Vec<Element<Msg>>,
}

// Usage
Element::tabs()
    .add("Contacts", contact_list_view)
    .add("Accounts", account_list_view)
    .add("Settings", settings_view)
    .selected(state.active_tab)
    .on_select(Msg::TabSelected)
    .build()

// Rendering: [ Contacts ]  Accounts  Settings
//             ─────────────────────────────────
//             <content for selected tab>
```

### Menu / Dropdown

```rust
Element::Menu {
    items: Vec<MenuItem<Msg>>,
    open: bool,
}

struct MenuItem<Msg> {
    label: String,
    on_select: Msg,
    shortcut: Option<String>,
    disabled: bool,
}

// Usage
Element::menu(state.menu_open)
    .item("New Contact", Msg::NewContact).shortcut("Ctrl+N")
    .item("Import...", Msg::Import).shortcut("Ctrl+I")
    .separator()
    .item("Exit", Msg::Quit).shortcut("Ctrl+Q")
    .build()
```

---

## 6. Layout Enhancements

### Additional Constraints

```rust
pub enum LayoutConstraint {
    Length(u16),      // ✅ Exists
    Min(u16),         // ✅ Exists
    Fill(u16),        // ✅ Exists

    // NEW:
    Percentage(u16),  // e.g., 30 = 30% of container
    Max(u16),         // At most N units
    Ratio(u16),       // For Ratio(1,2,1) style layouts
}

// Example
row![
    sidebar @ Percentage(20),      // 20% width sidebar
    content @ Fill(1),             // Remaining space
    details @ Max(40),             // At most 40 columns
]
```

### Spacer Element

```rust
// Instead of Element::text("")
Element::Spacer { size: u16 }

spacer()      // Default: 1 line/column
spacer(5)     // 5 lines/columns
```

---

## 7. Styling Improvements

### Style Composition

```rust
impl Style {
    fn merge(self, other: Style) -> Style {
        // Later style overrides earlier
    }
}

// Usage
let base_style = Style::default().fg(theme.text);
let hover_style = base_style.merge(Style::default().bg(theme.surface0));
```

### Pseudo-States

```rust
Element::Button {
    // ... existing fields ...

    style_normal: Option<Style>,
    style_hover: Option<Style>,
    style_active: Option<Style>,    // While pressed
    style_disabled: Option<Style>,
}

// Builder
button("Submit")
    .style_normal(Style::default().fg(theme.blue))
    .style_hover(Style::default().fg(theme.lavender))
    .style_disabled(Style::default().fg(theme.overlay0))
```

### Style Inheritance

```rust
// Children inherit parent's fg/bg if not overridden
Element::Container {
    // ... existing fields ...
    style: Option<Style>,  // Applied to all children
}

container(
    column![
        text("Inherits red"),
        text("Also red"),
    ]
)
.style(Style::default().fg(theme.red))
.build()
```

---

## 8. Performance Optimizations

### Virtual Scrolling for Lists

```rust
// Only render visible items (critical for 10,000+ item lists)
impl Renderer {
    fn render_list_virtualized(
        items: &[Element<Msg>],
        area: Rect,
        scroll_offset: usize,
    ) {
        let visible_items = area.height as usize;
        let start = scroll_offset;
        let end = (start + visible_items).min(items.len());

        // Only render items[start..end]
        for (i, item) in items[start..end].iter().enumerate() {
            // render at y = area.y + i
        }
    }
}
```

### View Memoization

```rust
// Cache view() results when state unchanged
pub struct Runtime<A: App> {
    // ... existing fields ...

    view_cache: Option<(StateHash, Element<A::Msg>)>,
}

// Hash state to detect changes
impl Runtime {
    fn render(&mut self, frame: &mut Frame) {
        let state_hash = hash(&self.state);

        let view = if let Some((cached_hash, cached_view)) = &self.view_cache {
            if cached_hash == state_hash {
                cached_view.clone()  // Reuse cached view
            } else {
                let new_view = A::view(&self.state, &self.theme);
                self.view_cache = Some((state_hash, new_view.clone()));
                new_view
            }
        } else {
            let new_view = A::view(&self.state, &self.theme);
            self.view_cache = Some((state_hash, new_view.clone()));
            new_view
        };

        Renderer::render(frame, &self.theme, &mut self.registry, &view, area);
    }
}
```

### Dirty Tracking

```rust
// Only re-render changed subtrees
pub enum Element<Msg> {
    // Add version/dirty flag to each element
    // Runtime compares previous frame to current frame
    // Skip rendering subtrees with matching version
}
```

---

## 9. Developer Experience

### Debug Overlay

```rust
// Press F12 to toggle debug overlay showing:
// - Element boundaries (colored boxes)
// - Element IDs
// - Layout constraints
// - Interaction areas
// - Focus state

Subscription::keyboard(KeyCode::F(12), "Toggle debug overlay", Msg::ToggleDebug)
```

### Performance Profiler

```rust
// Show render time per frame
// Highlight slow elements
// Log to debug output:
//   Frame #123: 16.7ms
//   - view(): 2.3ms
//   - render(): 14.4ms
//     - List (1000 items): 12.1ms ⚠️ SLOW
```

### Better Error Messages

```rust
// Current: "Button has no on_press handler"
// Better:
//   error: Button at src/apps/contacts.rs:45 is missing on_press handler
//
//   42 |     column![
//   43 |         text("Contacts"),
//   44 |         button("New Contact")  // ← Add .on_press(Msg::NewContact)
//      |         ^^^^^^^^^^^^^^^^^^^^^ this button has no handler
```

### Hot Reload (Future)

```rust
// Watch app source files
// On change: recompile and re-init app without restarting runtime
// Preserve state across reloads (serialize/deserialize)
```

---

## 10. Accessibility

### Keyboard-Only Navigation

**Goal**: Entire app usable without mouse

- ✅ Already have: Keyboard subscriptions for shortcuts
- ❌ Missing: Tab focus navigation
- ❌ Missing: Keyboard control for all interactive elements

### Screen Reader Support (Future)

```rust
// Add semantic labels for screen readers
Element::Button {
    // ... existing fields ...
    aria_label: Option<String>,
}

// Audio cues for state changes
// Text-to-speech for content
```

### High-Contrast Theme

```rust
// Add high-contrast variant to ThemeVariant
pub enum ThemeVariant {
    Mocha,
    Latte,
    HighContrast,  // Black/white with bold borders
}
```

### Customizable Keybindings

```rust
// Allow users to remap keys
pub struct KeyBindings {
    quit: KeyCode,
    help: KeyCode,
    focus_next: KeyCode,
    focus_prev: KeyCode,
    // ...
}

// Load from config file or environment
```

---

## Implementation Priority

### Phase 1 (Critical - Do First)
1. ✅ List widget - blocking for any selection UI
2. ✅ TextInput widget - blocking for search/forms
3. ✅ Ergonomic macros (column!, row!) - huge DX improvement
4. ✅ Focus management - Tab navigation is expected UX

### Phase 2 (Important - Do Soon)
5. Scrollable containers
6. ProgressBar & Spinner widgets
7. Checkbox & RadioGroup widgets
8. Additional layout constraints (Percentage, Max)

### Phase 3 (Nice to Have)
9. Table widget
10. Tabs widget
11. Menu/Dropdown widget
12. Style composition & pseudo-states
13. Virtual scrolling optimization

### Phase 4 (Future)
14. View memoization
15. Debug overlay
16. Hot reload
17. Screen reader support

---

## Notes

- Keep backward compatibility: all current code should still work
- Prefer explicit over implicit: builders > magic
- Document performance characteristics (O(n) rendering, virtual scrolling for O(1))
- Test with real-world apps (contacts, deadlines, migration)
