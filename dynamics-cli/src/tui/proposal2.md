# Widget Auto-Routing Proposal

**Status:** Proposal - Not Implemented
**Effort:** 6-8 hours
**Impact:** Eliminates ~90% of widget event boilerplate

---

## Problem Statement

Even with unified event patterns (TextInputEvent, SelectEvent, etc.) and Field types, we still have significant boilerplate:

```rust
// Current state after Phase 1 + 2 improvements
enum Msg {
    NameEvent(TextInputEvent),      // Just a carrier!
    SourceEvent(SelectEvent),        // Just a carrier!
    EntityEvent(AutocompleteEvent),  // Just a carrier!
    SubmitForm,                      // Actual app logic
}

fn update(state: &mut State, msg: Msg) -> Command<Msg> {
    match msg {
        // Pure routing boilerplate - immediately unwrap and delegate
        Msg::NameEvent(e) => {
            state.name.handle_event(e, None);
            Command::None
        }
        Msg::SourceEvent(e) => {
            state.source.handle_event(e, &state.environments);
            Command::None
        }
        Msg::EntityEvent(e) => {
            state.entity.handle_event(e, &state.entities);
            Command::None
        }

        // Actual app logic
        Msg::SubmitForm => { ... }
    }
}

fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
    Element::text_input("name", value, &mut state.name.state)
        .on_event(Msg::NameEvent)  // Boilerplate callback
        .build()
}
```

**The Msg variants are pure boilerplate** - we're just routing events to `field.handle_event()`.

---

## Solution: Invisible Auto-Routing

**Key insight:** Runtime intercepts widget events and dispatches to State BEFORE creating Msg. Only unhandled events become Msg.

### Runtime Flow

```
KeyPressed(key) for FocusId("name-input")
    ↓
1. Get widget event: TextInputEvent::Changed(key)
    ↓
2. Try: state.dispatch_widget_event(FocusId("name-input"), event)
    ↓
3a. If handled (true) → Done, no Msg created!
3b. If not handled (false) → Create Msg and call update()
```

---

## Architecture Changes

### 1. New Trait: `AppState` (Auto-Derived)

```rust
/// Trait for state types that can auto-dispatch widget events
pub trait AppState {
    /// Try to handle widget event internally
    /// Returns true if handled, false if should dispatch to update()
    fn dispatch_widget_event(&mut self, id: &FocusId, event: &dyn Any) -> bool {
        false  // Default: not handled
    }
}
```

**User writes:**

```rust
#[derive(AppState)]
struct State {
    #[widget("name-input")]
    name: TextInputField,

    #[widget("source-select", options = "self.environments")]
    source: SelectField,

    #[widget("entity-autocomplete", options = "self.all_entities")]
    entity: AutocompleteField,

    // Non-widget fields (no attribute)
    environments: Vec<String>,
    all_entities: Vec<String>,
}
```

**Macro generates:**

```rust
impl AppState for State {
    fn dispatch_widget_event(&mut self, id: &FocusId, event: &dyn Any) -> bool {
        match id.0 {
            "name-input" => {
                if let Some(e) = event.downcast_ref::<TextInputEvent>() {
                    self.name.handle_event(e.clone(), None);
                    return true;
                }
            }
            "source-select" => {
                if let Some(e) = event.downcast_ref::<SelectEvent>() {
                    self.source.handle_event::<()>(e.clone(), &self.environments);
                    return true;
                }
            }
            "entity-autocomplete" => {
                if let Some(e) = event.downcast_ref::<AutocompleteEvent>() {
                    self.entity.handle_event::<()>(e.clone(), &self.all_entities);
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}
```

### 2. Dispatch Target Enum

```rust
/// Target for event dispatch - either widget auto-routing or app message
pub enum DispatchTarget<Msg> {
    /// Widget event - try auto-dispatch, then fall back to Msg
    WidgetEvent(Box<dyn Any>),

    /// App message - go directly to update()
    AppMsg(Msg),
}
```

### 3. Runtime Dispatch Logic

```rust
// In Runtime::handle_key_event()
fn handle_key_event(&mut self, key: KeyCode) {
    if let Some(focused_id) = &self.focused_id {
        if let Some(handler) = self.focus_registry.get_handler(focused_id) {

            // Handler returns DispatchTarget instead of Msg
            match handler(key) {
                DispatchTarget::WidgetEvent(boxed_event) => {
                    // Try widget dispatch first
                    if self.state.dispatch_widget_event(focused_id, boxed_event.as_ref()) {
                        return;  // Handled! Done.
                    }
                    // Not handled - would need fallback mechanism
                }
                DispatchTarget::AppMsg(msg) => {
                    // Direct to update()
                    let cmd = Self::App::update(&mut self.state, msg);
                    self.execute_command(cmd);
                }
            }
        }
    }
}
```

