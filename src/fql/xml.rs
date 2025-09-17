use anyhow::Result;
use crate::fql::ast::*;

/// Converts an FQL AST into FetchXML string
///
/// # Arguments
/// * `query` - The parsed FQL query AST
///
/// # Returns
/// * `Ok(String)` - FetchXML string on success
/// * `Err(anyhow::Error)` - XML generation error
///
/// # Examples
/// ```rust
/// use dynamics_cli::fql::{tokenize, parse, to_fetchxml};
///
/// let tokens = tokenize(".account | .name, .revenue | limit(10)")?;
/// let query = parse(tokens)?;
/// let xml = to_fetchxml(query)?;
/// assert!(xml.contains("<entity name=\"account\">"));
/// ```
pub fn to_fetchxml(query: Query) -> Result<String> {
    let mut generator = XmlGenerator::new();
    generator.generate(query)
}

/// XML generation helper struct
#[derive(Debug)]
struct XmlGenerator {
    indent_level: usize,
    buffer: String,
}

impl XmlGenerator {
    fn new() -> Self {
        Self {
            indent_level: 0,
            buffer: String::new(),
        }
    }

    /// Generate complete FetchXML document
    fn generate(&mut self, query: Query) -> Result<String> {
        self.generate_fetch_element(&query)?;
        self.indent();
        self.generate_entity(&query.entity, &query)?;
        self.unindent();
        self.add_line("</fetch>");
        Ok(self.buffer.clone())
    }

    /// Generate fetch element with attributes
    fn generate_fetch_element(&mut self, query: &Query) -> Result<()> {
        let mut tag_str = "<fetch version=\"1.0\" output-format=\"xml-platform\" mapping=\"logical\"".to_string();

        tag_str.push_str(&format!(" distinct=\"{}\"", if query.distinct { "true" } else { "false" }));

        if let Some(limit) = query.limit {
            tag_str.push_str(&format!(" top=\"{}\"", limit));
        }

        if let Some((page_num, page_size)) = query.page {
            tag_str.push_str(&format!(" page=\"{}\" count=\"{}\"", page_num, page_size));
        }

        if query.options.return_total_record_count {
            tag_str.push_str(" returntotalrecordcount=\"true\"");
        }

        if query.options.no_lock {
            tag_str.push_str(" no-lock=\"true\"");
        }

        tag_str.push('>');
        self.add_line(&tag_str);
        Ok(())
    }

    /// Generate entity element
    fn generate_entity(&mut self, entity: &Entity, query: &Query) -> Result<()> {
        let mut entity_attrs = vec![("name", entity.name.as_str())];

        if let Some(alias) = &entity.alias {
            entity_attrs.push(("alias", alias.as_str()));
        }

        self.add_opening_tag("entity", &entity_attrs);
        self.indent();

        // Generate attributes
        if !query.attributes.is_empty() {
            self.generate_attributes(&query.attributes)?;
        }

        // Generate filters
        if !query.filters.is_empty() {
            self.generate_filters(&query.filters)?;
        }

        // Generate joins (link-entity elements)
        if !query.joins.is_empty() {
            self.generate_joins(&query.joins)?;
        }

        // Generate order
        if !query.order.is_empty() {
            self.generate_order(&query.order)?;
        }

        self.unindent();
        self.add_closing_tag("entity");
        Ok(())
    }

    /// Generate attribute elements
    fn generate_attributes(&mut self, attributes: &[Attribute]) -> Result<()> {
        for attr in attributes {
            if attr.name == "*" {
                self.add_self_closing_tag("all-attributes", &[]);
            } else {
                let mut attr_attrs = vec![("name", attr.name.as_str())];

                if let Some(alias) = &attr.alias {
                    attr_attrs.push(("alias", alias.as_str()));
                }

                self.add_self_closing_tag("attribute", &attr_attrs);
            }
        }
        Ok(())
    }

    /// Generate filter elements
    fn generate_filters(&mut self, filters: &[Filter]) -> Result<()> {
        if filters.len() == 1 {
            self.generate_filter(&filters[0])?;
        } else {
            // Multiple filters - wrap in AND
            self.add_opening_tag("filter", &[("type", "and")]);
            self.indent();
            for filter in filters {
                self.generate_filter(filter)?;
            }
            self.unindent();
            self.add_closing_tag("filter");
        }
        Ok(())
    }

    /// Generate a single filter condition
    fn generate_filter(&mut self, filter: &Filter) -> Result<()> {
        match filter {
            Filter::Condition { attribute, operator, value, entity_alias: _ } => {
                let op_str = self.operator_to_xml(operator);
                let value_str = self.value_to_xml(value)?;

                let condition_str = match operator {
                    FilterOperator::Null | FilterOperator::NotNull => {
                        format!("<condition attribute=\"{}\" operator=\"{}\" />",
                               self.escape_xml(attribute), op_str)
                    }
                    _ => {
                        format!("<condition attribute=\"{}\" operator=\"{}\" value=\"{}\" />",
                               self.escape_xml(attribute), op_str, self.escape_xml(&value_str))
                    }
                };

                self.add_line(&condition_str);
            },
            Filter::And(filters) => {
                self.add_opening_tag("filter", &[("type", "and")]);
                self.indent();
                for filter in filters {
                    self.generate_filter(filter)?;
                }
                self.unindent();
                self.add_closing_tag("filter");
            },
            Filter::Or(filters) => {
                self.add_opening_tag("filter", &[("type", "or")]);
                self.indent();
                for filter in filters {
                    self.generate_filter(filter)?;
                }
                self.unindent();
                self.add_closing_tag("filter");
            },
        }
        Ok(())
    }

