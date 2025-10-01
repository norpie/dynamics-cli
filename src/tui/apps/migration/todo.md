# Migration TUI → Elm-Inspired Framework: Full Analysis & Proposal

## Executive Summary

The existing migration module is a **substantial TUI implementation** (~36k LOC) with rich features:
- **7 screens** with complex navigation flows
- **13+ reusable components** (lists, modals, trees, field renderers)
- **Advanced features**: mouse support, async loading, hierarchical trees, field mapping, export

**Goal**: Migrate this to the new Elm-inspired TUI framework to gain:
- **Predictable state management** (pure update functions)
- **Cleaner architecture** (Msg/State pattern vs. mutable Screen trait)
- **Better testability** (pure functions vs. stateful rendering)
- **Reusable widgets** for other apps

---

## 1. Current Implementation Analysis

### 1.1 Architecture Pattern

**Current (raw ratatui)**:
```rust
trait Screen {
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn handle_event(&mut self, event: Event) -> ScreenResult;
    fn get_footer_actions(&self) -> Vec<FooterAction>;
    fn check_navigation(&mut self) -> Option<ScreenResult>;  // Async nav
}
```

**Issues**:
- `&mut self` in `render()` - allows mutation during rendering
- `RefCell` everywhere for interior mutability
- No clear data flow (events → state → view)
- Hard to test (stateful components)

### 1.2 Screens (Navigation Flow)

| Screen | Purpose | Complexity |
|--------|---------|------------|
| **MigrationSelectScreen** | List saved migrations, create new | ⭐⭐ Medium |
| **EnvironmentSelectScreen** | Two-phase selection (source → target) | ⭐⭐ Medium |
| **EntitySelectScreen** | Select entities to compare | ⭐⭐ Medium |
| **ComparisonSelectScreen** | Select entity pair | ⭐⭐ Medium |
| **LoadingScreen** | Async data fetch with progress | ⭐⭐ Medium |
| **UnifiedCompareScreen** | Main comparison (4 tabs, field mapping) | ⭐⭐⭐⭐⭐ Very Complex |

### 1.3 Components (Reusable Widgets)

| Component | Description | Framework Equivalent |
|-----------|-------------|---------------------|
| **ListComponent** | Selectable list + mouse + scroll | ❌ **MISSING** - need List widget |
| **FooterComponent** | Action bar with keybindings | ✅ Can build with Row/Text |
| **ModalComponent** | Generic modal overlay | ✅ Stack/Layer exists |
| **ConfirmationDialog** | Yes/No dialog | ✅ Can build with Panel/Button |
| **LoadingModal** | Spinner + progress tracking | ✅ Can build (extract spinner) |
| **PrefixMappingModal** | Prefix mapping config | ❌ Needs **TextInput** widget |
| **ManualMappingModal** | Manual field mapping | ❌ Needs **TextInput** widget |
| **ExamplesModal** | View example records | ✅ Can build with Column/Text |
| **HierarchyTree** | Expandable tree structure | ❌ **MISSING** - complex widget |
| **FieldRenderer** | Rich field rendering (type, required, mapping) | ✅ Can build with StyledText |

### 1.4 Missing Framework Features (BLOCKERS)

**HIGH PRIORITY (Must have for migration)**:
1. **List widget** - Used everywhere for selection ✅ **DONE**
2. **TextInput widget** - Required for search, prefix mapping, manual mapping
3. **Tree/Hierarchy widget** - Core of UnifiedCompareScreen (fields tab)
4. **Tabs widget** - UnifiedCompareScreen has 4 tabs (Fields, Relationships, Views, Forms)

**MEDIUM PRIORITY**:
5. **Scrollable containers** - Long lists of fields/entities

**LOW PRIORITY** (Can work around):
7. Ergonomic macros (`column![]` vs. verbose builders)

---

## 2. Mapping to App Trait Pattern

### 2.1 Example: MigrationSelectScreen → MigrationSelectApp

