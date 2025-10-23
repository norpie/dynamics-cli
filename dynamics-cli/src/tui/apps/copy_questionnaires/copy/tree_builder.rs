use super::domain::*;
use super::tree_items::SnapshotTreeItem;
use serde_json::Value;

/// Helper to build attribute list from all fields in a JSON object
fn build_field_attributes(value: &Value) -> Vec<SnapshotTreeItem> {
    let mut attrs = vec![];

    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            // Skip @OData annotation fields - they're metadata, not actual fields
            if key.contains("@OData") {
                continue;
            }

            // Check if there's a formatted value annotation for this field
            let formatted_key = format!("{}@OData.Community.Display.V1.FormattedValue", key);
            let display_value = if let Some(formatted) = obj.get(&formatted_key).and_then(|v| v.as_str()) {
                // Use the formatted value if available (e.g., user names, lookup names)
                formatted.to_string()
            } else {
                // Otherwise format the value normally
                match val {
                    Value::Null => "null".to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.clone(),
                    Value::Array(_) => "[array]".to_string(),
                    Value::Object(_) => "{object}".to_string(),
                }
            };

            attrs.push(SnapshotTreeItem::Attribute {
                label: key.clone(),
                value: display_value,
            });
        }
    }

    // Sort by key for consistency
    attrs.sort_by(|a, b| {
        if let (SnapshotTreeItem::Attribute { label: a_label, .. },
                SnapshotTreeItem::Attribute { label: b_label, .. }) = (a, b) {
            a_label.cmp(b_label)
        } else {
            std::cmp::Ordering::Equal
        }
    });

    attrs
}

