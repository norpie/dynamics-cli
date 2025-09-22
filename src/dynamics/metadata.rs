use anyhow::Result;
use log::debug;
use roxmltree::Document;

// Forward declaration for FieldInfo since it's defined in the compare module
// We'll need to move this struct to a shared location or define it here
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: String,
    pub is_required: bool,
    pub is_custom: bool,
}

#[derive(Debug, Clone)]
pub struct ViewInfo {
    pub name: String,
    pub entity_name: String,
    pub view_type: String,
    pub is_custom: bool,
    pub columns: Vec<ViewColumn>,
    pub fetch_xml: String,
}

#[derive(Debug, Clone)]
pub struct ViewColumn {
    pub name: String,
    pub width: Option<u32>,
    pub is_primary: bool,
}

/// Parse Dynamics 365 metadata XML and extract field information for a specific entity
pub fn parse_entity_fields(metadata_xml: &str, entity_name: &str) -> Result<Vec<FieldInfo>> {
    let doc = Document::parse(metadata_xml)
        .map_err(|e| anyhow::anyhow!("Failed to parse metadata XML: {}", e))?;

    debug!("Parsing metadata for entity: {}", entity_name);

    // Find the EntityType element for our entity
    // In EDMX, entities are defined as <EntityType Name="account">
    let entity_type = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("EntityType") &&
            node.attribute("Name").map_or(false, |name| name.eq_ignore_ascii_case(entity_name))
        })
        .ok_or_else(|| anyhow::anyhow!("Entity '{}' not found in metadata", entity_name))?;

    debug!("Found EntityType for: {}", entity_name);

    let mut fields = Vec::new();

    // Parse properties (fields) from the entity
    for property in entity_type.children().filter(|n| n.has_tag_name("Property")) {
        if let Some(field_name) = property.attribute("Name") {
            let field_type = property.attribute("Type").unwrap_or("unknown").to_string();

            // Check if field is nullable (required = !nullable)
            let nullable = property.attribute("Nullable")
                .map(|v| v == "true")
                .unwrap_or(true); // Default to nullable if not specified
            let is_required = !nullable;

            // Check if it's a custom field (typically contains underscore or starts with 'new_')
            let is_custom = field_name.contains('_') || field_name.starts_with("new_");

            fields.push(FieldInfo {
                name: field_name.to_string(),
                field_type: simplify_type(&field_type),
                is_required,
                is_custom,
            });

            debug!("Found field: {} (type: {}, required: {}, custom: {})",
                   field_name, field_type, is_required, is_custom);
        }
    }

    // Also check for NavigationProperty elements (relationships)
    for nav_prop in entity_type.children().filter(|n| n.has_tag_name("NavigationProperty")) {
        if let Some(field_name) = nav_prop.attribute("Name") {
            let field_type = nav_prop.attribute("Type").unwrap_or("unknown").to_string();
            let relationship_type = determine_relationship_type(&field_type, field_name);

            // Navigation properties are typically not required and not custom
            fields.push(FieldInfo {
                name: field_name.to_string(),
                field_type: relationship_type,
                is_required: false,
                is_custom: false,
            });

            debug!("Found navigation property: {} (type: {})", field_name, field_type);
        }
    }

    // Sort fields alphabetically
    fields.sort_by(|a, b| a.name.cmp(&b.name));

    debug!("Parsed {} fields for entity '{}'", fields.len(), entity_name);
    Ok(fields)
}

/// Simplify OData type names to more readable forms
fn simplify_type(odata_type: &str) -> String {
    match odata_type {
        "Edm.String" => "string".to_string(),
        "Edm.Int32" => "integer".to_string(),
        "Edm.Int64" => "long".to_string(),
        "Edm.Decimal" => "decimal".to_string(),
        "Edm.Double" => "double".to_string(),
        "Edm.Boolean" => "boolean".to_string(),
        "Edm.DateTime" => "datetime".to_string(),
        "Edm.DateTimeOffset" => "datetime".to_string(),
        "Edm.Guid" => "guid".to_string(),
        "Edm.Binary" => "binary".to_string(),
        // Handle collection types - these are typically 1:N relationships
        t if t.starts_with("Collection(") => {
            let inner = t.strip_prefix("Collection(").unwrap_or(t).strip_suffix(")").unwrap_or(t);
            let entity_name = extract_entity_name(inner);
            format!("1:N → {}", entity_name)
        }
        // Handle complex types (usually entity references)
        t if t.contains('.') => {
            let parts: Vec<&str> = t.split('.').collect();
            parts.last().map_or(t, |v| v).to_lowercase()
        }
        // Return as-is for unknown types
        t => t.to_string(),
    }
}

/// Determine the relationship type for navigation properties
fn determine_relationship_type(field_type: &str, field_name: &str) -> String {
    if field_type.starts_with("Collection(") {
        // This is a 1:N relationship (one entity has many related entities)
        let inner = field_type.strip_prefix("Collection(").unwrap_or(field_type).strip_suffix(")").unwrap_or(field_type);
        let entity_name = extract_entity_name(inner);
        format!("1:N → {}", entity_name)
    } else if field_type.contains('.') {
        // This is likely an N:1 relationship (many entities reference one entity)
        let entity_name = extract_entity_name(field_type);

        // Use field name patterns to provide better context
        if field_name.ends_with("id") || field_name.contains("lookup") {
            format!("N:1 → {}", entity_name)
        } else {
            format!("N:1 → {}", entity_name)
        }
    } else {
        // Fallback for unknown navigation property types
        format!("nav → {}", field_type)
    }
}

