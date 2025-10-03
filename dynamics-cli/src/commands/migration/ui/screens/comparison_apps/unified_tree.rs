use crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::{layout::Rect, widgets::ListState};

use super::{unified_hierarchy_node::UnifiedHierarchyNode, unified_renderer::UnifiedRenderer};

/// Unified tree component that can handle any hierarchy type
/// Replaces the old HierarchyTree with a simpler, more generic approach
#[derive(Debug)]
pub struct UnifiedTree {
    pub nodes: Vec<UnifiedHierarchyNode>,
    pub list_state: ListState,
}

impl UnifiedTree {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            list_state: ListState::default(),
        }
    }

    /// Create a tree with initial nodes
    pub fn with_nodes(nodes: Vec<UnifiedHierarchyNode>) -> Self {
        Self {
            nodes,
            list_state: ListState::default(),
        }
    }

    /// Set the tree nodes
    pub fn set_nodes(&mut self, nodes: Vec<UnifiedHierarchyNode>) {
        self.nodes = nodes;
        self.list_state = ListState::default(); // Reset selection
    }

    /// Get flattened list of visible nodes (for rendering in lists)
    pub fn get_visible_items(&self) -> Vec<(&UnifiedHierarchyNode, usize)> {
        UnifiedRenderer::get_flattened_visible_nodes(&self.nodes, 0)
    }

    /// Navigate to next item
    pub fn next(&mut self) {
        let visible_items = self.get_visible_items();
        if visible_items.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let next = if selected >= visible_items.len() - 1 {
            0
        } else {
            selected + 1
        };
        self.list_state.select(Some(next));
    }

    /// Navigate to previous item
    pub fn previous(&mut self) {
        let visible_items = self.get_visible_items();
        if visible_items.is_empty() {
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0);
        let previous = if selected == 0 {
            visible_items.len() - 1
        } else {
            selected - 1
        };
        self.list_state.select(Some(previous));
    }

    /// Toggle expansion of currently selected item
    pub fn toggle_selected(&mut self) {
        let node_name = {
            let visible_items = self.get_visible_items();
            if let Some(selected_index) = self.list_state.selected() {
                if let Some((selected_node, _level)) = visible_items.get(selected_index) {
                    selected_node.name.clone()
                } else {
                    return;
                }
            } else {
                return;
            }
        };

        Self::toggle_node_by_name_static(&node_name, &mut self.nodes);
    }

    /// Toggle expansion of a node by name (static version to avoid borrowing issues)
    fn toggle_node_by_name_static(target_name: &str, nodes: &mut [UnifiedHierarchyNode]) -> bool {
        for node in nodes {
            if node.name == target_name {
                node.toggle_expansion();
                return true;
            }

            // Recursively search children
            if Self::toggle_node_by_name_static(target_name, &mut node.children) {
                return true;
            }
        }
        false
    }

    /// Handle mouse events
    pub fn handle_mouse_event(&mut self, mouse: &MouseEvent, area: Rect) {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                // Calculate which item was clicked based on mouse position
                if mouse.row >= area.y && mouse.row < area.y + area.height {
                    let clicked_index = (mouse.row - area.y) as usize;
                    let visible_items = self.get_visible_items();

                    if clicked_index < visible_items.len() {
                        self.list_state.select(Some(clicked_index));
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                self.previous();
            }
            MouseEventKind::ScrollDown => {
                self.next();
            }
            _ => {}
        }
    }

    /// Get currently selected node
    pub fn get_selected_node(&self) -> Option<&UnifiedHierarchyNode> {
        let visible_items = self.get_visible_items();
        if let Some(selected_index) = self.list_state.selected() {
            visible_items
                .get(selected_index)
                .map(|(node, _level)| *node)
        } else {
            None
        }
    }

    /// Get total number of nodes in the tree (for display)
    pub fn get_total_count(&self) -> usize {
        Self::count_nodes_recursive(&self.nodes)
    }

    /// Get number of currently visible nodes (for display)
    pub fn get_visible_count(&self) -> usize {
        self.get_visible_items().len()
    }

    /// Recursively count all nodes in the tree
    fn count_nodes_recursive(nodes: &[UnifiedHierarchyNode]) -> usize {
        let mut count = nodes.len();
        for node in nodes {
            count += Self::count_nodes_recursive(&node.children);
        }
        count
    }

    /// Expand all nodes in the tree
    pub fn expand_all(&mut self) {
        Self::expand_all_recursive(&mut self.nodes);
    }

    /// Collapse all nodes in the tree
    pub fn collapse_all(&mut self) {
        Self::collapse_all_recursive(&mut self.nodes);
    }

    /// Recursively expand all nodes
    fn expand_all_recursive(nodes: &mut [UnifiedHierarchyNode]) {
        for node in nodes {
            if node.is_expandable() {
                node.is_expanded = true;
            }
            Self::expand_all_recursive(&mut node.children);
        }
    }

    /// Recursively collapse all nodes
    fn collapse_all_recursive(nodes: &mut [UnifiedHierarchyNode]) {
        for node in nodes {
            node.is_expanded = false;
            Self::collapse_all_recursive(&mut node.children);
        }
    }

    /// Find a node by name (for debugging/testing)
    pub fn find_node_by_name(&self, target_name: &str) -> Option<&UnifiedHierarchyNode> {
        Self::find_node_by_name_recursive(target_name, &self.nodes)
    }

    /// Recursive helper for finding nodes by name
    fn find_node_by_name_recursive<'a>(
        target_name: &str,
        nodes: &'a [UnifiedHierarchyNode],
    ) -> Option<&'a UnifiedHierarchyNode> {
        for node in nodes {
            if node.name == target_name {
                return Some(node);
            }

            if let Some(found) = Self::find_node_by_name_recursive(target_name, &node.children) {
                return Some(found);
            }
        }
        None
    }
}

impl Default for UnifiedTree {
    fn default() -> Self {
        Self::new()
    }
}