pub fn build_snapshot_tree(questionnaire: &Questionnaire) -> Vec<SnapshotTreeItem> {
    use std::collections::HashMap;

    let mut root_items = vec![];

    // Build lookup maps for conditions (by question_id) and template_lines (by group_id)
    let mut conditions_by_question: HashMap<&str, Vec<&Condition>> = HashMap::new();
    for condition in &questionnaire.conditions {
        // Get the question_id from the condition's raw data
        if let Some(question_id) = condition.raw.get("_nrq_questionid_value").and_then(|v| v.as_str()) {
            conditions_by_question.entry(question_id).or_insert_with(Vec::new).push(condition);
        }
    }

    let mut template_lines_by_group: HashMap<&str, Vec<&TemplateLine>> = HashMap::new();
    for template_line in &questionnaire.template_lines {
        template_lines_by_group.entry(template_line.group_id.as_str()).or_insert_with(Vec::new).push(template_line);
    }

    // Pages > Groups > Questions with Conditions nested
    if !questionnaire.pages.is_empty() {
        let page_children: Vec<SnapshotTreeItem> = questionnaire.pages.iter()
            .map(|page| {
                let group_children: Vec<SnapshotTreeItem> = page.groups.iter()
                    .map(|group| {
                        let question_children: Vec<SnapshotTreeItem> = group.questions.iter()
                            .map(|question| {
                                // Build children for this question
                                let mut question_children_vec = vec![];

                                // Add conditions as expandable section if any exist for this question
                                if let Some(conditions) = conditions_by_question.get(question.id.as_str()) {
                                    let condition_items: Vec<SnapshotTreeItem> = conditions.iter()
                                        .map(|condition| {
                                            let action_children: Vec<SnapshotTreeItem> = condition.actions.iter()
                                                .map(|action| {
                                                    SnapshotTreeItem::Entity {
                                                        name: action.name.clone(),
                                                        id: action.id.clone(),
                                                        children: build_field_attributes(&action.raw),
                                                    }
                                                })
                                                .collect();

                                            // Put actions before condition attributes
                                            let mut condition_children_vec = vec![];
                                            if !condition.actions.is_empty() {
                                                condition_children_vec.push(SnapshotTreeItem::Category {
                                                    name: "Actions".to_string(),
                                                    count: condition.actions.len(),
                                                    children: action_children,
                                                });
                                            }
                                            condition_children_vec.extend(build_field_attributes(&condition.raw));

                                            SnapshotTreeItem::Entity {
                                                name: condition.name.clone(),
                                                id: condition.id.clone(),
                                                children: condition_children_vec,
                                            }
                                        })
                                        .collect();

                                    question_children_vec.push(SnapshotTreeItem::Category {
                                        name: "Conditions".to_string(),
                                        count: condition_items.len(),
                                        children: condition_items,
                                    });
                                }

                                // Add question attributes after conditions
                                question_children_vec.extend(build_field_attributes(&question.raw));

                                SnapshotTreeItem::Entity {
                                    name: question.name.clone(),
                                    id: question.id.clone(),
                                    children: question_children_vec,
                                }
                            })
                            .collect();

                        // Build children for this group
                        let mut group_children_vec = vec![];

                        // Add questions category
                        if !group.questions.is_empty() {
                            group_children_vec.push(SnapshotTreeItem::Category {
                                name: "Questions".to_string(),
                                count: group.questions.len(),
                                children: question_children,
                            });
                        }

                        // Add template lines category if any exist for this group
                        if let Some(template_lines) = template_lines_by_group.get(group.id.as_str()) {
                            let template_line_items: Vec<SnapshotTreeItem> = template_lines.iter()
                                .map(|line| {
                                    SnapshotTreeItem::Entity {
                                        name: format!("Template: {}", line.template.name.clone().unwrap_or_else(|| line.template.id.clone())),
                                        id: line.id.clone(),
                                        children: build_field_attributes(&line.raw),
                                    }
                                })
                                .collect();

                            group_children_vec.push(SnapshotTreeItem::Category {
                                name: "Template Lines".to_string(),
                                count: template_line_items.len(),
                                children: template_line_items,
                            });
                        }

                        // Add group attributes after expandable sections
                        group_children_vec.extend(build_field_attributes(&group.raw));

                        SnapshotTreeItem::Entity {
                            name: group.name.clone(),
                            id: group.id.clone(),
                            children: group_children_vec,
                        }
                    })
                    .collect();

                // Put expandable sections BEFORE attributes
                let mut page_children_vec = vec![];
                if !page.groups.is_empty() {
                    page_children_vec.push(SnapshotTreeItem::Category {
                        name: "Groups".to_string(),
                        count: page.groups.len(),
                        children: group_children,
                    });
                }
                page_children_vec.extend(build_field_attributes(&page.raw));

                SnapshotTreeItem::Entity {
                    name: page.name.clone(),
                    id: page.id.clone(),
                    children: page_children_vec,
                }
            })
            .collect();

        root_items.push(SnapshotTreeItem::Category {
            name: "Pages".to_string(),
            count: questionnaire.pages.len(),
            children: page_children,
        });
    }

    // Page Lines (junction records with ordering)
    if !questionnaire.page_lines.is_empty() {
        let page_line_children: Vec<SnapshotTreeItem> = questionnaire.page_lines.iter()
            .map(|line| {
                let id = line.get("nrq_questionnairepagelineid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let page_id = line.get("_nrq_questionnairepageid_value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let order = line.get("nrq_order")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                SnapshotTreeItem::Entity {
                    name: format!("Page Order {} (Page: {})", order, page_id),
                    id,
                    children: build_field_attributes(line),
                }
            })
            .collect();

        root_items.push(SnapshotTreeItem::Category {
            name: "Page Lines".to_string(),
            count: questionnaire.page_lines.len(),
            children: page_line_children,
        });
    }

    // Group Lines (junction records with ordering)
    if !questionnaire.group_lines.is_empty() {
        let group_line_children: Vec<SnapshotTreeItem> = questionnaire.group_lines.iter()
            .map(|line| {
                let id = line.get("nrq_questiongrouplineid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let group_id = line.get("_nrq_questiongroupid_value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let order = line.get("nrq_order")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                SnapshotTreeItem::Entity {
                    name: format!("Group Order {} (Group: {})", order, group_id),
                    id,
                    children: build_field_attributes(line),
                }
            })
            .collect();

        root_items.push(SnapshotTreeItem::Category {
            name: "Group Lines".to_string(),
            count: questionnaire.group_lines.len(),
            children: group_line_children,
        });
    }

    // Classifications section - show all individual items
    log::debug!("Building classifications tree:");
    log::debug!("  Categories: {}", questionnaire.classifications.categories.len());
    log::debug!("  Domains: {}", questionnaire.classifications.domains.len());
    log::debug!("  Funds: {}", questionnaire.classifications.funds.len());
    log::debug!("  Supports: {}", questionnaire.classifications.supports.len());
    log::debug!("  Types: {}", questionnaire.classifications.types.len());
    log::debug!("  Subcategories: {}", questionnaire.classifications.subcategories.len());
    log::debug!("  Flemish Shares: {}", questionnaire.classifications.flemish_shares.len());

    let mut classification_children = vec![];

    if !questionnaire.classifications.categories.is_empty() {
        let category_items: Vec<SnapshotTreeItem> = questionnaire.classifications.categories.iter()
            .map(|ref_item| SnapshotTreeItem::Entity {
                name: ref_item.name.clone().unwrap_or_else(|| ref_item.id.clone()),
                id: ref_item.id.clone(),
                children: vec![],
            })
            .collect();

        classification_children.push(SnapshotTreeItem::Category {
            name: "Categories".to_string(),
            count: questionnaire.classifications.categories.len(),
            children: category_items,
        });
    }

    if !questionnaire.classifications.domains.is_empty() {
        let domain_items: Vec<SnapshotTreeItem> = questionnaire.classifications.domains.iter()
            .map(|ref_item| SnapshotTreeItem::Entity {
                name: ref_item.name.clone().unwrap_or_else(|| ref_item.id.clone()),
                id: ref_item.id.clone(),
                children: vec![],
            })
            .collect();

        classification_children.push(SnapshotTreeItem::Category {
            name: "Domains".to_string(),
            count: questionnaire.classifications.domains.len(),
            children: domain_items,
        });
    }

    if !questionnaire.classifications.funds.is_empty() {
        let fund_items: Vec<SnapshotTreeItem> = questionnaire.classifications.funds.iter()
            .map(|ref_item| SnapshotTreeItem::Entity {
                name: ref_item.name.clone().unwrap_or_else(|| ref_item.id.clone()),
                id: ref_item.id.clone(),
                children: vec![],
            })
            .collect();

        classification_children.push(SnapshotTreeItem::Category {
            name: "Funds".to_string(),
            count: questionnaire.classifications.funds.len(),
            children: fund_items,
        });
    }

    if !questionnaire.classifications.supports.is_empty() {
        let support_items: Vec<SnapshotTreeItem> = questionnaire.classifications.supports.iter()
            .map(|ref_item| SnapshotTreeItem::Entity {
                name: ref_item.name.clone().unwrap_or_else(|| ref_item.id.clone()),
                id: ref_item.id.clone(),
                children: vec![],
            })
            .collect();

        classification_children.push(SnapshotTreeItem::Category {
            name: "Supports".to_string(),
            count: questionnaire.classifications.supports.len(),
            children: support_items,
        });
    }

    if !questionnaire.classifications.types.is_empty() {
        let type_items: Vec<SnapshotTreeItem> = questionnaire.classifications.types.iter()
            .map(|ref_item| SnapshotTreeItem::Entity {
                name: ref_item.name.clone().unwrap_or_else(|| ref_item.id.clone()),
                id: ref_item.id.clone(),
                children: vec![],
            })
            .collect();

        classification_children.push(SnapshotTreeItem::Category {
            name: "Types".to_string(),
            count: questionnaire.classifications.types.len(),
            children: type_items,
        });
    }

    if !questionnaire.classifications.subcategories.is_empty() {
        let subcategory_items: Vec<SnapshotTreeItem> = questionnaire.classifications.subcategories.iter()
            .map(|ref_item| SnapshotTreeItem::Entity {
                name: ref_item.name.clone().unwrap_or_else(|| ref_item.id.clone()),
                id: ref_item.id.clone(),
                children: vec![],
            })
            .collect();

        classification_children.push(SnapshotTreeItem::Category {
            name: "Subcategories".to_string(),
            count: questionnaire.classifications.subcategories.len(),
            children: subcategory_items,
        });
    }

    if !questionnaire.classifications.flemish_shares.is_empty() {
        let flemish_share_items: Vec<SnapshotTreeItem> = questionnaire.classifications.flemish_shares.iter()
            .map(|ref_item| SnapshotTreeItem::Entity {
                name: ref_item.name.clone().unwrap_or_else(|| ref_item.id.clone()),
                id: ref_item.id.clone(),
                children: vec![],
            })
            .collect();

        classification_children.push(SnapshotTreeItem::Category {
            name: "Flemish Shares".to_string(),
            count: questionnaire.classifications.flemish_shares.len(),
            children: flemish_share_items,
        });
    }

    if !classification_children.is_empty() {
        root_items.push(SnapshotTreeItem::Category {
            name: "Classifications".to_string(),
            count: classification_children.iter().map(|c| {
                if let SnapshotTreeItem::Category { count, .. } = c {
                    *count
                } else {
                    0
                }
            }).sum(),
            children: classification_children,
        });
    }

    root_items
}
