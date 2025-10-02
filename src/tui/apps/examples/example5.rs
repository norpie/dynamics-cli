use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, Command, Element, Subscription, Theme, LayoutConstraint, FocusId};
use crate::tui::widgets::{TreeItem, TreeState, TreeEvent};

pub struct Example5;

#[derive(Clone)]
pub enum Msg {
    LeftTreeEvent(TreeEvent),
    RightTreeEvent(TreeEvent),
}

pub struct State {
    left_tree: TreeState,
    right_tree: TreeState,
    left_root: Vec<FileNode>,
    right_root: Vec<FileNode>,
}

/// Represents a file or folder in the tree
#[derive(Clone, Debug)]
struct FileNode {
    name: String,
    path: String,  // Full path for unique ID
    node_type: NodeType,
    level: usize,
    children: Vec<FileNode>,
}

#[derive(Clone, Debug, PartialEq)]
enum NodeType {
    Folder,
    RustFile,
    TomlFile,
    MarkdownFile,
    TextFile,
}

impl FileNode {
    fn folder(name: impl Into<String>, path: impl Into<String>, level: usize, children: Vec<FileNode>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            node_type: NodeType::Folder,
            level,
            children,
        }
    }

    fn rust_file(name: impl Into<String>, path: impl Into<String>, level: usize) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            node_type: NodeType::RustFile,
            level,
            children: vec![],
        }
    }

    fn toml_file(name: impl Into<String>, path: impl Into<String>, level: usize) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            node_type: NodeType::TomlFile,
            level,
            children: vec![],
        }
    }

    fn markdown_file(name: impl Into<String>, path: impl Into<String>, level: usize) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            node_type: NodeType::MarkdownFile,
            level,
            children: vec![],
        }
    }

    fn text_file(name: impl Into<String>, path: impl Into<String>, level: usize) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            node_type: NodeType::TextFile,
            level,
            children: vec![],
        }
    }

    fn icon(&self) -> &'static str {
        match self.node_type {
            NodeType::Folder => "📁",
            NodeType::RustFile => "🦀",
            NodeType::TomlFile => "⚙️",
            NodeType::MarkdownFile => "📝",
            NodeType::TextFile => "📄",
        }
    }
}

impl TreeItem for FileNode {
    type Msg = Msg;