**Current (stateful)**:
```rust
pub struct MigrationSelectScreen {
    migrations: Vec<SavedMigration>,
    list: ListComponent<MigrationItem>,  // Mutable state
    config: Config,
    show_delete_confirmation: bool,
    delete_confirmation_modal: Option<ModalComponent<ConfirmationDialog>>,
}

impl Screen for MigrationSelectScreen {
    fn render(&mut self, f: &mut Frame, area: Rect) { /* ... */ }
    fn handle_event(&mut self, event: Event) -> ScreenResult { /* ... */ }
}
```

**Proposed (pure functions)**:
```rust
pub struct MigrationSelectApp;

#[derive(Clone)]
pub struct State {
    migrations: Vec<SavedMigration>,
    selected_index: Option<usize>,
    config: Config,
    show_delete_confirmation: bool,
    confirming_migration_name: Option<String>,
}

pub enum Msg {
    MigrationSelected(usize),
    CreateNew,
    DeleteSelected,
    ConfirmDelete,
    CancelDelete,
}

impl App for MigrationSelectApp {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::MigrationSelected(idx) => {
                let migration = state.migrations[idx].clone();
                Command::navigate_to(AppId::ComparisonSelect(migration))
            }
            Msg::CreateNew => {
                Command::navigate_to(AppId::EnvironmentSelect)
            }
            Msg::DeleteSelected => {
                if let Some(idx) = state.selected_index {
                    state.confirming_migration_name = Some(state.migrations[idx].name.clone());
                    state.show_delete_confirmation = true;
                }
                Command::none()
            }
            Msg::ConfirmDelete => {
                if let Some(name) = &state.confirming_migration_name {
                    let config = state.config.clone();
                    let name = name.clone();
                    // Async command to delete + reload
                    Command::perform(
                        async move {
                            config.remove_migration(&name)?;
                            Ok(())
                        },
                        |_| Msg::DeleteComplete,
                    )
                } else {
                    Command::none()
                }
            }
            Msg::CancelDelete => {
                state.show_delete_confirmation = false;
                state.confirming_migration_name = None;
                Command::none()
            }
        }
    }

    fn view(state: &State, theme: &Theme) -> Element<Msg> {
        let migration_list = Element::list(
            state.migrations.iter().map(|m| {
                Element::text(format!("{} → {}", m.source_env, m.target_env))
            }).collect()
        )
        .on_select(Msg::MigrationSelected)
        .selected(state.selected_index)
        .build();

        let content = Element::panel(migration_list)
            .title("Select Migration")
            .build();

        if state.show_delete_confirmation {
            Element::stack(vec![
                content,
                // Confirmation modal
                Element::panel(
                    Element::column(vec![
                        Element::text("Delete migration?"),
                        Element::row(vec![
                            Element::button("Delete").on_press(Msg::ConfirmDelete),
                            Element::button("Cancel").on_press(Msg::CancelDelete),
                        ]),
                    ])
                )
                .title("Confirm Delete")
                .build()
            ])
            .alignment(Alignment::Center)
            .build()
        } else {
            content
        }
    }

    fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Char('n'), "New migration", Msg::CreateNew),
            Subscription::keyboard(KeyCode::Delete, "Delete", Msg::DeleteSelected),
        ]
    }
}
```

**Benefits**:
- Pure `update()` - no hidden state mutations
- Pure `view()` - renders same output for same state
- Testable: `update(&mut state, Msg::CreateNew)` → assert command
- Clear data flow: Event → Msg → update → Command → view

---

## 3. Proposed Module Structure

```
src/tui/apps/migration/
├── mod.rs                          # Module exports
├── types.rs                        # Shared types (SavedMigration, Config wrappers)
│
├── migration_select.rs             # App: Select/create/delete migrations
├── environment_select.rs           # App: Select source + target environments
├── entity_select.rs                # App: Select entities from environment
├── comparison_select.rs            # App: Select entity pair to compare
├── loading.rs                      # App: Async data loading with progress
├── unified_compare/                # Most complex screen - needs sub-module
│   ├── mod.rs                      # Main app
│   ├── state.rs                    # State type (complex)
│   ├── msg.rs                      # Msg type (complex)
│   ├── update.rs                   # Update logic
│   ├── view.rs                     # View function
│   ├── tabs/                       # Tab-specific logic
│   │   ├── fields_tab.rs           # Fields comparison view
│   │   ├── relationships_tab.rs    # Relationships view
│   │   ├── views_tab.rs            # Views comparison
│   │   └── forms_tab.rs            # Forms comparison
│   └── field_mapping.rs            # Field mapping logic
│
└── widgets/                        # Migration-specific widgets
    ├── migration_list.rs           # List of migrations with actions
    ├── entity_tree.rs              # Hierarchical entity browser
    ├── field_comparison.rs         # Side-by-side field comparison
    ├── mapping_indicator.rs        # Visual mapping indicator
    └── progress_tracker.rs         # Fetch progress display
```

