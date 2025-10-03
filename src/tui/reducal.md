# Framework DX Improvements - Boilerplate Reduction

## 1. Complete Field Type System

**Status:** 80% complete (AutocompleteField exists)

Every widget should have a companion Field type that bundles value + state together.

**Current pattern (3 separate fields):**
```rust
struct Form {
    name: String,
    name_input_state: TextInputState,
    source_env: Option<String>,
    source_select_state: SelectState,
}

Msg::NameChanged(key) => {
    if let Some(new) = state.name_input_state.handle_key(key, &state.name, None) {
        state.name = new;
    }
}
```

**Target pattern (value + state bundled):**
```rust
struct Form {
    name: TextInputField,
    source_env: SelectField,
}

Msg::NameEvent(event) => {
    state.form.name.handle_event::<Msg>(event);
    Command::None
}
```

**Required implementations:**
- `TextInputField` (value: String, state: TextInputState)
- `SelectField` (value: Option<String>, state: SelectState)
- `ListField` (items: Vec<T>, state: ListState)
- `TreeField` (root: T, state: TreeState)

**API:**
- `.value() -> &String` / `.value_mut() -> &mut String`
- `.set_value(v: String)`
- `.state() -> &WidgetState` / `.state_mut() -> &mut WidgetState`
- `.handle_event::<Msg>(event, ...)`

**Impact:** Eliminates state/value synchronization bugs, reduces form code by ~60%

---

## 2. Async Resource Automation

**Problem:** Every Resource load requires 2 Msg variants + 8 lines of boilerplate

**Current pattern:**
```rust
Msg::LoadData => {
    state.data = Resource::Loading;
    Command::perform(
        async { fetch().await.map_err(|e| e.to_string()) },
        Msg::DataLoaded
    )
}
Msg::DataLoaded(result) => {
    state.data = Resource::from_result(result);
    Command::None
}
```

**Target pattern (macro):**
```rust
Msg::LoadData => {
    load_resource!(state.data, fetch())
}

// Or with callback:
Msg::LoadData => {
    load_resource!(state.data, fetch(), on_complete: Msg::DataReady)
}
```

**Macro implementation:**
```rust
#[macro_export]
macro_rules! load_resource {
    ($field:expr, $future:expr) => {{
        $field = Resource::Loading;
        Command::perform($future, |result| {
            $field = Resource::from_result(result);
            Command::None
        })
    }};
    ($field:expr, $future:expr, on_complete: $msg:expr) => {{
        $field = Resource::Loading;
        Command::perform($future, |result| {
            $field = Resource::from_result(result);
            Command::from($msg)
        })
    }};
}
```

**Advanced option (derive macro):**
```rust
#[derive(App)]
struct State {
    #[resource(loader = "fetch_migrations", msg = "MigrationsLoaded")]
    migrations: Resource<Vec<Migration>>,
}

// Auto-generates:
// - Msg::MigrationsLoaded(Result<Vec<Migration>, String>)
// - Handler that sets migrations = Resource::from_result(result)
```

**Impact:** Reduces async boilerplate by 50%, eliminates redundant Msg variants

---

## 3. Validation Framework

**Problem:** Validation logic scattered across update handlers (12% of codebase)

**Current pattern (ad-hoc checks):**
```rust
Msg::Submit => {
    let name = state.form.name.trim();
    if name.is_empty() {
        state.validation_error = Some("Name required");
        return Command::None;
    }

    let source = state.form.source_env.clone();
    if source.is_none() {
        state.validation_error = Some("Source required");
        return Command::None;
    }

    let target = state.form.target_env.clone();
    if target.is_none() {
        state.validation_error = Some("Target required");
        return Command::None;
    }

    if source == target {
        state.validation_error = Some("Source and target must differ");
        return Command::None;
    }

    // actual submission logic
}
```

**Target pattern (declarative):**
```rust
#[derive(Validate)]
struct CreateMigrationForm {
    #[validate(not_empty, message = "Name required")]
    name: TextInputField,

    #[validate(required, message = "Source required")]
    source_env: SelectField,

    #[validate(required, different_from = "source_env", message = "Must differ from source")]
    target_env: SelectField,
}

Msg::Submit => {
    match state.form.validate() {
        Ok(validated) => {
            let (name, source, target) = validated.into();
            Command::perform(create_migration(name, source, target), Msg::Created)
        }
        Err(error) => {
            state.form.error = Some(error);
            Command::None
        }
    }
}
```

**Built-in validators:**
- `required` - field must have value
- `not_empty` - string must not be empty/whitespace
- `min_length(n)` - minimum string length
- `max_length(n)` - maximum string length
- `regex(pattern)` - must match regex
- `custom(fn)` - custom validation function

**Cross-field validators:**
- `different_from = "field"` - must differ from another field
- `matches = "field"` - must match another field (password confirmation)
- `one_of = ["field1", "field2"]` - at least one must have value

