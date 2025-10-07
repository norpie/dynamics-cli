use crossterm::event::KeyCode;
use std::collections::{HashMap, HashSet};
use crate::tui::{Element, Theme};

/// Trait for items that can be displayed in a tree
pub trait TreeItem: Clone {
    type Msg: Clone;

    /// Unique ID for this node (must be stable across frames)
    /// Recommended format: "{name}_{level}" for compatibility with old system
    fn id(&self) -> String;

    /// Check if this node has children
    fn has_children(&self) -> bool;

    /// Get children of this node (only called if has_children() is true)
    fn children(&self) -> Vec<Self>;

    /// Render this node as an Element
    /// depth: indentation level (0 = root)
    /// is_selected: whether this node is currently selected
    /// is_expanded: whether this node is currently expanded
    fn to_element(
        &self,
        theme: &Theme,
        depth: usize,
        is_selected: bool,
        is_expanded: bool,
    ) -> Element<Self::Msg>;
}

/// Trait for items that can be displayed in a table-style tree with columns
///
/// This extends TreeItem to support table rendering with proper column alignment
/// and borders between columns, ideal for queue management UIs.
pub trait TableTreeItem: TreeItem {
    /// Return column values for this row as strings
    ///
    /// Each String represents the content for one column.
    /// The first column will automatically have tree indentation applied.
    ///
    /// # Arguments
    /// * `depth` - indentation level (0 = root)
    /// * `is_selected` - whether this node is currently selected
    /// * `is_expanded` - whether this node is currently expanded
    fn to_table_columns(
        &self,
        depth: usize,
        is_selected: bool,
        is_expanded: bool,
    ) -> Vec<String>;

    /// Define column widths using ratatui Constraints
    ///
    /// This is a static method that returns the layout constraints for all columns.
    /// Example: vec![Constraint::Length(5), Constraint::Fill(1), Constraint::Length(10)]
    fn column_widths() -> Vec<ratatui::layout::Constraint>;

    /// Define column headers
    ///
    /// This is a static method that returns the header labels for each column.
    /// Example: vec!["Pri".to_string(), "Operation".to_string(), "Status".to_string()]
    fn column_headers() -> Vec<String>;
}

/// Manages tree expansion, selection, and scrolling state
#[derive(Debug, Clone)]
pub struct TreeState {
    // Core state
    expanded: HashSet<String>,      // IDs of expanded nodes
    selected: Option<String>,        // Selected node ID
    scroll_offset: usize,
    scroll_off: usize,               // Scrolloff distance (vim-like)
    viewport_height: Option<usize>,  // Last known viewport height from renderer

    // Cached metadata for O(1) lookups (Approach 4 - Smart State)
    node_parents: HashMap<String, String>,   // child_id → parent_id
    node_depths: HashMap<String, usize>,     // id → depth
    visible_order: Vec<String>,              // DFS order of visible nodes
    cache_valid: bool,                       // Whether cache needs rebuild
}

impl Default for TreeState {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeState {
    /// Create a new TreeState with no selection
    pub fn new() -> Self {
        Self {
            expanded: HashSet::new(),
            selected: None,
            scroll_offset: 0,
            scroll_off: 5,
            viewport_height: None,
            node_parents: HashMap::new(),
            node_depths: HashMap::new(),
            visible_order: vec![],
            cache_valid: false,
        }
    }

    /// Set the viewport height (called by renderer with actual area height)
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = Some(height);
    }

    /// Invalidate the metadata cache (forces rebuild on next flatten)
    pub fn invalidate_cache(&mut self) {
        self.cache_valid = false;
    }

    /// Create a new TreeState with first node selected
    pub fn with_selection() -> Self {
        let mut state = Self::new();
        // Selection will be set when tree is first built
        state
    }

    /// Set the scroll-off distance (rows from edge before scrolling)
    pub fn with_scroll_off(mut self, scroll_off: usize) -> Self {
        self.scroll_off = scroll_off;
        self
    }

    /// Get currently selected node ID
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set selected node by ID
    pub fn select(&mut self, node_id: Option<String>) {
        self.selected = node_id;
    }

    /// Check if a node is expanded
    pub fn is_expanded(&self, node_id: &str) -> bool {
        self.expanded.contains(node_id)
    }

    /// Expand a node
    pub fn expand(&mut self, node_id: &str) {
        self.expanded.insert(node_id.to_string());
        self.cache_valid = false;
    }

    /// Collapse a node
    pub fn collapse(&mut self, node_id: &str) {
        self.expanded.remove(node_id);
        self.cache_valid = false;
    }

