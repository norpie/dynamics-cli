use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::commands::migration::ui::components::field_renderer::{
    FieldDisplayInfo, FieldMapping, FieldRenderer, MappingSource, MatchState,
};

use super::unified_hierarchy_node::{MappingType, UnifiedHierarchyNode};

/// Unified renderer that handles all hierarchy types with a single rendering function
pub struct UnifiedRenderer;

impl UnifiedRenderer {
    /// Universal rendering function that works for any hierarchy node
    pub fn render_node(node: &UnifiedHierarchyNode, level: usize) -> Line<'static> {
        let mut spans = Vec::new();

        // Add indentation based on level
        if level > 0 {
            spans.push(Span::styled("  ".repeat(level), Style::default()));
        }

        // If this is a field node, use FieldRenderer for rich rendering
        if let Some(field_info) = &node.field_info {
            return Self::render_field_node(node, field_info, level);
        }

        // Container node rendering
        Self::render_container_node(node, &mut spans);

        Line::from(spans)
    }

    /// Render a field node using the unified FieldRenderer
    fn render_field_node(
        node: &UnifiedHierarchyNode,
        field_info: &crate::dynamics::metadata::FieldInfo,
        level: usize,
    ) -> Line<'static> {
        // Create FieldDisplayInfo from the unified node
        let field_display_info = if let Some(mapping_target) = &node.mapping_target {
            // Field has a mapping
            let mapping = FieldMapping {
                mapped_field_name: mapping_target.clone(),
                mapping_source: Self::convert_mapping_type_to_source(&node.mapping_type),
                match_state: Self::convert_mapping_type_to_match_state(&node.mapping_type),
            };

            FieldDisplayInfo::with_mapping(field_info.clone(), mapping)
        } else {
            // Field has no mapping
            FieldDisplayInfo::new(field_info.clone())
        };

        // Use FieldRenderer but add indentation
        let field_line = FieldRenderer::render_field_line(&field_display_info);

        // Add indentation to the field line
        if level > 0 {
            let indent = "  ".repeat(level);
            let mut spans = vec![Span::styled(indent, Style::default())];
            spans.extend(field_line.spans);
            Line::from(spans)
        } else {
            field_line
        }
    }

    /// Render a container node (non-field)
    fn render_container_node(node: &UnifiedHierarchyNode, spans: &mut Vec<Span<'static>>) {
        // Add expand/collapse indicator
        if node.is_expandable() {
            let indicator = if node.is_expanded { "▼" } else { "►" };
            spans.push(Span::styled(
                format!("{} ", indicator),
                Style::default().fg(Color::Cyan),
            ));
        } else {
            // Add space for alignment if not expandable
            spans.push(Span::styled("  ", Style::default()));
        }

        // Add the node name with icon
        spans.push(Span::styled(
            format!("{} {}", node.icon, node.name),
            Style::default().fg(Color::White),
        ));

        // Add item count for collapsed expandable nodes
        if node.is_expandable() && !node.is_expanded && node.item_count > 0 {
            spans.push(Span::styled(
                format!(" [{}]", node.item_count),
                Style::default().fg(Color::Gray),
            ));
        }

        // Add bidirectional mapping indicator only for matched nodes
        if let Some(_target) = &node.mapping_target {
            // Only show bidirectional indicator for actual matches
            match node.mapping_type {
                MappingType::Exact
                | MappingType::FullMatch
                | MappingType::Prefix
                | MappingType::Manual => {
                    spans.push(Span::styled(" ←→ ", Style::default().fg(Color::Green)));
                    spans.push(Span::styled(
                        format!("[{}]", Self::mapping_type_display(&node.mapping_type)),
                        Style::default().fg(Color::Gray),
                    ));
                }
                MappingType::Mixed => {
                    spans.push(Span::styled(" ←→ ", Style::default().fg(Color::Yellow)));
                    spans.push(Span::styled(
                        format!("[{}]", Self::mapping_type_display(&node.mapping_type)),
                        Style::default().fg(Color::Gray),
                    ));
                }
                MappingType::Unmapped => {
                    // Don't show anything for unmapped nodes
                }
            }
        }
    }

    /// Convert unified MappingType to FieldRenderer MappingSource
    fn convert_mapping_type_to_source(mapping_type: &MappingType) -> MappingSource {
        match mapping_type {
            MappingType::Exact | MappingType::FullMatch => MappingSource::Exact,
            MappingType::Prefix => MappingSource::Prefix,
            MappingType::Manual => MappingSource::Manual,
            MappingType::Mixed => MappingSource::Manual, // Mixed mappings show as manual
            MappingType::Unmapped => MappingSource::Exact, // Default fallback
        }
    }

    /// Convert unified MappingType to FieldRenderer MatchState
    fn convert_mapping_type_to_match_state(mapping_type: &MappingType) -> MatchState {
        match mapping_type {
            MappingType::FullMatch | MappingType::Exact | MappingType::Manual => {
                MatchState::FullMatch
            }
            MappingType::Prefix => MatchState::FullMatch, // Prefix matches are still good matches
            MappingType::Mixed => MatchState::MixedMatch, // Partial hierarchical matches
            MappingType::Unmapped => MatchState::NoMatch,
        }
    }

    /// Get display string for mapping type
    fn mapping_type_display(mapping_type: &MappingType) -> &'static str {
        match mapping_type {
            MappingType::Exact => "exact",
            MappingType::Prefix => "prefix",
            MappingType::Manual => "manual",
            MappingType::FullMatch => "full",
            MappingType::Mixed => "mixed",
            MappingType::Unmapped => "",
        }
    }

    /// Render a complete tree recursively
    pub fn render_tree(nodes: &[UnifiedHierarchyNode], level: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for node in nodes {
            // Render this node
            lines.push(Self::render_node(node, level));

            // If expanded and has children, render children recursively
            if node.is_expanded && !node.children.is_empty() {
                let child_lines = Self::render_tree(&node.children, level + 1);
                lines.extend(child_lines);
            }
        }

        lines
    }

    /// Get flattened list of visible nodes for list rendering
    pub fn get_flattened_visible_nodes(
        nodes: &[UnifiedHierarchyNode],
        level: usize,
    ) -> Vec<(&UnifiedHierarchyNode, usize)> {
        let mut result = Vec::new();

        for node in nodes {
            // Add this node
            result.push((node, level));

            // If expanded and has children, add children recursively
            if node.is_expanded && !node.children.is_empty() {
                let child_nodes = Self::get_flattened_visible_nodes(&node.children, level + 1);
                result.extend(child_nodes);
            }
        }

        result
    }
}