### 4. Widget Renderers (Auto-Detect)

Renderers automatically use `WidgetEvent` when no custom callback is provided:

```rust
// In text_input renderer:
fn render_text_input<Msg>(..., on_event: &Option<fn(TextInputEvent) -> Msg>, ...) {

    let on_key: Box<dyn Fn(KeyCode) -> DispatchTarget<Msg>> = if let Some(custom_handler) = on_event {
        // Custom callback provided - use AppMsg
        Box::new(move |key| {
            let event = if key == KeyCode::Enter {
                TextInputEvent::Submit
            } else {
                TextInputEvent::Changed(key)
            };
            DispatchTarget::AppMsg(custom_handler(event))
        })
    } else {
        // No callback - use WidgetEvent for auto-dispatch
        Box::new(|key| {
            let event = if key == KeyCode::Enter {
                TextInputEvent::Submit
            } else {
                TextInputEvent::Changed(key)
            };
            DispatchTarget::WidgetEvent(Box::new(event))
        })
    };

    focus_registry.register_focusable(FocusableInfo {
        id: id.clone(),
        rect: area,
        on_key,
        on_focus: on_focus.clone(),
        on_blur: on_blur.clone(),
        inside_panel,
    });
}
```

### 5. Updated `App` Trait

```rust
pub trait App {
    type State: AppState;  // Now requires AppState
    type Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg>;
    fn view(state: &mut State, theme: &Theme) -> Element<Msg>;
    fn subscriptions(state: &State) -> Vec<Subscription<Msg>>;
}
```

---

## User Experience

### Before (Current - Disgusting Boilerplate)

```rust
enum Msg {
    NameEvent(TextInputEvent),
    SourceEvent(SelectEvent),
    EntityEvent(AutocompleteEvent),
    SubmitForm,
}

struct State {
    name: TextInputField,
    source: SelectField,
    entity: AutocompleteField,
    environments: Vec<String>,
    entities: Vec<String>,
}

fn update(state: &mut State, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::NameEvent(e) => {
            state.name.handle_event(e, None);
            Command::None
        }
        Msg::SourceEvent(e) => {
            state.source.handle_event(e, &state.environments);
            Command::None
        }
        Msg::EntityEvent(e) => {
            state.entity.handle_event(e, &state.entities);
            Command::None
        }
        Msg::SubmitForm => { /* actual logic */ }
    }
}

fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
    col![
        Element::text_input("name", value, &mut state.name.state)
            .on_event(Msg::NameEvent)
            .build(),

        Element::select("source", &mut state.source.state)
            .on_event(Msg::SourceEvent)
            .build(),

        Element::autocomplete("entity", value, &mut state.entity.state)
            .on_event(Msg::EntityEvent)
            .build(),
    ]
}
```

### After (Invisible Auto-Routing)

```rust
// Only app-specific messages!
enum Msg {
    SubmitForm,
}

#[derive(AppState)]
struct State {
    #[widget("name-input")]
    name: TextInputField,

    #[widget("source-select", options = "self.environments")]
    source: SelectField,

    #[widget("entity-autocomplete", options = "self.all_entities")]
    entity: AutocompleteField,

    // Non-widget fields
    environments: Vec<String>,
    all_entities: Vec<String>,
}

fn update(state: &mut State, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::SubmitForm => { /* actual logic */ }
    }
}

fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
    // No .on_event() needed - auto-dispatched!
    col![
        Element::text_input(
            FocusId::new("name-input"),
            state.name.value(),
            &mut state.name.state
        ).build(),

        Element::select(
            FocusId::new("source-select"),
            &mut state.source.state
        ).build(),

        Element::autocomplete(
            FocusId::new("entity-autocomplete"),
            state.entity.value(),
            &mut state.entity.state
        ).build(),
    ]
}
```

### Opt-Out for Custom Logic

When you need custom handling, just don't mark it as a widget:

```rust
#[derive(AppState)]
struct State {
    // Auto-routed
    #[widget("name-input")]
    name: TextInputField,

    // NOT marked as widget - manual handling
    special_input: TextInputField,
}

enum Msg {
    SpecialInputChanged(TextInputEvent),  // Still need this for special case
}

fn update(state: &mut State, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::SpecialInputChanged(e) => {
            // Custom logic here
            state.special_input.handle_event(e, None);

            // Do something special based on input
            if state.special_input.value().len() > 10 {
                return Command::navigate_to(AppId::SomeOtherApp);
            }

            Command::None
        }
    }
}

fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
    col![
        // Auto-routed (no callback)
        Element::text_input("name-input", ...).build(),

        // Manual callback - opts out of auto-dispatch
        Element::text_input("special", ...)
            .on_event(Msg::SpecialInputChanged)
            .build(),
    ]
}
```

---

## Implementation Checklist

1. **Add `AppState` trait** to `dynamics-cli/src/tui/app.rs`
2. **Add `DispatchTarget<Msg>` enum** to `dynamics-cli/src/tui/command.rs`
3. **Create `#[derive(AppState)]` macro** in `dynamics-lib-macros/src/app_state.rs`
   - Parse struct fields for `#[widget(...)]` attributes
   - Generate match arms for each widget
   - Extract `options` attribute for handle_event params
4. **Update `FocusableInfo`** in `dynamics-cli/src/tui/renderer/focus_registry.rs`
   - Change `on_key: Box<dyn Fn(KeyCode) -> Option<Msg>>`
   - To `on_key: Box<dyn Fn(KeyCode) -> DispatchTarget<Msg>>`
5. **Update all widget renderers** to detect custom callbacks:
   - `text_input.rs` - check `on_event`, return WidgetEvent or AppMsg
   - `select.rs` - check `on_event`, return WidgetEvent or AppMsg
   - `autocomplete.rs` - check `on_event`, return WidgetEvent or AppMsg
   - `tree.rs` - check `on_event`, return WidgetEvent or AppMsg
   - `list.rs` - check `on_activate`/`on_navigate`, return WidgetEvent or AppMsg
6. **Update `Runtime::handle_key_event()`** in `dynamics-cli/src/tui/runtime.rs`
   - Try `state.dispatch_widget_event()` for WidgetEvent
   - Call `update()` only for AppMsg or unhandled events
7. **Update `App` trait** in `dynamics-cli/src/tui/app.rs`
   - Add `State: AppState` bound
8. **Update all example apps** to use `#[derive(AppState)]`

---

## Benefits

✅ **Completely invisible** - just add `#[widget("id")]` attribute to State fields
✅ **No Msg bloat** - only app-specific messages in enum
✅ **No update() boilerplate** - widget events handled automatically
✅ **Opt-out available** - use `.on_event()` for custom logic
✅ **Type safe** - macro generates correct downcasts at compile time
✅ **Zero runtime cost** - same dispatch as current, just different path
✅ **Backwards compatible** - opt-in via derive, old code still works

---

## Tradeoffs

### Pros
- Eliminates ~90% of widget event boilerplate
- Makes app code focus on business logic only
- Simple mental model: "widgets handle themselves unless you say otherwise"
- Minimal API surface (just one attribute)

### Cons
- Adds `AppState` trait to architecture
- Uses `dyn Any` for event downcasting (type-erased)
- Slightly more complex runtime dispatch path
- Debugging might be harder (event flow less explicit)
- FocusId strings must match `#[widget("id")]` (compile-time check not possible)

---

## Complexity Assessment

**Previous estimate:** 20 hours (thought it required full architecture overhaul)
**Actual estimate:** 6-8 hours

**Why much simpler:**
- No need to redesign Element<Msg>
- No need for new widget abstraction layer
- Builds on existing Field types and unified events
- Runtime changes are minimal (one dispatch check)
- Most work is in the derive macro

**Breakdown:**
- AppState trait + DispatchTarget enum: 30 min
- Derive macro implementation: 3-4 hours
- Runtime dispatch logic: 1 hour
- Update widget renderers: 1-2 hours
- Testing + example migration: 1-2 hours

---

## Decision

**Recommendation:** Implement this if you find the current event routing "disgusting" (your words).

The effort is relatively low (6-8h) for high impact (eliminates most remaining boilerplate). The architecture changes are minimal and backwards compatible.

**Alternative:** Accept ~50% boilerplate reduction from current work and call it done. The remaining 2 lines per widget (Msg variant + update arm) are acceptable for many use cases.

**Next step:** Prototype the derive macro to validate the approach works as expected.