    fn id(&self) -> String {
        self.path.clone()
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    fn children(&self) -> Vec<Self> {
        self.children.clone()
    }

    fn to_element(
        &self,
        theme: &Theme,
        depth: usize,
        is_selected: bool,
        is_expanded: bool,
    ) -> Element<Msg> {
        // Build indentation
        let indent = "  ".repeat(depth);

        // Build expander icon
        let expander = if self.has_children() {
            if is_expanded { "▼ " } else { "▶ " }
        } else {
            "  "
        };

        // Choose color based on file type
        let file_color = match self.node_type {
            NodeType::Folder => theme.blue,
            NodeType::RustFile => theme.peach,
            NodeType::TomlFile => theme.yellow,
            NodeType::MarkdownFile => theme.green,
            NodeType::TextFile => theme.text,
        };

        // Build the line
        let mut spans = vec![
            Span::raw(indent),
            Span::styled(expander.to_string(), Style::default().fg(theme.overlay1)),
            Span::raw(format!("{} ", self.icon())),
            Span::styled(self.name.clone(), Style::default().fg(file_color)),
        ];

        // Add child count for folders
        if self.node_type == NodeType::Folder && !self.children.is_empty() {
            spans.push(Span::styled(
                format!(" ({})", self.children.len()),
                Style::default().fg(theme.overlay1),
            ));
        }

        let line = Line::from(spans);

        // Apply background if selected
        let mut builder = Element::styled_text(line);
        if is_selected {
            builder = builder.background(Style::default().bg(theme.surface0));
        }

        builder.build()
    }
}

impl Default for State {
    fn default() -> Self {
        // Create sample project structure - left tree (Rust project)
        let left_root = vec![
            FileNode::folder("dynamics-cli", "dynamics-cli", 0, vec![
                FileNode::folder("src", "dynamics-cli/src", 1, vec![
                    FileNode::rust_file("main.rs", "dynamics-cli/src/main.rs", 2),
                    FileNode::rust_file("lib.rs", "dynamics-cli/src/lib.rs", 2),
                    FileNode::folder("commands", "dynamics-cli/src/commands", 2, vec![
                        FileNode::rust_file("mod.rs", "dynamics-cli/src/commands/mod.rs", 3),
                        FileNode::rust_file("migration.rs", "dynamics-cli/src/commands/migration.rs", 3),
                        FileNode::rust_file("query.rs", "dynamics-cli/src/commands/query.rs", 3),
                    ]),
                    FileNode::folder("tui", "dynamics-cli/src/tui", 2, vec![
                        FileNode::rust_file("mod.rs", "dynamics-cli/src/tui/mod.rs", 3),
                        FileNode::rust_file("app.rs", "dynamics-cli/src/tui/app.rs", 3),
                        FileNode::rust_file("runtime.rs", "dynamics-cli/src/tui/runtime.rs", 3),
                        FileNode::folder("widgets", "dynamics-cli/src/tui/widgets", 3, vec![
                            FileNode::rust_file("list.rs", "dynamics-cli/src/tui/widgets/list.rs", 4),
                            FileNode::rust_file("tree.rs", "dynamics-cli/src/tui/widgets/tree.rs", 4),
                            FileNode::rust_file("text_input.rs", "dynamics-cli/src/tui/widgets/text_input.rs", 4),
                        ]),
                    ]),
                ]),
                FileNode::folder("tests", "dynamics-cli/tests", 1, vec![
                    FileNode::rust_file("integration_test.rs", "dynamics-cli/tests/integration_test.rs", 2),
                ]),
                FileNode::toml_file("Cargo.toml", "dynamics-cli/Cargo.toml", 1),
                FileNode::markdown_file("README.md", "dynamics-cli/README.md", 1),
                FileNode::text_file(".gitignore", "dynamics-cli/.gitignore", 1),
            ]),
        ];

        // Create sample project structure - right tree (Smaller project)
        let right_root = vec![
            FileNode::folder("web-app", "web-app", 0, vec![
                FileNode::folder("src", "web-app/src", 1, vec![
                    FileNode::rust_file("main.rs", "web-app/src/main.rs", 2),
                    FileNode::folder("api", "web-app/src/api", 2, vec![
                        FileNode::rust_file("mod.rs", "web-app/src/api/mod.rs", 3),
                        FileNode::rust_file("routes.rs", "web-app/src/api/routes.rs", 3),
                    ]),
                    FileNode::folder("models", "web-app/src/models", 2, vec![
                        FileNode::rust_file("user.rs", "web-app/src/models/user.rs", 3),
                        FileNode::rust_file("session.rs", "web-app/src/models/session.rs", 3),
                    ]),
                ]),
                FileNode::toml_file("Cargo.toml", "web-app/Cargo.toml", 1),
                FileNode::markdown_file("README.md", "web-app/README.md", 1),
            ]),
        ];

        Self {
            left_tree: TreeState::with_selection(),
            right_tree: TreeState::new(),
            left_root,
            right_root,
        }
    }
}

impl App for Example5 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::LeftTreeEvent(event) => {
                state.left_tree.handle_event(event);
                Command::None
            }
            Msg::RightTreeEvent(event) => {
                state.right_tree.handle_event(event);
                Command::None
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        // Build left tree
        let left_tree = Element::tree(
            FocusId::new("left_tree"),
            &state.left_root,
            &mut state.left_tree,
            theme,
        )
        .on_event(Msg::LeftTreeEvent)
        .build();

        // Build right tree
        let right_tree = Element::tree(
            FocusId::new("right_tree"),
            &state.right_root,
            &mut state.right_tree,
            theme,
        )
        .on_event(Msg::RightTreeEvent)
        .build();

        // Wrap in panels
        let left_panel = Element::panel(left_tree)
            .title("Project A")
            .build();

        let right_panel = Element::panel(right_tree)
            .title("Project B")
            .build();

        // Layout side by side
        Element::row(vec![left_panel, right_panel]).build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![]
    }

    fn title() -> &'static str {
        "Example 5"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        let left_selected = state.left_tree.selected().unwrap_or("none").to_string();
        let right_selected = state.right_tree.selected().unwrap_or("none").to_string();

        Some(Line::from(vec![
            Span::styled("Left: ".to_string(), Style::default().fg(theme.overlay1)),
            Span::styled(left_selected, Style::default().fg(theme.blue)),
            Span::raw("  ".to_string()),
            Span::styled("Right: ".to_string(), Style::default().fg(theme.overlay1)),
            Span::styled(right_selected, Style::default().fg(theme.blue)),
            Span::raw("  ".to_string()),
            Span::styled("Arrow keys: navigate".to_string(), Style::default().fg(theme.overlay1)),
            Span::raw(" | ".to_string()),
            Span::styled("Enter: toggle".to_string(), Style::default().fg(theme.overlay1)),
        ]))
    }
}
