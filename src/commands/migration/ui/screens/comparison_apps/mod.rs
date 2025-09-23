pub mod common;
pub mod comparison_app;
pub mod converters;
pub mod matching;

// New unified approach
pub mod unified_hierarchy_node;
pub mod unified_renderer;
pub mod unified_tree;

#[cfg(test)]
mod test_bidirectional_matching;

pub use comparison_app::ComparisonApp;

// New unified exports
