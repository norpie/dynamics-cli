use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use super::{HierarchyTree, TreeNode};
use crate::{
    commands::migration::ui::components::field_renderer::{
        FieldDisplayInfo, FieldMapping, FieldRenderer, MappingSource,
    },
    commands::migration::ui::screens::comparison::data_models::ExamplesState,
    dynamics::metadata::FieldInfo,
};


impl HierarchyTree {
    /// Render a tree node as a Line for display
    pub fn render_node_line(node: &TreeNode, level: usize) -> Line<'static> {
        let mut spans = Vec::new();

        // Add indentation based on level
        if level > 0 {
            spans.push(Span::styled("  ".repeat(level), Style::default()));
        }

        // Add expand/collapse indicator
        if node.data.is_expandable() {
            let indicator = if node.is_expanded { "▼" } else { "►" };
            spans.push(Span::styled(
                format!("{} ", indicator),
                Style::default().fg(Color::Cyan),
            ));
        } else {
            // Add space for alignment if not expandable
            spans.push(Span::styled("  ", Style::default()));
        }

        // Add the node name - colored by hierarchical match state
        let name_color = Self::get_hierarchical_node_color(node);

        spans.push(Span::styled(
            node.data.display_name(),
            Style::default().fg(name_color),
        ));

        // Add item count for collapsed expandable nodes
        if node.data.is_expandable() && !node.is_expanded {
            spans.push(Span::styled(
                format!(" [{}]", node.data.item_count()),
                Style::default().fg(Color::Gray),
            ));
        }

        // Add mapping indicator only for matched container nodes (not unmapped ones)
        if let Some(target) = node.data.mapping_target() {
            // Only show mapping for container nodes (field nodes handled separately)
            if !node.data.is_field_node() {
                // Use field-style formatting: single arrow → instead of bidirectional ←→
                spans.push(Span::styled(" → ", Style::default().fg(Color::Gray)));

                // Target name in blue (consistent with field formatting)
                spans.push(Span::styled(target, Style::default().fg(Color::Blue)));

                // Show actual mapping type instead of hardcoded "exact"
                if let Some(mapping_type) = Self::get_node_mapping_type(node) {
                    spans.push(Span::styled(
                        format!(" [{}]", mapping_type),
                        Style::default().fg(Color::Gray),
                    ));
                }
            }
        }

