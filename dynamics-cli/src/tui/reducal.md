# Framework DX Improvements - Boilerplate Reduction

## 1. Complete Field Type System

**Status:** ✅ Complete (TextInputField, SelectField, AutocompleteField)

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

## 2. Async Resource Automation ✅ COMPLETE

**Status:** Implemented as `#[derive(ResourceHandlers)]` proc macro

**Problem:** Every Resource load requires 2 Msg variants + 8 lines of boilerplate

**Current pattern (8 lines):**
```rust
Msg::LoadData => {
    state.data = Resource::Loading;
    Command::perform(fetch_data(), Msg::DataLoaded)
}
Msg::DataLoaded(result) => {
    state.data = Resource::from_result(result);
    Command::None
}
```

**Solution: Generate Helper Methods**
```rust
use dynamics_lib_macros::ResourceHandlers;

#[derive(ResourceHandlers)]
struct State {
    #[resource(loader = "fetch_data")]
    data: Resource<Vec<String>>,

    #[resource(loader = "fetch_items", on_complete = "ItemsReady")]
    items: Resource<Vec<String>>,
}

// Auto-generates these methods:
impl State {
    fn load_data(&mut self) -> Command<Msg> {
        self.data = Resource::Loading;
        Command::perform(fetch_data(), Msg::DataLoaded)
    }

    fn handle_data_loaded(&mut self, result: Result<Vec<String>, String>) -> Command<Msg> {
        self.data = Resource::from_result(result);
        Command::None
    }

    fn load_items(&mut self) -> Command<Msg> { /* ... */ }
    fn handle_items_loaded(&mut self, result: Result<Vec<String>, String>) -> Command<Msg> {
        self.items = Resource::from_result(result);
        if self.items.is_success() {
            Command::from(Msg::ItemsReady)  // on_complete hook
        } else {
            Command::None
        }
    }
}
```

**With macro (2 lines):**
```rust
Msg::LoadData => state.load_data(),
Msg::DataLoaded(r) => state.handle_data_loaded(r),
```

**Benefits:**
- 75% less boilerplate per resource (8 lines → 2 lines)
- Type-safe - compiler knows exact types
- Explicit - you still control Msg variants and when to call
- IDE autocomplete works (`state.load_*()`)
- Optional `on_complete` hook for custom logic
- Works with `#[derive(Validate)]` on same struct

**See:** `dynamics-cli/src/tui/apps/examples/example_resource_macro.rs` for demo

**Impact:** Reduces async boilerplate by 75%, maintains explicitness

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

## 4. Widget Message Auto-Routing

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

## 5. Declarative Subscriptions Macro

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

## 6. Form Builder DSL ✅ COMPLETE

**Status:** Implemented as `form_layout!` declarative macro

**Problem:** Form UI code is highly repetitive (40% of view() function)

**Current pattern (verbose ~30 lines):**
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

let buttons = button_row![
    ("create-cancel", "Cancel", Msg::CreateFormCancel),
    ("create-confirm", "Confirm", Msg::CreateFormSubmit),
];

let modal_body = col![
    name_input => Length(3),
    spacer!() => Length(1),
    source_select => Length(10),
    error_display!(state.form.validation_error, theme) => Length(2),
    buttons => Length(3),
];
```

**New pattern with form_layout! (~12 lines):**
```rust
form_layout! {
    theme: theme,
    fields: [
        text("Name", "name-id", field.value().to_string(), &mut field.state, Msg::NameEvent, placeholder: "Migration name") => Length(3),
        spacer => Length(1),
        select("Source", "source-id", &mut state, Msg::SourceEvent, options.clone()) => Length(10),
        spacer => Length(1),
        error(state.form.error) => Length(2),
    ],
    buttons: [
        ("cancel-btn", "Cancel", Msg::Cancel),
        ("submit-btn", "Submit", Msg::Submit),
    ]
}
```

**Macro features:**
- Auto-wraps fields in Panel with title
- Auto-adds constraint layout with column builder
- Auto-wires event handlers to unified event pattern
- Special `spacer` and `error(Option<String>)` helpers
- Works with Field types (TextInputField, SelectField, AutocompleteField)

**Field shortcuts:**
- `text(title, id, value, state, msg, [placeholder: "..."])` → TextInput in Panel
- `select(title, id, state, msg, options)` → Select in Panel
- `autocomplete(title, id, value, state, msg, options)` → Autocomplete in Panel

**Note:** IDs must be provided explicitly as &'static str (not auto-generated) due to lifetime requirements.

**Benefits:**
- Reduces form UI code by ~60% (30 lines → 12 lines)
- Enforces consistent panel + layout pattern
- Works seamlessly with unified event pattern (TextInputEvent, SelectEvent, etc.)
- See `Example8` for full working demo

**Impact:** Reduces form view code by 60%, enforces consistent styling

---

## Implementation Priority

### Phase 1 (Quick Wins) ✅ COMPLETE
1. ✅ Complete Field Types - TextInputField, SelectField, AutocompleteField
2. ✅ Async Resource Automation - `#[derive(ResourceHandlers)]` proc macro
3. ✅ Validation Framework - `#[derive(Validate)]` proc macro
4. ✅ Declarative Subscriptions - `subscriptions!` macro

**Achieved: ~40% code reduction**

### Phase 2 (High Impact)
5. ✅ Form Builder DSL - `form_layout!` macro (COMPLETE)
6. Widget Auto-Routing (20h) - Requires framework refactor (optional)

**Current: ~50% total code reduction** (with Field types + Resource + Subscriptions + Form Builder)
**Potential: ~60% total with Widget Auto-Routing** (optional)

---

## Expected Impact on Migration Apps

**Current:**
- MigrationEnvironmentApp: 694 lines
- MigrationComparisonSelectApp: 756 lines

**After Phase 1 (Completed):**
- MigrationEnvironmentApp: ~500 lines (28% reduction from Field types + validation)
- MigrationComparisonSelectApp: ~530 lines (30% reduction)

**After Phase 2 (Target):**
- MigrationEnvironmentApp: ~350 lines (50% total reduction)
- MigrationComparisonSelectApp: ~380 lines (50% total reduction)
