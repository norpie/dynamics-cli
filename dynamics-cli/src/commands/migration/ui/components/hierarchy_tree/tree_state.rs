use super::{HierarchyNode, TreeNode};

/// Sorting mode for tree nodes
#[derive(Debug, Clone, PartialEq)]
pub enum SortMode {
    Alphabetical,
    ReverseAlphabetical,
}
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use std::collections::HashMap;

/// A hierarchical tree component that supports collapsible nodes
#[derive(Debug, Clone)]
pub struct HierarchyTree {
    /// The root nodes of the tree
    nodes: Vec<TreeNode>,

    /// Track which nodes are expanded
    expanded_state: HashMap<String, bool>,

    /// Current selection state for the flattened view
    pub list_state: ListState,

    /// Cache of the flattened tree for rendering
    flattened_cache: Vec<(TreeNode, usize)>, // (node, level)

    /// Whether the cache needs to be rebuilt
    cache_dirty: bool,
}

impl Default for HierarchyTree {
    fn default() -> Self {
        Self::new()
    }
}

impl HierarchyTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            expanded_state: HashMap::new(),
            list_state: ListState::default(),
            flattened_cache: Vec::new(),
            cache_dirty: true,
        }
    }

    /// Set the root nodes of the tree
    pub fn set_nodes(&mut self, nodes: Vec<TreeNode>) {
        self.nodes = nodes;
        self.cache_dirty = true;
        // Reset selection to first item
        if !self.nodes.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Toggle the expansion state of a node by its key
    pub fn toggle_expansion(&mut self, node_key: &str) {
        let current = self.expanded_state.get(node_key).copied().unwrap_or(false);
        let new_state = !current;
        self.expanded_state.insert(node_key.to_string(), new_state);
        self.cache_dirty = true;

        // Update the expansion state in the actual tree nodes
        Self::update_node_expansion_static(&mut self.nodes, node_key, new_state);
    }

    /// Recursively update expansion state in the tree nodes (static version)
    fn update_node_expansion_static(nodes: &mut [TreeNode], target_key: &str, expanded: bool) {
        for node in nodes {
            if node.data.node_key() == target_key {
                node.is_expanded = expanded;
                return;
            }
            if !node.children.is_empty() {
                Self::update_node_expansion_static(&mut node.children, target_key, expanded);
            }
        }
    }

    /// Ensure the cache is up to date
    fn ensure_cache_fresh(&mut self) {
        if self.cache_dirty {
            self.rebuild_flattened_cache();
            self.cache_dirty = false;
        }
    }

    /// Get the count of flattened items without mutating the cache
    pub fn get_flattened_count(&self) -> usize {
        if self.cache_dirty {
            // If cache is dirty, we need to calculate without updating it
            let mut temp_cache = Vec::new();
            for node in &self.nodes {
                Self::flatten_node_recursive_static(node, 0, &mut temp_cache);
            }
            temp_cache.len()
        } else {
            self.flattened_cache.len()
        }
    }

    /// Get the count of visible items (for display purposes)
    pub fn get_visible_count(&self) -> usize {
        self.get_flattened_count()
    }

    /// Get the flattened view of the tree for rendering
    pub fn get_flattened_items(&mut self) -> &[(TreeNode, usize)] {
        self.ensure_cache_fresh();
        &self.flattened_cache
    }

    /// Rebuild the flattened cache from the current tree state
    fn rebuild_flattened_cache(&mut self) {
        self.flattened_cache.clear();
        for node in &self.nodes {
            Self::flatten_node_recursive_static(node, 0, &mut self.flattened_cache);
        }
    }

    /// Recursively flatten a node and its children if expanded (static version)
    fn flatten_node_recursive_static(
        node: &TreeNode,
        level: usize,
        cache: &mut Vec<(TreeNode, usize)>,
    ) {
        cache.push((node.clone(), level));

        if node.is_expanded {
            for child in &node.children {
                Self::flatten_node_recursive_static(child, level + 1, cache);
            }
        }
    }

    /// Navigate to the next item
    pub fn next(&mut self) {
        // Get length first to avoid borrowing conflict
        self.ensure_cache_fresh();
        let items_len = self.flattened_cache.len();

        if items_len == 0 {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= items_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Navigate to the previous item
    pub fn previous(&mut self) {
        // Get length first to avoid borrowing conflict
        self.ensure_cache_fresh();
        let items_len = self.flattened_cache.len();

        if items_len == 0 {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    items_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Get the currently selected node
    pub fn get_selected(&mut self) -> Option<&TreeNode> {
        self.ensure_cache_fresh();
        if let Some(index) = self.list_state.selected() {
            self.flattened_cache.get(index).map(|(node, _)| node)
        } else {
            None
        }
    }

    /// Toggle expansion of the currently selected node
    pub fn toggle_selected(&mut self) {
        // Get node key first to avoid borrowing conflict
        let node_key = {
            self.ensure_cache_fresh();
            if let Some(index) = self.list_state.selected() {
                if let Some((node, _)) = self.flattened_cache.get(index) {
                    if node.data.is_expandable() {
                        Some(node.data.node_key())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(key) = node_key {
            self.toggle_expansion(&key);
        }
    }

    /// Find node index by its mapping target name (for mirroring)
    pub fn find_node_by_mapping_name(&mut self, target_name: &str) -> Option<usize> {
        self.ensure_cache_fresh();

        for (index, (node, _)) in self.flattened_cache.iter().enumerate() {
            // Check if this node's name matches the target we're looking for
            if node.data.display_name().contains(target_name) {
                return Some(index);
            }
            // Also check mapping target in reverse (for bidirectional matching)
            if let Some(mapping_target) = node.data.mapping_target() {
                if mapping_target == target_name {
                    return Some(index);
                }
            }
        }
        None
    }

    /// Set selection by node name (for mirroring)
    pub fn set_selected_by_name(&mut self, target_name: &str) -> bool {
        if let Some(index) = self.find_node_by_mapping_name(target_name) {
            self.list_state.select(Some(index));
            true
        } else {
            false
        }
    }

    /// Get the current selected node's mapping target (for finding mirror node)
    pub fn get_selected_mapping_target(&mut self) -> Option<String> {
        self.ensure_cache_fresh();
        if let Some(index) = self.list_state.selected() {
            if let Some((node, _)) = self.flattened_cache.get(index) {
                return node.data.mapping_target();
            }
        }
        None
    }

    /// Get the current selected node's display name (for finding mirror node)
    pub fn get_selected_node_name(&mut self) -> Option<String> {
        self.ensure_cache_fresh();
        if let Some(index) = self.list_state.selected() {
            if let Some((node, _)) = self.flattened_cache.get(index) {
                return Some(node.data.clean_name().to_string());
            }
        }
        None
    }

    /// Get the current selected node's expansion state (for mirroring)
    pub fn get_selected_node_expanded_state(&mut self) -> Option<bool> {
        self.ensure_cache_fresh();
        if let Some(index) = self.list_state.selected() {
            if let Some((node, _)) = self.flattened_cache.get(index) {
                if node.data.is_expandable() {
                    let node_key = node.data.node_key();
                    // Default to false (collapsed) if no entry exists, same as toggle_expansion
                    return Some(self.expanded_state.get(&node_key).copied().unwrap_or(false));
                }
            }
        }
        None
    }

    /// Set the current selected node's expansion state (for mirroring)
    pub fn set_selected_node_expanded_state(&mut self, expanded: bool) -> bool {
        self.ensure_cache_fresh();
        if let Some(index) = self.list_state.selected() {
            if let Some((node, _)) = self.flattened_cache.get(index) {
                if node.data.is_expandable() {
                    let node_key = node.data.node_key();

                    // Update both the HashMap AND the actual tree node (like toggle_expansion does)
                    self.expanded_state.insert(node_key.clone(), expanded);
                    self.cache_dirty = true;

                    // Update the expansion state in the actual tree nodes
                    Self::update_node_expansion_static(&mut self.nodes, &node_key, expanded);

                    return true;
                }
            }
        }
        false
    }

    /// Get expansion state of a specific node by name
    pub fn get_node_expand_state(&self, node_name: &str) -> Option<bool> {
        // Try to find by exact key match first
        if let Some(&expanded) = self.expanded_state.get(node_name) {
            return Some(expanded);
        }

        // Fallback: search through all keys for partial matches
        for (key, &expanded) in &self.expanded_state {
            if key.contains(node_name) || node_name.contains(key) {
                return Some(expanded);
            }
        }
        None
    }

    /// Set expansion state of a specific node by name
    pub fn set_node_expand_state(&mut self, node_name: &str, expanded: bool) -> bool {
        // First try to find the exact node key by searching through the tree
        self.ensure_cache_fresh();

        for (node, _) in &self.flattened_cache {
            let clean_name = node.data.clean_name();

            if clean_name == node_name || node.data.display_name().contains(node_name) {
                let node_key = node.data.node_key();
                self.expanded_state.insert(node_key, expanded);
                self.cache_dirty = true;
                return true;
            }
        }

        false
    }

    /// Sort the tree nodes recursively according to the sort mode
    pub fn sort_nodes(&mut self, sort_mode: &SortMode) {
        match sort_mode {
            SortMode::Alphabetical => {
                Self::sort_nodes_recursive_static(&mut self.nodes, false);
            }
            SortMode::ReverseAlphabetical => {
                Self::sort_nodes_recursive_static(&mut self.nodes, true);
            }
        }
        self.cache_dirty = true;
    }

    /// Recursively sort nodes at all levels
    fn sort_nodes_recursive_static(nodes: &mut Vec<TreeNode>, reverse: bool) {
        // Sort current level
        nodes.sort_by(|a, b| {
            // Use clean_name directly instead of parsing display_name
            let clean_name_a = a.data.clean_name().to_lowercase();
            let clean_name_b = b.data.clean_name().to_lowercase();

            let comparison = clean_name_a.cmp(&clean_name_b);
            if reverse {
                comparison.reverse()
            } else {
                comparison
            }
        });

        // Recursively sort children of each node
        for node in nodes.iter_mut() {
            if !node.children.is_empty() {
                Self::sort_nodes_recursive_static(&mut node.children, reverse);
            }
        }
    }


    /// Handle mouse events for the tree
    pub fn handle_mouse_event(&mut self, mouse: &MouseEvent, area: Rect) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.previous();
            }
            MouseEventKind::ScrollDown => {
                self.next();
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // Calculate which row was clicked
                let row = mouse.row.saturating_sub(area.y + 1); // +1 for border
                self.ensure_cache_fresh();

                if let Some(new_index) = row.checked_sub(0)
                    && (new_index as usize) < self.flattened_cache.len()
                {
                    self.list_state.select(Some(new_index as usize));

                    // Double-click or single click to expand/collapse
                    if let Some((node, _)) = self.flattened_cache.get(new_index as usize)
                        && node.data.is_expandable()
                    {
                        self.toggle_expansion(&node.data.node_key());
                    }
                }
            }
            _ => {}
        }
    }
}