**Validation trait:**
```rust
pub trait Validate {
    type Output;
    fn validate(&self) -> Result<Self::Output, String>;
}
```

**Impact:** Reduces validation code by 70%, centralizes rules, improves maintainability

---

## 4. CRUD App Generator

**Problem:** Create/Delete/Rename patterns repeated in every app (100+ lines each)

**Current:** Manual implementation of 6 Msg variants, 3 modal handlers, state management

**Target pattern:**
```rust
crud_app! {
    Model: Migration,
    Repository: migrations_repo(),

    List {
        display: |m| format!("{} ({} -> {})", m.name, m.source, m.target),
        on_select: |m| navigate_to_detail(m),
    }

    Create {
        form: CreateMigrationForm,
        action: |form| migrations_repo().create(form),
    }

    Delete {
        confirm: true,
        message: |m| format!("Delete migration '{}'?", m.name),
        action: |m| migrations_repo().delete(m.id),
    }

    Rename {
        form: RenameForm,
        action: |id, name| migrations_repo().rename(id, name),
    }
}
```

**Macro generates:**
- State struct with list_state, modals, forms
- Msg enum with variants:
  - `ListNavigate(KeyCode)`
  - `SelectItem(usize)`
  - `OpenCreateModal`, `CreateFormSubmit`, `CreateFormCancel`, `ItemCreated(Result<T>)`
  - `RequestDelete`, `ConfirmDelete`, `CancelDelete`, `ItemDeleted(Result<()>)`
  - `RequestRename`, `RenameFormSubmit`, `RenameFormCancel`, `ItemRenamed(Result<()>)`
- `update()` with all handlers
- `view()` with conditional modals
- `subscriptions()` with n/N/d/D/r/R keybindings

**Customization points:**
- Custom form rendering via `render_form` callback
- Additional Msg variants via `extend_msg!` block
- Additional update handlers via `extend_update!` block
- Custom list item rendering via ListItem impl

**Impact:** Reduces CRUD apps from ~700 lines to ~50 lines, enforces consistent UX

---

## 5. Widget Message Auto-Routing

**Problem:** All widget events route through global Msg enum, causing bloat

**Current pattern:**
```rust
enum Msg {
    ListNavigate(KeyCode),
    ListActivate(usize),
    NameInputChanged(KeyCode),
    SourceSelectEvent(SelectEvent),
    TargetSelectEvent(SelectEvent),
    // ... 20+ widget variants
}

fn update(state: &mut State, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::ListNavigate(key) => {
            state.list_state.handle_key(key, state.items.len(), 20);
            Command::None
        }
        Msg::NameInputChanged(key) => {
            if let Some(new) = state.name_state.handle_key(key, &state.name, None) {
                state.name = new;
            }
            Command::None
        }
        // ... repeat for every widget
    }
}
```

**Target pattern (auto-routing):**
```rust
#[derive(App)]
struct State {
    #[widget(id = "migration-list")]
    list: ListWidget<Migration>,

    #[widget(id = "name-input")]
    name: TextInputWidget,

    // Only app-specific messages in Msg enum
}

enum Msg {
    MigrationSelected(Migration),  // app logic only
    SubmitForm,
    NavigateBack,
}
```

**How it works:**
- Derive macro generates routing logic
- Widget events intercepted and routed to `widget.handle_internal_event()`
- Only custom callbacks trigger Msg (on_select, on_submit, etc)
- view() uses `widget.render()` instead of manual builders

**WidgetField trait:**
```rust
pub trait WidgetField {
    type Event;
    fn handle_internal_event(&mut self, event: Self::Event) -> Option<Command<Msg>>;
    fn render(&mut self, theme: &Theme) -> Element<Msg>;
}
```

**Impact:** Eliminates ~30% of Msg variants, reduces match arms, cleaner app logic

---

## 6. Declarative Subscriptions Macro

**Problem:** Imperative subscription building is verbose and hard to read

**Current pattern:**
```rust
fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
    let mut subs = vec![];

    if !state.initialized {
        subs.push(Subscription::timer(Duration::from_millis(1), Msg::Initialize));
    }

    if !state.show_create_modal && !state.show_delete_confirm && !state.show_rename_modal {
        subs.push(Subscription::keyboard(KeyCode::Char('n'), "Create new", Msg::OpenCreate));
        subs.push(Subscription::keyboard(KeyCode::Char('N'), "Create new", Msg::OpenCreate));
        subs.push(Subscription::keyboard(KeyCode::Char('d'), "Delete", Msg::RequestDelete));
        subs.push(Subscription::keyboard(KeyCode::Char('D'), "Delete", Msg::RequestDelete));
        subs.push(Subscription::keyboard(KeyCode::Char('r'), "Rename", Msg::RequestRename));
        subs.push(Subscription::keyboard(KeyCode::Char('R'), "Rename", Msg::RequestRename));
    }

    subs.push(Subscription::subscribe("migration:selected", |data| {
        serde_json::from_value::<MigrationMetadata>(data).ok().map(Msg::Initialize)
    }));

    subs
}
```