    /// Generate link-entity elements for joins
    fn generate_joins(&mut self, joins: &[Join]) -> Result<()> {
        for join in joins {
            self.generate_join(join)?;
        }
        Ok(())
    }

    /// Generate a single join
    fn generate_join(&mut self, join: &Join) -> Result<()> {
        let mut link_attrs = vec![
            ("name", join.entity.name.as_str()),
            ("from", join.on_condition.from_attribute.as_str()),
            ("to", join.on_condition.to_attribute.as_str()),
        ];

        if let Some(alias) = &join.entity.alias {
            link_attrs.push(("alias", alias.as_str()));
        }

        match join.join_type {
            JoinType::Inner => link_attrs.push(("link-type", "inner")),
            JoinType::Left => link_attrs.push(("link-type", "outer")),
        }

        self.add_opening_tag("link-entity", &link_attrs);
        self.indent();

        // Generate attributes for joined entity
        if !join.attributes.is_empty() {
            self.generate_attributes(&join.attributes)?;
        }

        // Generate filters for joined entity
        if !join.filters.is_empty() {
            self.generate_filters(&join.filters)?;
        }

        self.unindent();
        self.add_closing_tag("link-entity");
        Ok(())
    }

    /// Generate order elements
    fn generate_order(&mut self, order: &[OrderBy]) -> Result<()> {
        for order_item in order {
            let mut order_attrs = vec![("attribute", order_item.attribute.as_str())];

            match order_item.direction {
                OrderDirection::Descending => {
                    order_attrs.push(("descending", "true"));
                },
                OrderDirection::Ascending => {
                    // Ascending is default, no need to specify
                },
            }

            self.add_self_closing_tag("order", &order_attrs);
        }
        Ok(())
    }

    /// Convert filter operator to FetchXML operator string
    fn operator_to_xml(&self, operator: &FilterOperator) -> &'static str {
        match operator {
            FilterOperator::Equal => "eq",
            FilterOperator::NotEqual => "ne",
            FilterOperator::GreaterThan => "gt",
            FilterOperator::GreaterThanOrEqual => "ge",
            FilterOperator::LessThan => "lt",
            FilterOperator::LessThanOrEqual => "le",
            FilterOperator::Like => "like",
            FilterOperator::NotLike => "not-like",
            FilterOperator::BeginsWith => "begins-with",
            FilterOperator::EndsWith => "ends-with",
            FilterOperator::In => "in",
            FilterOperator::NotIn => "not-in",
            FilterOperator::Between => "between",
            FilterOperator::Null => "null",
            FilterOperator::NotNull => "not-null",
        }
    }

    /// Convert filter value to XML value string
    fn value_to_xml(&self, value: &FilterValue) -> Result<String> {
        match value {
            FilterValue::String(s) => Ok(self.escape_xml(s)),
            FilterValue::Number(n) => Ok(n.to_string()),
            FilterValue::Integer(i) => Ok(i.to_string()),
            FilterValue::Boolean(b) => Ok(if *b { "true".to_string() } else { "false".to_string() }),
            FilterValue::Date(d) => Ok(d.clone()),
            FilterValue::Null => Ok(String::new()),
            FilterValue::List(values) => {
                let str_values: Result<Vec<String>> = values.iter()
                    .map(|v| self.value_to_xml(v))
                    .collect();
                Ok(str_values?.join(","))
            },
            FilterValue::Range(start, end) => {
                let start_str = self.value_to_xml(start)?;
                let end_str = self.value_to_xml(end)?;
                Ok(format!("{},{}", start_str, end_str))
            },
        }
    }

    /// Add indented line to buffer
    fn add_line(&mut self, content: &str) {
        self.buffer.push_str(&self.get_indent());
        self.buffer.push_str(content);
        self.buffer.push('\n');
    }

    /// Add opening tag
    fn add_opening_tag(&mut self, tag: &str, attributes: &[(&str, &str)]) {
        let mut tag_str = format!("<{}", tag);

        for (name, value) in attributes {
            tag_str.push_str(&format!(" {}=\"{}\"", name, self.escape_xml(value)));
        }

        tag_str.push('>');
        self.add_line(&tag_str);
    }

    /// Add closing tag
    fn add_closing_tag(&mut self, tag: &str) {
        self.add_line(&format!("</{}>", tag));
    }

    /// Add self-closing tag
    fn add_self_closing_tag(&mut self, tag: &str, attributes: &[(&str, &str)]) {
        let mut tag_str = format!("<{}", tag);

        for (name, value) in attributes {
            tag_str.push_str(&format!(" {}=\"{}\"", name, self.escape_xml(value)));
        }

        tag_str.push_str(" />");
        self.add_line(&tag_str);
    }

    /// Increase indentation level
    fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation level
    fn unindent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Get current indentation string
    fn get_indent(&self) -> String {
        "  ".repeat(self.indent_level)
    }

    /// Escape XML special characters
    fn escape_xml(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}