### 3.1 Widget Strategy

**Option A: Implement missing widgets in framework first** (RECOMMENDED)
- Pros: Reusable across all apps, cleaner API
- Cons: More upfront work
- **Widgets needed**: ~~List~~ ✅, TextInput, Tree, Tabs

**Option B: Build migration-specific widgets, extract later**
- Pros: Faster initial migration
- Cons: Code duplication, harder to maintain

**Recommendation**: Option A - the migration app needs these widgets, but so will other apps (contacts, deadlines). Invest in the framework.

---

## 4. Implementation Phases

### Phase 1: Build Missing Framework Widgets ⚙️

**Goal**: Implement HIGH PRIORITY missing widgets

1. **List widget** (src/tui/element.rs)
   - Selection, keyboard nav (Up/Down/PageUp/PageDown)
   - Mouse support (click, scroll)
   - Virtual scrolling for 1000+ items
   - `Element::list(items).on_select(Msg::ItemSelected)`

2. **TextInput widget**
   - Text editing (insert, backspace, cursor movement)
   - Placeholder text
   - `Element::text_input(value).on_change(Msg::InputChanged)`

3. **Tree widget** (complex)
   - Hierarchical structure with expand/collapse
   - `Element::tree(root_node).on_expand(Msg::NodeExpanded)`
   - Virtualized rendering

4. **Tabs widget**
   - Tab bar + content switching
   - `Element::tabs().add("Tab1", view1).selected(0)`

### Phase 2: Migrate Simple Screens 🏗️

**Goal**: Convert simple screens to validate pattern

1. **MigrationSelectApp**
   - State: list of migrations, selected index, modal state
   - Msg: Select, CreateNew, Delete, Confirm
   - View: List + confirmation modal

2. **EnvironmentSelectApp**
   - State: phase (source vs. target), environments, selections
   - Msg: SelectSource, SelectTarget
   - View: List with title update

3. **LoadingApp**
   - State: progress tracking, async task handles
   - Msg: UpdateProgress, FetchComplete, FetchFailed
   - View: Spinner + progress bars

### Phase 3: Migrate Complex Screen 🏔️

**Goal**: Convert UnifiedCompareScreen

**Challenges**:
- Very stateful (36+ fields in current implementation)
- 4 tabs with different views
- Complex interactions (field mapping, prefix config)
- Async data fetching

**Strategy**:
1. Split into sub-apps per tab (FieldsTab, RelationshipsTab, ViewsTab, FormsTab)
2. Use pub/sub for cross-tab communication
3. Shared state via parent app
4. Extract field mapping logic to pure functions

**State structure**:
```rust
pub struct UnifiedCompareState {
    // Data
    comparison: SavedComparison,
    source_fields: Vec<FieldInfo>,
    target_fields: Vec<FieldInfo>,
    source_relationships: Vec<RelationshipInfo>,
    target_relationships: Vec<RelationshipInfo>,
    // ... views, forms, examples

    // UI state
    active_tab: TabId,  // Fields | Relationships | Views | Forms
    field_mappings: HashMap<String, String>,
    prefix_mappings: Vec<PrefixMapping>,
    hide_matched: bool,
    sort_mode: SortMode,

    // Modal state
    show_prefix_modal: bool,
    show_manual_modal: bool,
    show_examples_modal: bool,

    // Selection state per tab
    fields_tab: FieldsTabState,
    relationships_tab: RelationshipsTabState,
    views_tab: ViewsTabState,
    forms_tab: FormsTabState,
}
```

### Phase 4: Polish & Export 🎨

**Goal**: Export functionality, refinements