**Target pattern:**
```rust
subscriptions! {
    timer!(1ms, when: !state.initialized, Msg::Initialize);

    when(!state.show_create_modal && !state.show_delete_confirm && !state.show_rename_modal) {
        key!('n' | 'N', "Create new migration", Msg::OpenCreate);
        key!('d' | 'D', "Delete migration", Msg::RequestDelete);
        key!('r' | 'R', "Rename migration", Msg::RequestRename);
    }

    event!("migration:selected", |data| {
        serde_json::from_value::<MigrationMetadata>(data).ok().map(Msg::Initialize)
    });
}
```

**Macro features:**
- `key!(char | char, "description", Msg)` - multiple keys to same message
- `when(condition) { ... }` - conditional subscriptions
- `timer!(duration, when: condition, Msg)` - conditional timer
- `event!(topic, |data| ...)` - pubsub with inline transformer

**Impact:** More readable, supports key aliases, reduces lines by ~40%

---

## 7. Form Builder DSL

**Problem:** Form UI code is highly repetitive (40% of view() function)

**Current pattern:**
```rust
let name_input = Element::panel(
    Element::text_input("create-name-input", &state.form.name, &state.form.name_input_state)
        .placeholder("Migration name")
        .on_change(Msg::CreateFormNameChanged)
        .build()
).title("Name").build();

let source_select = Element::panel(
    Element::select("create-source-select", source_options, &mut state.form.source_select_state)
        .on_event(Msg::CreateFormSourceEvent)
        .build()
).title("Source Environment").build();

let target_select = Element::panel(
    Element::select("create-target-select", target_options, &mut state.form.target_select_state)
        .on_event(Msg::CreateFormTargetEvent)
        .build()
).title("Target Environment").build();

let buttons = button_row![
    ("create-cancel", "Cancel", Msg::CreateFormCancel),
    ("create-confirm", "Confirm", Msg::CreateFormSubmit),
];

let modal_body = col![
    name_input => Length(3),
    spacer!() => Length(1),
    source_select => Length(10),
    spacer!() => Length(1),
    target_select => Length(10),
    spacer!() => Length(1),
    error_display!(state.form.validation_error, theme) => Length(2),
    buttons => Length(3),
];
```

**Target pattern:**
```rust
form_layout! {
    fields: [
        text("Name", state.form.name, placeholder: "Migration name") => 3,
        spacer => 1,
        select("Source Environment", state.form.source_env, &source_options) => 10,
        spacer => 1,
        select("Target Environment", state.form.target_env, &target_options) => 10,
        spacer => 1,
        error(state.form.validation_error) => 2,
    ],
    buttons: [
        "Cancel" => Msg::Cancel,
        "Create" => Msg::Submit,
    ],
}
```

**Macro features:**
- Auto-generates IDs from field names (kebab-case)
- Auto-wraps fields in Panel with title
- Auto-adds constraint layout
- Auto-wires event handlers (assuming Field types)
- Special `spacer` and `error(Option<String>)` helpers

**Field shortcuts:**
- `text(title, field, [options])` → TextInputField in Panel
- `select(title, field, &options)` → SelectField in Panel
- `autocomplete(title, field, &options)` → AutocompleteField in Panel
- `list(title, field)` → ListField in Panel
- `tree(title, field)` → TreeField in Panel

**Impact:** Reduces form UI code by ~75%, enforces consistent styling

---

## Implementation Priority

### Phase 1 (Quick Wins - 7 hours)
1. ✅ Complete Field Types (2h) - AutocompleteField pattern for all widgets
2. Async Resource Macro (2h) - `load_resource!` macro
3. Declarative Subscriptions (3h) - `subscriptions!` macro

**Expected reduction: ~30% less code**

### Phase 2 (High Impact - 14 hours)
4. Validation Framework (8h) - Derive macro + validators
5. Form Builder DSL (6h) - `form_layout!` macro

**Expected reduction: ~50% less code**

### Phase 3 (Architectural - 36 hours)
6. CRUD Generator (16h) - `crud_app!` macro
7. Widget Auto-Routing (20h) - Requires framework refactor

**Expected reduction: ~70% less code**

---

## Expected Impact on Migration Apps

**Current:**
- MigrationEnvironmentApp: 694 lines
- MigrationComparisonSelectApp: 756 lines

**After Phase 1+2:**
- MigrationEnvironmentApp: ~350 lines (49% reduction)
- MigrationComparisonSelectApp: ~380 lines (50% reduction)

**After Phase 3:**
- MigrationEnvironmentApp: ~200 lines (71% reduction)
- MigrationComparisonSelectApp: ~230 lines (70% reduction)