/// Extract entity name from OData type string
fn extract_entity_name(odata_type: &str) -> String {
    if odata_type.contains('.') {
        let parts: Vec<&str> = odata_type.split('.').collect();
        parts.last().map_or(odata_type, |v| v).to_string()
    } else {
        odata_type.to_string()
    }
}

/// Parse FetchXML to extract view column information
pub fn parse_view_columns(fetch_xml: &str) -> Result<Vec<ViewColumn>> {
    let doc = Document::parse(fetch_xml)
        .map_err(|e| anyhow::anyhow!("Failed to parse FetchXML: {}", e))?;

    let mut columns = Vec::new();

    // Find all attribute elements in the FetchXML
    for attribute in doc.descendants().filter(|n| n.has_tag_name("attribute")) {
        if let Some(name) = attribute.attribute("name") {
            // Check if this is a primary attribute (usually the first one or explicitly marked)
            let is_primary = attribute.attribute("primary")
                .map(|v| v == "true")
                .unwrap_or(false);

            columns.push(ViewColumn {
                name: name.to_string(),
                width: None, // FetchXML doesn't contain width info
                is_primary,
            });
        }
    }

    Ok(columns)
}

/// Detailed view structure for hierarchical comparison
#[derive(Debug, Clone)]
pub struct ViewStructure {
    pub name: String,
    pub entity_name: String,
    pub view_type: String,
    pub is_custom: bool,
    pub columns: Vec<ViewColumnDetail>,
    pub filters: Vec<ViewFilter>,
    pub sort_orders: Vec<ViewSortOrder>,
    pub fetch_xml_details: FetchXmlDetails,
}

#[derive(Debug, Clone)]
pub struct ViewColumnDetail {
    pub name: String,
    pub alias: Option<String>,
    pub width: Option<u32>,
    pub is_primary: bool,
    pub data_type: String,
    pub aggregate: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ViewFilter {
    pub attribute: String,
    pub operator: String,
    pub value: Option<String>,
    pub entity_alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ViewSortOrder {
    pub attribute: String,
    pub direction: String, // "asc" or "desc"
    pub entity_alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FetchXmlDetails {
    pub entity: String,
    pub top_count: Option<u32>,
    pub distinct: bool,
    pub no_lock: bool,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// Parse FetchXML to extract complete view structure
pub fn parse_view_structure(view_info: &ViewInfo) -> Result<ViewStructure> {
    let doc = Document::parse(&view_info.fetch_xml)
        .map_err(|e| anyhow::anyhow!("Failed to parse FetchXML: {}", e))?;

    // Parse fetch element attributes
    let fetch_node = doc.descendants()
        .find(|n| n.has_tag_name("fetch"))
        .ok_or_else(|| anyhow::anyhow!("No fetch element found"))?;

    let fetch_xml_details = FetchXmlDetails {
        entity: view_info.entity_name.clone(),
        top_count: fetch_node.attribute("top").and_then(|v| v.parse().ok()),
        distinct: fetch_node.attribute("distinct").map(|v| v == "true").unwrap_or(false),
        no_lock: fetch_node.attribute("no-lock").map(|v| v == "true").unwrap_or(false),
        page: fetch_node.attribute("page").and_then(|v| v.parse().ok()),
        page_size: fetch_node.attribute("page-size").and_then(|v| v.parse().ok()),
    };

    // Parse columns (attributes)
    let mut columns = Vec::new();
    for attribute in doc.descendants().filter(|n| n.has_tag_name("attribute")) {
        if let Some(name) = attribute.attribute("name") {
            columns.push(ViewColumnDetail {
                name: name.to_string(),
                alias: attribute.attribute("alias").map(|s| s.to_string()),
                width: None, // Not available in FetchXML
                is_primary: attribute.attribute("primary").map(|v| v == "true").unwrap_or(false),
                data_type: "unknown".to_string(), // Would need metadata lookup
                aggregate: attribute.attribute("aggregate").map(|s| s.to_string()),
            });
        }
    }

    // Parse filters (conditions)
    let mut filters = Vec::new();
    for condition in doc.descendants().filter(|n| n.has_tag_name("condition")) {
        if let Some(attribute) = condition.attribute("attribute") {
            filters.push(ViewFilter {
                attribute: attribute.to_string(),
                operator: condition.attribute("operator").unwrap_or("eq").to_string(),
                value: condition.attribute("value").map(|s| s.to_string()),
                entity_alias: condition.attribute("entityname").map(|s| s.to_string()),
            });
        }
    }

    // Parse sort orders
    let mut sort_orders = Vec::new();
    for order in doc.descendants().filter(|n| n.has_tag_name("order")) {
        if let Some(attribute) = order.attribute("attribute") {
            sort_orders.push(ViewSortOrder {
                attribute: attribute.to_string(),
                direction: order.attribute("descending")
                    .map(|v| if v == "true" { "desc" } else { "asc" })
                    .unwrap_or("asc")
                    .to_string(),
                entity_alias: order.attribute("entityname").map(|s| s.to_string()),
            });
        }
    }

    Ok(ViewStructure {
        name: view_info.name.clone(),
        entity_name: view_info.entity_name.clone(),
        view_type: view_info.view_type.clone(),
        is_custom: view_info.is_custom,
        columns,
        filters,
        sort_orders,
        fetch_xml_details,
    })
}