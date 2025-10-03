// Demo of ResourceHandlers macro
use dynamics_lib_macros::ResourceHandlers;
use crossterm::event::KeyCode;
use crate::tui::{App, Command, Element, Subscription, Theme, Resource};

pub struct ExampleResourceMacro;

// Async loader functions
async fn fetch_user() -> Result<String, String> {
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok("John Doe".to_string())
}

async fn fetch_items() -> Result<Vec<String>, String> {
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(vec!["Apple".to_string(), "Banana".to_string(), "Orange".to_string()])
}

#[derive(Clone)]
pub enum Msg {
    LoadUser,
    UserLoaded(Result<String, String>),
    LoadItems,
    ItemsLoaded(Result<Vec<String>, String>),
    Reset,
}

// Apply the ResourceHandlers macro
#[derive(ResourceHandlers)]
pub struct State {
    #[resource(loader = "fetch_user")]
    user: Resource<String>,

    #[resource(loader = "fetch_items")]
    items: Resource<Vec<String>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            user: Resource::NotAsked,
            items: Resource::NotAsked,
        }
    }
}

impl App for ExampleResourceMacro {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            // BEFORE (8 lines):
            // Msg::LoadUser => {
            //     state.user = Resource::Loading;
            //     Command::perform(fetch_user(), Msg::UserLoaded)
            // }
            // Msg::UserLoaded(result) => {
            //     state.user = Resource::from_result(result);
            //     Command::None
            // }

            // AFTER (2 lines) - Using generated methods:
            Msg::LoadUser => state.load_user(),
            Msg::UserLoaded(r) => state.handle_user_loaded(r),

            // Same for items
            Msg::LoadItems => state.load_items(),
            Msg::ItemsLoaded(r) => state.handle_items_loaded(r),

            Msg::Reset => {
                state.user = Resource::NotAsked;
                state.items = Resource::NotAsked;
                Command::None
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        use crate::{col, use_constraints};
        use_constraints!();

        let user_status = match &state.user {
            Resource::NotAsked => "User: Not loaded",
            Resource::Loading => "User: Loading...",
            Resource::Success(name) => &format!("User: {}", name),
            Resource::Failure(err) => &format!("User Error: {}", err),
        };

        let items_status = match &state.items {
            Resource::NotAsked => "Items: Not loaded".to_string(),
            Resource::Loading => "Items: Loading...".to_string(),
            Resource::Success(items) => format!("Items: {}", items.join(", ")),
            Resource::Failure(err) => format!("Items Error: {}", err),
        };

        col![
            Element::text("ResourceHandlers Macro Demo"),
            Element::text(""),
            Element::text(user_status),
            Element::text(&items_status),
            Element::text(""),
            Element::text("Press 'u' to load user, 'i' to load items, 'r' to reset"),
        ]
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Char('u'), "Load user", Msg::LoadUser),
            Subscription::keyboard(KeyCode::Char('i'), "Load items", Msg::LoadItems),
            Subscription::keyboard(KeyCode::Char('r'), "Reset", Msg::Reset),
        ]
    }

    fn title() -> &'static str {
        "Resource Macro Demo"
    }
}