1. Implement export to JSON/Excel (reuse existing logic)
2. Add keyboard shortcuts documentation
3. Improve visual polish (animations, better colors)
4. Performance optimization (virtual scrolling, memoization)

---

## 5. Migration Strategy Decision Points

### 5.1 Big Bang vs. Incremental?

**Option A: Incremental (RECOMMENDED)**
- Keep old `src/commands/migration/ui/` intact
- Build new `src/tui/apps/migration/` in parallel
- Switch when ready (feature flag or CLI arg `--new-ui`)
- Fallback if issues

**Option B: Big Bang**
- Delete old implementation
- Force completion
- Risky if blockers emerge

**Recommendation**: Incremental

### 5.2 Reuse Components?

**Question**: Can we reuse any existing components?

**Answer**: Probably not directly. The existing components are tightly coupled to:
- `Screen` trait (not `App` trait)
- Mutable rendering (`&mut self`)
- `RefCell` for state management
- ratatui widgets directly (not our `Element` tree)

**Exception**: Business logic can be extracted:
- Field matching algorithms (src/commands/migration/ui/screens/comparison_apps/matching/)
- Export logic (src/commands/migration/export.rs)
- Data models (FieldInfo, ViewInfo, etc.)

### 5.3 Testing Strategy

**Current**: Minimal tests (UI is hard to test)

**New**: Unit test pure functions
```rust
#[test]
fn test_migration_selection() {
    let mut state = State::new();
    let cmd = update(&mut state, Msg::MigrationSelected(0));

    assert!(matches!(cmd, Command::NavigateTo(_)));
}

#[test]
fn test_delete_confirmation_flow() {
    let mut state = State::new();

    update(&mut state, Msg::DeleteSelected);
    assert!(state.show_delete_confirmation);

    update(&mut state, Msg::CancelDelete);
    assert!(!state.show_delete_confirmation);
}
```

---

## 6. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Missing widgets delay project** | High | Build widgets first (Phase 1) before migration |
| **Framework limitations discovered** | Medium | Incremental migration allows pivoting |
| **Lost features in migration** | Medium | Comprehensive feature checklist, side-by-side testing |
| **Performance regression** | Low | Benchmark before/after, virtual scrolling |
| **User disruption** | Low | Feature flag for gradual rollout |

---

## 7. Categorized Functionality

### ✅ **Already Supported by Framework**

- **Basic layout**: Column, Row, Panel, Container
- **Text rendering**: Text, StyledText
- **Mouse events**: Click, hover (via InteractionRegistry)
- **Async commands**: Command::perform for background tasks
- **Navigation**: Command::navigate_to between apps
- **Modals**: Stack/Layer for overlays
- **Theming**: Catppuccin colors (need to migrate from old STYLES)
- **Keyboard shortcuts**: Subscription::keyboard

### ❌ **Missing from Framework (BLOCKERS)**

- ~~**List widget**~~ ✅ **DONE** - Used in 6/7 screens
- **TextInput widget** - Required for 3+ modals
- **Tree/Hierarchy widget** - Core of UnifiedCompareScreen
- **Tabs widget** - UnifiedCompareScreen has 4 tabs
- **Scrollable containers** - Long lists
- **Ergonomic macros** - DX improvement (`column![]` vs. builders)

### ⚙️ **Domain-Specific (Implement in migration/ module)**

- Field matching algorithms (exact, prefix, manual)
- Entity metadata fetching (API calls)
- Export to JSON/Excel
- Migration persistence (save/load from config)
- Example record handling

---

## 8. Next Steps & Recommendations

### Immediate Actions

1. ~~**Build List widget**~~ ✅ **DONE**
   - Most critical blocker
   - Used everywhere
   - Enables MigrationSelectApp, EnvironmentSelectApp

2. **Build TextInput widget**
   - Second most critical
   - Required for search, mapping modals

3. **Design Tree widget API**
   - Most complex widget
   - Review proposal_new.md suggestions
   - Consider reusing ratatui-tree-widget crate?

### Medium-Term Goals

4. **Migrate MigrationSelectApp**
   - Validate pattern with simple screen
   - Learn pain points

5. **Migrate EnvironmentSelectApp**
   - Multi-phase selection pattern
   - Refine List widget API

