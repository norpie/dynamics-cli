# UI Ergonomics Proposals - Layout Macro Design

## Current State Analysis

**Pain Points from Code Analysis:**
- **30% layout code** - ColumnBuilder/RowBuilder verbosity dominates
- **Spacers** - `Element::text("")` for gaps appears 15+ times
- **Button rows** - 17 lines for Cancel/Confirm pattern
- **Modal structure** - 15+ lines for stack overlay boilerplate
- **Conditional UI** - verbose if/else for validation errors, loading states

---

## Proposal 1: Core Layout Macros (from proposal_new.md)

### Implementation Strategy

```rust
// src/tui/macros.rs
#[macro_export]
macro_rules! col {
    // Simple: no constraints, use Fill(1) default
    [ $($child:expr),* $(,)? ] => {{
        let mut builder = $crate::tui::element::ColumnBuilder::new();
        $(
            builder = builder.add($child, $crate::tui::LayoutConstraint::Fill(1));
        )*
        builder.build()
    }};

    // With constraints using @ syntax
    [ $($child:expr @ $constraint:expr),* $(,)? ] => {{
        let mut builder = $crate::tui::element::ColumnBuilder::new();
        $(
            builder = builder.add($child, $constraint);
        )*
        builder.build()
    }};
}

#[macro_export]
macro_rules! row {
    // Same pattern as col!
    [ $($child:expr),* $(,)? ] => {{ /* ... */ }};
    [ $($child:expr @ $constraint:expr),* $(,)? ] => {{ /* ... */ }};
}
```

### Usage Comparison

**Before (17 lines):**
```rust
ColumnBuilder::new()
    .add(name_input, LayoutConstraint::Length(3))
    .add(Element::text(""), LayoutConstraint::Length(1))
    .add(source_select, LayoutConstraint::Length(10))
    .add(Element::text(""), LayoutConstraint::Length(1))
    .add(target_select, LayoutConstraint::Length(10))
    .add(Element::text(""), LayoutConstraint::Length(1))
    .add(buttons, LayoutConstraint::Length(3))
    .spacing(0)
    .build()
```

**After (9 lines, 47% reduction):**
```rust
col![
    name_input @ Length(3),
    spacer!(1),
    source_select @ Length(10),
    spacer!(1),
    target_select @ Length(10),
    spacer!(1),
    buttons @ Length(3),
]
```

**Estimated savings: 200-250 lines across migration module (13-16% of total)**

---

## Proposal 2: Helper Macros

### 2.1 Spacer Element

```rust
#[macro_export]
macro_rules! spacer {
    () => { Element::text("") };
    ($height:expr) => {
        Element::column(vec![Element::text(""); $height]).build()
    };
}

// Usage
spacer!()     // 1 line gap
spacer!(3)    // 3 line gap
```

**Savings: 15 instances × 15 chars = ~1.5 lines**

### 2.2 Use Shorthand

```rust
// Import all constraint types at once
#[macro_export]
macro_rules! use_constraints {
    () => {
        use $crate::tui::LayoutConstraint::{Length, Min, Fill};
    };
}

// In view functions
use_constraints!();
col![
    thing @ Length(3),  // no need for LayoutConstraint::Length
    thing @ Fill(1),
]
```

---

## Proposal 3: Pattern Macros

### 3.1 Button Row Pattern

**Current (17 lines):**
```rust
RowBuilder::new()
    .add(
        Element::button(FocusId::new("cancel"), "Cancel")
            .on_press(Msg::Cancel)
            .build(),
        LayoutConstraint::Fill(1),
    )
    .add(Element::text("  "), LayoutConstraint::Length(2))
    .add(
        Element::button(FocusId::new("confirm"), "Confirm")
            .on_press(Msg::Confirm)
            .build(),
        LayoutConstraint::Fill(1),
    )
    .spacing(0)
    .build()
```

**Proposed (1 line, 94% reduction):**
```rust
#[macro_export]
macro_rules! button_row {
    [ $(($id:literal, $label:literal, $msg:expr)),* $(,)? ] => {{
        let mut builder = $crate::tui::element::RowBuilder::new();
        let count = [ $($label),* ].len();
        let mut idx = 0;
        $(
            if idx > 0 {
                builder = builder.add(Element::text("  "), LayoutConstraint::Length(2));
            }
            builder = builder.add(
                Element::button($id, $label)
                    .on_press($msg)
                    .build(),
                LayoutConstraint::Fill(1)
            );
            idx += 1;
        )*
        builder.spacing(0).build()
    }};
}

// Usage
button_row![
    ("cancel", "Cancel", Msg::Cancel),
    ("confirm", "Confirm", Msg::Confirm),
]
```