        Line::from(spans)
    }

    /// Render a tree node as a Line for display, with field-aware rendering support
    pub fn render_node_line_with_field_data(
        node: &TreeNode,
        level: usize,
        fields: &[FieldInfo],
        examples_state: &ExamplesState,
        is_source: bool,
    ) -> Line<'static> {
        let mut spans = Vec::new();

        // Add indentation based on level
        if level > 0 {
            spans.push(Span::styled("  ".repeat(level), Style::default()));
        }

        // Check if this is a field node that should use rich rendering
        if node.data.is_field_node()
            && let Some(field_info) = node.data.get_field_info()
        {
            // Try to find matching FieldInfo from metadata
            if let Some(field) = fields.iter().find(|f| f.name == field_info.field_name) {
                // Extract example values if examples mode is enabled
                let (source_example_value, target_example_value) = if examples_state.examples_mode_enabled {
                    if is_source {
                        // Rendering source side - current logic is correct
                        let source_value = examples_state.get_example_value(&field.name, true);
                        let target_field_name = field_info.mapping_target.as_ref().unwrap_or(&field.name);
                        log::debug!("🔍 SOURCE SIDE Field {}: mapping_target = {:?}, using target_field_name = {}",
                                   field.name, field_info.mapping_target, target_field_name);
                        let target_value = examples_state.get_example_value(target_field_name, false);
                        (source_value, target_value)
                    } else {
                        // Rendering target side - reverse the logic
                        let target_value = examples_state.get_example_value(&field.name, false);
                        let source_field_name = field_info.mapping_target.as_ref().unwrap_or(&field.name);
                        log::debug!("🎯 TARGET SIDE Field {}: mapping_target = {:?}, using source_field_name = {}",
                                   field.name, field_info.mapping_target, source_field_name);
                        let source_value = examples_state.get_example_value(source_field_name, true);
                        (source_value, target_value)
                    }
                } else {
                    (None, None)
                };

                // Create FieldDisplayInfo with actual FieldInfo from metadata
                let field_display_info = if let Some(mapping_target) = &field_info.mapping_target {
                    let mapping = FieldMapping {
                        mapped_field_name: mapping_target.clone(),
                        mapping_source: field_info
                            .mapping_source
                            .clone()
                            .unwrap_or(MappingSource::Manual),
                        match_state: field_info.match_state.clone(),
                    };
                    FieldDisplayInfo::with_mapping(field.clone(), mapping)
                        .with_example_values(source_example_value, target_example_value)
                } else {
                    FieldDisplayInfo::new(field.clone())
                        .with_example_values(source_example_value, target_example_value)
                };

                // Use FieldRenderer for rich rendering but skip indentation (already added)
                let field_line = FieldRenderer::render_field_line_with_context(
                    &field_display_info,
                    Some(is_source),
                    examples_state.examples_mode_enabled
                );

                // Extract spans from field rendering, skip indentation we already added
                let field_spans: Vec<Span> = field_line.spans.into_iter().collect();
                spans.extend(field_spans);

                return Line::from(spans);
            } else {
                // No matching FieldInfo found (e.g., ViewItems with column names)
                // Create a synthetic FieldInfo from the rendering info
                let synthetic_field = FieldInfo {
                    name: field_info.field_name.clone(),
                    field_type: field_info.field_type.clone(),
                    is_required: field_info.is_required,
                    is_custom: false, // Default for synthetic fields
                };

                // Extract example values if examples mode is enabled
                let (source_example_value, target_example_value) = if examples_state.examples_mode_enabled {
                    let source_value = examples_state.get_example_value(&synthetic_field.name, true);
                    // For target, use the mapped field name if available
                    let target_field_name = field_info.mapping_target.as_ref().unwrap_or(&synthetic_field.name);
                    let target_value = examples_state.get_example_value(target_field_name, false);
                    (source_value, target_value)
                } else {
                    (None, None)
                };

                let field_display_info = if let Some(mapping_target) = &field_info.mapping_target {
                    let mapping = FieldMapping {
                        mapped_field_name: mapping_target.clone(),
                        mapping_source: field_info
                            .mapping_source
                            .clone()
                            .unwrap_or(MappingSource::Manual),
                        match_state: field_info.match_state.clone(),
                    };
                    FieldDisplayInfo::with_mapping(synthetic_field, mapping)
                        .with_example_values(source_example_value, target_example_value)
                } else {
                    FieldDisplayInfo::new(synthetic_field)
                        .with_example_values(source_example_value, target_example_value)
                };

                // Use FieldRenderer for rich rendering
                let field_line = FieldRenderer::render_field_line_with_context(
                    &field_display_info,
                    Some(is_source),
                    examples_state.examples_mode_enabled
                );

                // Extract spans from field rendering, skip indentation we already added
                let field_spans: Vec<Span> = field_line.spans.into_iter().collect();
                spans.extend(field_spans);

                return Line::from(spans);
            }
        }

        // Default rendering for non-field nodes or if field info not found

        // Add expand/collapse indicator
        if node.data.is_expandable() {
            let indicator = if node.is_expanded { "▼" } else { "►" };
            spans.push(Span::styled(
                format!("{} ", indicator),
                Style::default().fg(Color::Cyan),
            ));
        } else {
            // Add space for alignment if not expandable
            spans.push(Span::styled("  ", Style::default()));
        }

        // Add the node name - colored by hierarchical match state
        let name_color = Self::get_hierarchical_node_color(node);

        spans.push(Span::styled(
            node.data.display_name(),
            Style::default().fg(name_color),
        ));

        // Add item count for collapsed expandable nodes
        if node.data.is_expandable() && !node.is_expanded {
            spans.push(Span::styled(
                format!(" [{}]", node.data.item_count()),
                Style::default().fg(Color::Gray),
            ));
        }

        // Add mapping indicator only for matched container nodes (not unmapped ones)
        if let Some(target) = node.data.mapping_target() {
            // Only show mapping for container nodes (field nodes handled above with FieldRenderer)
            if !node.data.is_field_node() {
                // Use field-style formatting: single arrow → instead of bidirectional ←→
                spans.push(Span::styled(" → ", Style::default().fg(Color::Gray)));

                // Target name in blue (consistent with field formatting)
                spans.push(Span::styled(target, Style::default().fg(Color::Blue)));

                // Show actual mapping type instead of hardcoded "exact"
                if let Some(mapping_type) = Self::get_node_mapping_type(node) {
                    spans.push(Span::styled(
                        format!(" [{}]", mapping_type),
                        Style::default().fg(Color::Gray),
                    ));
                }
            }
        }

        Line::from(spans)
    }

    /// Get the mapping type string from a tree node (if available)
    fn get_node_mapping_type(node: &TreeNode) -> Option<String> {
        node.data.mapping_type()
    }

    /// Get the appropriate color for a mapping based on hierarchy node state
    fn get_mapping_color(node: &TreeNode) -> Color {
        if node.data.is_field_node() {
            // Field nodes use their own FieldRenderer colors
            return Color::Green;
        }

        // For container nodes, analyze child match states
        Self::calculate_hierarchical_color(&node.children)
    }

    /// Get the hierarchical color for a node based on its mapping and ALL descendants
    fn get_hierarchical_node_color(node: &TreeNode) -> Color {
        // Rule 1: If node is unmapped, always red
        if node.data.mapping_target().is_none() {
            return Color::Red;
        }

        // Rule 2: Node is mapped - check if ANY descendant (at any depth) is unmapped
        if Self::has_any_unmapped_descendant(node) {
            return Color::Yellow; // Yellow: me mapped, but some descendant unmapped
        }

        // Rule 3: Node mapped and ALL descendants mapped
        Color::Green
    }

    /// Recursively check if ANY descendant (at any depth) is unmapped
    fn has_any_unmapped_descendant(node: &TreeNode) -> bool {
        for child in &node.children {
            // If this direct child is unmapped, we found an unmapped descendant
            if child.data.mapping_target().is_none() {
                return true;
            }

            // Recursively check if this child has any unmapped descendants
            if Self::has_any_unmapped_descendant(child) {
                return true;
            }
        }

        // No unmapped descendants found
        false
    }

    /// Calculate hierarchical color based on child node states
    fn calculate_hierarchical_color(children: &[TreeNode]) -> Color {
        if children.is_empty() {
            return Color::Green; // No children means perfect match
        }

        let mut matched_count = 0;
        let mut total_count = 0;

        for child in children {
            total_count += 1;

            if child.data.mapping_target().is_some() {
                matched_count += 1;
            }

            // Recursively check grandchildren for deeper analysis
            let child_color = Self::calculate_hierarchical_color(&child.children);
            if child_color == Color::Yellow {
                // If any child is yellow (mixed), this container should also be yellow
                return Color::Yellow;
            }
        }

        // Determine color based on match ratio
        if matched_count == 0 {
            Color::Red // No matches
        } else if matched_count == total_count {
            Color::Green // Perfect match (all children matched)
        } else {
            Color::Yellow // Mixed match (some children matched)
        }
    }
}
