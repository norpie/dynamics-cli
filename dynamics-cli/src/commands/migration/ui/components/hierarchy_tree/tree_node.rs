use super::{HierarchyNode, TreeNode};
use std::sync::Arc;

impl TreeNode {
    pub fn new<T: HierarchyNode + 'static>(data: T, level: usize) -> Self {
        Self {
            data: Arc::new(data),
            children: Vec::new(),
            is_expanded: false,
            level,
        }
    }

    pub fn with_children<T: HierarchyNode + 'static>(
        data: T,
        children: Vec<TreeNode>,
        level: usize,
    ) -> Self {
        Self {
            data: Arc::new(data),
            children,
            is_expanded: false,
            level,
        }
    }

    pub fn from_arc(data: Arc<dyn HierarchyNode>, level: usize) -> Self {
        Self {
            data,
            children: Vec::new(),
            is_expanded: false,
            level,
        }
    }

    pub fn from_arc_with_children(
        data: Arc<dyn HierarchyNode>,
        children: Vec<TreeNode>,
        level: usize,
    ) -> Self {
        Self {
            data,
            children,
            is_expanded: false,
            level,
        }
    }
}