**Savings: 8 occurrences × 15 lines = 120 lines (8%)**

### 3.2 Modal Stack Pattern

**Current (15 lines):**
```rust
Element::stack(vec![
    crate::tui::Layer {
        element: main_ui,
        alignment: Alignment::TopLeft,
        dim_below: false,
    },
    crate::tui::Layer {
        element: modal_content,
        alignment: Alignment::Center,
        dim_below: true,
    },
])
```

**Proposed (1 line, 93% reduction):**
```rust
#[macro_export]
macro_rules! modal {
    ($base:expr, $overlay:expr) => {
        Element::stack(vec![
            Layer::new($base),
            Layer::new($overlay).center().dim(true),
        ])
    };
    ($base:expr, $overlay:expr, $align:expr) => {
        Element::stack(vec![
            Layer::new($base),
            Layer::new($overlay).align($align).dim(true),
        ])
    };
}

// Usage
modal!(main_ui, modal_content)
modal!(main_ui, modal_content, Alignment::TopRight)
```

**Savings: 6 occurrences × 13 lines = 78 lines (5%)**

### 3.3 Error Display Pattern

**Current (13 lines):**
```rust
let error_section = if let Some(ref error) = state.form.validation_error {
    ColumnBuilder::new()
        .add(
            Element::styled_text(Line::from(vec![
                Span::styled(format!("⚠ {}", error), Style::default().fg(theme.red))
            ])).build(),
            LayoutConstraint::Length(1)
        )
        .add(Element::text(""), LayoutConstraint::Length(1))
        .spacing(0)
        .build()
} else {
    Element::text("")
};
```

**Proposed (1 line, 92% reduction):**
```rust
#[macro_export]
macro_rules! error_display {
    ($error_opt:expr, $theme:expr) => {
        if let Some(ref err) = $error_opt {
            col![
                Element::styled_text(Line::from(vec![
                    Span::styled(format!("⚠ {}", err), Style::default().fg($theme.red))
                ])).build() @ Length(1),
                spacer!() @ Length(1),
            ]
        } else {
            Element::text("")
        }
    };
}

// Usage
error_display!(state.form.validation_error, theme)
```

**Savings: 4 occurrences × 11 lines = 44 lines (3%)**

### 3.4 Labeled Input Pattern

**Current (10 lines):**
```rust
Element::panel(
    Element::text_input(
        FocusId::new("create-name-input"),
        &state.create_form.name,
        &state.create_form.name_input_state
    )
    .placeholder("Migration name")
    .on_change(Msg::CreateFormNameChanged)
    .build()
)
.title("Name")
.build()
```

**Proposed (6 lines, 40% reduction):**
```rust
#[macro_export]
macro_rules! labeled_input {
    ($title:literal, $id:literal, $value:expr, $state:expr, $on_change:expr) => {
        Element::panel(
            Element::text_input($id, $value, $state)
                .on_change($on_change)
                .build()
        )
        .title($title)
        .build()
    };
}

// Usage
labeled_input!(
    "Name",
    "create-name-input",
    &state.create_form.name,
    &state.create_form.name_input_state,
    Msg::CreateFormNameChanged
)
```

**Savings: 10 occurrences × 4 lines = 40 lines (3%)**

---

## Proposal 4: Alternative Approaches (Non-Macro)

### 4.1 Constraint Type Aliases

```rust
// Shorter names in scope
use LayoutConstraint::{Length as Len, Fill, Min};

col![
    thing @ Len(3),  // vs Length(3)
    thing @ Fill(1),
]
```

**Pros:** No macros, simple imports
**Cons:** Minimal savings (~4 chars per use)

### 4.2 Element Helper Methods

```rust
impl<Msg> Element<Msg> {
    pub fn with_len(self, n: u16) -> (LayoutConstraint, Self) {
        (LayoutConstraint::Length(n), self)
    }

    pub fn with_fill(self, n: u16) -> (LayoutConstraint, Self) {
        (LayoutConstraint::Fill(n), self)
    }
}

// Usage with builder
ColumnBuilder::new()
    .add_item(thing.with_len(3))  // returns (Length(3), thing)
    .add_item(thing.with_fill(1))
    .build()
```

**Pros:** Type-safe, chainable
**Cons:** Still verbose, doesn't help with builder boilerplate

### 4.3 Modal Helper Function

