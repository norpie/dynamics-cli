# Proposed Architecture: Elm-Inspired TUI Framework

Complete redesign with only theming staying the same. Here's the full picture:

## Core App Structure

```rust
trait App: Sized + 'static {
    type State: Default;
    type Msg: Clone + Send;

    // Pure state updates
    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg>;

    // Declarative UI
    fn view(state: &Self::State, theme: &Theme) -> Element<Self::Msg>;

    // Input subscriptions (keyboard, timers, events)
    fn subscriptions(state: &Self::State) -> Vec<Subscription<Self::Msg>>;
}
```

## Side Effects via Commands

```rust
enum Command<Msg> {
    None,
    Batch(Vec<Command<Msg>>),

    // Navigation
    NavigateTo(AppId),

    // Async operations
    Perform(Box<dyn Future<Output = Msg>>),

    // Pub/sub between apps
    Publish { topic: String, data: serde_json::Value },

    Quit,
}

// Helper constructors
impl<Msg> Command<Msg> {
    fn perform<F, T>(future: F, to_msg: fn(T) -> Msg) -> Self
    where F: Future<Output = T> + Send + 'static { ... }
}
```

## Declarative UI Elements

```rust
enum Element<Msg> {
    // Primitives
    Text(String),
    Button { label: String, on_press: Msg, on_hover: Option<Msg> },

    // Layout
    Column { children: Vec<Element<Msg>>, spacing: u16 },
    Row { children: Vec<Element<Msg>>, spacing: u16 },

    // Complex widgets
    List { items: Vec<Element<Msg>>, selected: Option<usize>, on_select: fn(usize) -> Msg },
    TextInput { value: String, on_change: fn(String) -> Msg },

    // Styling
    Styled { child: Box<Element<Msg>>, style: Style },
}

// Builder macros for ergonomics
column![
    text("Hello"),
    button("Click me").on_press(Msg::ButtonClicked),
]
```

## Input Subscriptions

```rust
enum Subscription<Msg> {
    // Keyboard shortcuts
    Keyboard(KeyCode, Msg),

    // Periodic tasks
    Timer { interval: Duration, msg: Msg },

    // Event bus
    Subscribe { topic: String, handler: fn(serde_json::Value) -> Option<Msg> },
}
```

## Complete Example

```rust
struct ContactListApp;

#[derive(Clone)]
enum Msg {
    LoadContacts,
    ContactsLoaded(Result<Vec<Contact>>),
    SelectContact(usize),
    ContactClicked,
    ContactHovered(bool),
}

struct State {
    contacts: Vec<Contact>,
    selected: Option<usize>,
    loading: bool,
    hovered: bool,
}

impl App for ContactListApp {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::LoadContacts => {
                state.loading = true;
                Command::perform(
                    api::fetch_contacts(),
                    Msg::ContactsLoaded
                )
            }
            Msg::ContactsLoaded(Ok(contacts)) => {
                state.contacts = contacts;
                state.loading = false;
                Command::None
            }
            Msg::SelectContact(idx) => {
                state.selected = Some(idx);
                Command::publish("contact.selected", contacts[idx].id)
            }
            Msg::ContactClicked => Command::navigate_to(AppId::ContactDetail),
            Msg::ContactHovered(hovered) => {
                state.hovered = hovered;
                Command::None
            }
        }
    }

    fn view(state: &State, theme: &Theme) -> Element<Msg> {
        column![
            text("Contacts").fg(theme.text),

            if state.loading {
                spinner()
            } else {
                list(state.contacts.iter().map(|c|
                    button(&c.name)
                        .on_press(Msg::ContactClicked)
                        .on_hover(Msg::ContactHovered(true))
                        .on_hover_exit(Msg::ContactHovered(false))
                ))
            }
        ]
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            keyboard(KeyCode::F5, Msg::LoadContacts),
        ]
    }
}
```

## Benefits

✅ **Zero boilerplate** - just 3 functions
✅ **Handlers co-located** with UI elements
✅ **No string IDs**, no manual registry calls
✅ **Type-safe** message passing
✅ **Clear data flow**: Event → Msg → Update → Command → View
✅ **Easy async** with Command::perform
✅ **Testable** - update() is pure logic
✅ **Scalable** - pub/sub for inter-app communication