6. **Build Tabs widget + start UnifiedCompareApp**
   - Tackle the big one
   - Iterative refinement

### Long-Term Vision

7. **Extract reusable widgets to framework**
   - entity_tree → Tree widget

8. **Delete old implementation**
   - Remove 36k LOC of old code
   - Celebrate clean architecture 🎉

---

## 9. Open Questions for Discussion

1. **Widget priority**: Agree on List → TextInput → Tree → Tabs order?
2. **Tree widget complexity**: Implement from scratch or use ratatui-tree-widget?
3. **Incremental migration**: Keep both implementations for how long?
4. **Feature parity**: Which features can we defer (e.g., mouse hover on fields)?
5. **Naming**: Keep "migration" name or rename to "entity-compare"?

---

## 10. Task Checklist

### Phase 1: Framework Widgets
- [x] Implement List widget in src/tui/element.rs
  - [x] Basic rendering & selection state
  - [x] Keyboard navigation (Up/Down/PageUp/PageDown/Home/End)
  - [x] Mouse support (click, scroll)
  - [x] Virtual scrolling optimization
  - [ ] Tests
- [ ] Implement TextInput widget
  - [ ] Text editing (insert, delete, cursor movement)
  - [ ] Placeholder text
  - [ ] Password mode
  - [ ] Max length validation
  - [ ] Tests
- [ ] Implement Tree widget
  - [ ] Hierarchical data structure
  - [ ] Expand/collapse functionality
  - [ ] Keyboard navigation
  - [ ] Virtualized rendering
  - [ ] Tests
- [ ] Implement Tabs widget
  - [ ] Tab bar rendering
  - [ ] Content switching
  - [ ] Keyboard navigation (Left/Right)
  - [ ] Tests

### Phase 2: Simple Apps
- [ ] MigrationSelectApp
  - [ ] Define State & Msg types
  - [ ] Implement update() logic
  - [ ] Implement view() with List widget
  - [ ] Add delete confirmation modal
  - [ ] Wire up navigation to other apps
  - [ ] Tests
- [ ] EnvironmentSelectApp
  - [ ] Define State & Msg for two-phase selection
  - [ ] Implement update() logic
  - [ ] Implement view() with dynamic title
  - [ ] Wire up navigation
  - [ ] Tests
- [ ] LoadingApp
  - [ ] Define State with progress tracking
  - [ ] Implement async data fetching
  - [ ] Implement view() with spinner and task list
  - [ ] Handle success/failure states
  - [ ] Tests

### Phase 3: Complex App
- [ ] UnifiedCompareApp - Structure
  - [ ] Define complex State type
  - [ ] Define Msg enum with all actions
  - [ ] Split into sub-modules (state.rs, msg.rs, update.rs, view.rs)
- [ ] UnifiedCompareApp - Fields Tab
  - [ ] Extract field matching logic
  - [ ] Implement fields comparison view
  - [ ] Add field mapping functionality
  - [ ] Prefix mapping modal
  - [ ] Manual mapping modal
- [ ] UnifiedCompareApp - Other Tabs
  - [ ] Relationships tab
  - [ ] Views tab
  - [ ] Forms tab
- [ ] UnifiedCompareApp - Integration
  - [ ] Tab switching logic
  - [ ] Cross-tab communication (pub/sub)
  - [ ] Examples modal
  - [ ] Export functionality
  - [ ] Tests

### Phase 4: Polish
- [x] Migrate to Catppuccin theme colors
- [ ] Add help text/documentation
- [ ] Performance benchmarking
- [ ] Feature parity checklist vs. old implementation
- [ ] User acceptance testing
- [ ] Delete old implementation

---

## Conclusion

The migration module is **substantial** (36k LOC) but **well-suited** for the Elm-inspired pattern. The main blocker is **missing widgets** (List, TextInput, Tree, Tabs).

**Recommended path**:
1. Build framework widgets (Phase 1)
2. Migrate simple screens (Phase 2)
3. Migrate complex screen (Phase 3)
4. Polish & export (Phase 4)

**Payoff**: Clean architecture, testable code, reusable widgets for other apps, deletion of 36k LOC of technical debt.