```rust
// In element/mod.rs
impl<Msg> Element<Msg> {
    pub fn as_modal(self, overlay: Element<Msg>) -> Element<Msg> {
        Element::stack(vec![
            Layer::new(self),
            Layer::new(overlay).center().dim(true),
        ])
    }
}

// Usage
main_ui.as_modal(modal_content)
```

**Pros:** Clean, discoverable
**Cons:** Limited to simple case (no alignment customization)

---

## Proposal 5: Hybrid Approach (Recommended)

**Combine macros + helpers for maximum ergonomics:**

### Tier 1: Core Layout (Macros - biggest win)
- `col![]` / `row![]` - **~250 line savings (16%)**
- `spacer!()` - **~15 line savings (1%)**

### Tier 2: Common Patterns (Macros - high frequency)
- `button_row![]` - **~120 line savings (8%)**
- `modal!()` - **~78 line savings (5%)**
- `error_display!()` - **~44 line savings (3%)**

### Tier 3: Specialized (Functions - lower frequency)
- `labeled_input!()` - **~40 line savings (3%)**
- Element helper methods (`.as_modal()`, `.with_len()`)

### Total Estimated Savings
- **547 lines eliminated (35% of layout code, 11% of total codebase)**
- Migration module: 1564 lines → **~1017 lines (35% reduction)**

---

## Proposal 6: Advanced Macros (Proc Macro Path)

**Only if declarative macros prove insufficient:**

```rust
// Requires proc_macro crate
use tui::view;

#[view]
fn create_modal(state: &State, theme: &Theme) -> Element<Msg> {
    panel("Create New Migration", width: 80) {
        col(padding: 2, spacing: 1) {
            labeled_input("Name", "name-input", state.name, state.name_state) {
                .placeholder("Migration name")
                .on_change(Msg::NameChanged)
            }

            select("Source", "source-select", source_options, state.source_state) {
                .on_event(Msg::SourceEvent)
            }

            button_row {
                "Cancel" => Msg::Cancel
                "Confirm" => Msg::Confirm
            }
        }
    }
}
```

**Pros:**
- Cleanest syntax (60-70% reduction)
- Near-SwiftUI/Jetpack Compose ergonomics

**Cons:**
- Complex implementation (~2-3 days work)
- Harder debugging
- IDE support issues
- Macro expansion opacity

---

## Implementation Roadmap

### Phase 1: Quick Wins (1-2 hours)
1. `spacer!()` macro - simple, frequent use
2. Constraint type aliases (`use LayoutConstraint::*`)

**Deliverable:** 5-10% code reduction

### Phase 2: Core Layout (3-4 hours)
1. `col![]` macro with `@` constraint syntax
2. `row![]` macro
3. Test with migration apps
4. Document edge cases

**Deliverable:** 15-20% code reduction

### Phase 3: Pattern Macros (4-5 hours)
1. `button_row![]` - high value
2. `modal!()` - medium value
3. `error_display!()` - medium value
4. `labeled_input!()` - lower value

**Deliverable:** 30-35% code reduction total

### Phase 4: Polish (2-3 hours)
1. Comprehensive tests
2. Documentation with examples
3. Migration guide for existing code
4. Error message improvements

**Total Time Investment: 10-14 hours**
**Expected Payoff: 35% layout code reduction (547 lines in migration module alone)**

---

## Risk Analysis

### Low Risk
- **spacer!()** - trivial macro, can't break
- **Type aliases** - zero risk, just imports

### Medium Risk
- **col!/row!** - Complex pattern matching, needs good tests
- **Constraint @ syntax** - May confuse newcomers initially

### High Risk
- **Proc macros** - Debugging pain, maintenance burden
- **Over-abstraction** - "Magical" code harder to understand

### Mitigation
1. Comprehensive macro tests
2. Clear documentation with before/after examples
3. Keep builders as primary API (macros as sugar)
4. Gradual rollout (tier by tier)

---

## Recommendation

**Start with Tier 1 + Tier 2 hybrid approach:**

1. Implement `spacer!()`, `col![]`, `row![]` first (Tier 1)
2. Migrate migration apps to use them
3. Measure actual savings
4. If successful, add `button_row![]`, `modal!()` (Tier 2)
5. Re-evaluate need for Tier 3/proc macros

**Success Criteria:**
- ✅ 25%+ layout code reduction
- ✅ Readable (newcomers can understand)
- ✅ Maintainable (clear macro errors)
- ✅ No regression in type safety

This balances **developer comfort** (your goal) with **code maintainability** (long-term health).