    /// Toggle expansion of a node
    pub fn toggle(&mut self, node_id: &str) {
        if self.expanded.contains(node_id) {
            self.collapse(node_id);
        } else {
            self.expand(node_id);
        }
    }

    /// Get parent of a node (O(1) with cache)
    pub fn parent_of(&self, node_id: &str) -> Option<&str> {
        self.node_parents.get(node_id).map(|s| s.as_str())
    }

    /// Get depth of a node (O(1) with cache)
    pub fn depth_of(&self, node_id: &str) -> Option<usize> {
        self.node_depths.get(node_id).copied()
    }

    /// Navigate to next visible node
    pub fn navigate_next(&mut self) {
        if let Some(current) = &self.selected {
            if let Some(pos) = self.visible_order.iter().position(|id| id == current) {
                if pos + 1 < self.visible_order.len() {
                    let new_pos = pos + 1;
                    self.selected = Some(self.visible_order[new_pos].clone());

                    // Scroll down incrementally to maintain scrolloff from bottom
                    // Use actual viewport height if available, otherwise don't adjust scroll
                    if let Some(viewport_height) = self.viewport_height {
                        let item_count = self.visible_order.len();

                        // Don't scroll if all items fit
                        if item_count > viewport_height {
                            let max_offset = item_count.saturating_sub(viewport_height);
                            let scroll_trigger = self.scroll_offset + viewport_height - self.scroll_off;

                            if new_pos >= scroll_trigger && self.scroll_offset < max_offset {
                                self.scroll_offset += 1;
                            }
                        }
                    }
                }
            }
        } else if !self.visible_order.is_empty() {
            // No selection, select first
            self.selected = Some(self.visible_order[0].clone());
        }
    }

    /// Navigate to previous visible node
    pub fn navigate_prev(&mut self) {
        if let Some(current) = &self.selected {
            if let Some(pos) = self.visible_order.iter().position(|id| id == current) {
                if pos > 0 {
                    self.selected = Some(self.visible_order[pos - 1].clone());
                    // Adjust scroll if needed (scrolloff logic) - same as List widget
                    if (pos as isize - self.scroll_offset as isize) <= self.scroll_off as isize {
                        self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    }
                }
            }
        } else if !self.visible_order.is_empty() {
            // No selection, select first
            self.selected = Some(self.visible_order[0].clone());
        }
    }

    /// Navigate to parent node
    pub fn navigate_to_parent(&mut self) {
        if let Some(current) = &self.selected {
            if let Some(parent) = self.parent_of(current) {
                self.selected = Some(parent.to_string());
            }
        }
    }

    /// Handle keyboard navigation (returns true if handled)
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Up => {
                self.navigate_prev();
                true
            }
            KeyCode::Down => {
                self.navigate_next();
                true
            }
            KeyCode::Right => {
                // Expand selected node
                if let Some(id) = &self.selected.clone() {
                    if !self.is_expanded(id) {
                        self.toggle(id);
                    }
                }
                true
            }
            KeyCode::Left => {
                // Collapse or jump to parent
                if let Some(id) = &self.selected.clone() {
                    if self.is_expanded(id) {
                        self.toggle(id);
                    } else {
                        self.navigate_to_parent();
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// Update scroll offset based on selection and visible height
    pub fn update_scroll(&mut self, visible_height: usize) {
        if let Some(selected) = &self.selected {
            // Find index in visible order
            if let Some(sel_idx) = self.visible_order.iter().position(|id| id == selected) {
                let item_count = self.visible_order.len();

                // Don't scroll if all items fit on screen
                if item_count <= visible_height {
                    self.scroll_offset = 0;
                    return;
                }

                // Calculate ideal scroll range to keep selection visible with scrolloff
                let min_scroll = sel_idx.saturating_sub(visible_height.saturating_sub(self.scroll_off + 1));
                let max_scroll = sel_idx.saturating_sub(self.scroll_off);

                if self.scroll_offset < min_scroll {
                    self.scroll_offset = min_scroll;
                } else if self.scroll_offset > max_scroll {
                    self.scroll_offset = max_scroll;
                }

                // Final clamp to valid range (prevents empty lines at bottom)
                let max_offset = item_count.saturating_sub(visible_height);
                self.scroll_offset = self.scroll_offset.min(max_offset);
            }
        }
    }

    /// Handle tree event (unified event pattern)
    /// Returns Some(selected_id) on Toggle event, None otherwise
    pub fn handle_event(&mut self, event: crate::tui::widgets::events::TreeEvent) -> Option<String> {
        use crate::tui::widgets::events::TreeEvent;

        match event {
            TreeEvent::Navigate(key) => {
                self.handle_key(key);
                None
            }
            TreeEvent::Toggle => {
                // Toggle current selection
                if let Some(id) = self.selected.clone() {
                    self.toggle(&id);
                }
                None
            }
        }
    }

    /// Rebuild metadata cache from tree structure
    /// This is called internally when cache is invalid
    pub(crate) fn rebuild_metadata<T: TreeItem>(
        &mut self,
        root_items: &[T],
    ) {
        self.node_parents.clear();
        self.node_depths.clear();
        self.visible_order.clear();

        for item in root_items {
            self.build_metadata_recursive(item, None, 0);
        }

        self.cache_valid = true;

        // If no selection and there are items, select first
        if self.selected.is_none() && !self.visible_order.is_empty() {
            self.selected = Some(self.visible_order[0].clone());
        }
    }

    fn build_metadata_recursive<T: TreeItem>(
        &mut self,
        item: &T,
        parent_id: Option<String>,
        depth: usize,
    ) {
        let id = item.id();

        // Record parent relationship
        if let Some(parent) = parent_id {
            self.node_parents.insert(id.clone(), parent);
        }

        // Record depth
        self.node_depths.insert(id.clone(), depth);

        // Add to visible order
        self.visible_order.push(id.clone());

        // Recursively process children if expanded
        if self.is_expanded(&id) && item.has_children() {
            for child in item.children() {
                self.build_metadata_recursive(&child, Some(id.clone()), depth + 1);
            }
        }
    }
}

/// Internal structure for flattened tree nodes
pub(crate) struct FlatNode<Msg> {
    pub id: String,
    pub element: Element<Msg>,
    pub depth: usize,
}

/// Internal structure for flattened table tree nodes
pub struct FlatTableNode {
    pub id: String,
    pub columns: Vec<String>,
    pub depth: usize,
    pub is_selected: bool,
    pub is_expanded: bool,
}

/// Flatten tree into displayable nodes based on expansion state
pub(crate) fn flatten_tree<T: TreeItem>(
    root_items: &[T],
    state: &mut TreeState,
    theme: &Theme,
) -> Vec<FlatNode<T::Msg>> {
    // Rebuild metadata cache if invalid
    if !state.cache_valid {
        state.rebuild_metadata(root_items);
    }

    let mut result = vec![];
    for item in root_items {
        flatten_recursive(item, state, theme, 0, &mut result);
    }
    result
}

fn flatten_recursive<T: TreeItem>(
    item: &T,
    state: &TreeState,
    theme: &Theme,
    depth: usize,
    result: &mut Vec<FlatNode<T::Msg>>,
) {
    let id = item.id();
    let is_expanded = state.is_expanded(&id);
    let is_selected = state.selected() == Some(&id);
    let has_children = item.has_children();

    // Render node (delegates to TreeItem::to_element)
    let element = item.to_element(theme, depth, is_selected, is_expanded);

    result.push(FlatNode {
        id: id.clone(),
        element,
        depth,
    });

    // Recursively flatten children if expanded
    if is_expanded && has_children {
        for child in item.children() {
            flatten_recursive(&child, state, theme, depth + 1, result);
        }
    }
}

/// Flatten table tree into displayable rows based on expansion state
pub(crate) fn flatten_table_tree<T: TableTreeItem>(
    root_items: &[T],
    state: &mut TreeState,
) -> Vec<FlatTableNode> {
    // Rebuild metadata cache if invalid
    if !state.cache_valid {
        state.rebuild_metadata(root_items);
    }

    let mut result = vec![];
    for item in root_items {
        flatten_table_recursive(item, state, 0, &mut result);
    }
    result
}

fn flatten_table_recursive<T: TableTreeItem>(
    item: &T,
    state: &TreeState,
    depth: usize,
    result: &mut Vec<FlatTableNode>,
) {
    let id = item.id();
    let is_expanded = state.is_expanded(&id);
    let is_selected = state.selected() == Some(&id);
    let has_children = item.has_children();

    // Get column data from item
    let columns = item.to_table_columns(depth, is_selected, is_expanded);

    result.push(FlatTableNode {
        id: id.clone(),
        columns,
        depth,
        is_selected,
        is_expanded,
    });

    // Recursively flatten children if expanded
    if is_expanded && has_children {
        for child in item.children() {
            flatten_table_recursive(&child, state, depth + 1, result);
        }
    }
}
