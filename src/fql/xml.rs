use crate::fql::ast::*;
use anyhow::Result;

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
/// use anyhow::Result;
///
/// fn example() -> Result<()> {
///     let fql = ".account | .name, .revenue | limit(10)";
///     let tokens = tokenize(fql)?;
///     let query = parse(tokens, fql)?;
///     let xml = to_fetchxml(query)?;
///     assert!(xml.contains("<entity name=\"account\">"));
///     Ok(())
/// }
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
        let mut tag_str =
            "<fetch version=\"1.0\" output-format=\"xml-platform\" mapping=\"logical\"".to_string();

        tag_str.push_str(&format!(
            " distinct=\"{}\"",
            if query.distinct { "true" } else { "false" }
        ));

        // Add aggregate attribute if query has aggregations or group by
        if !query.aggregations.is_empty() || !query.group_by.is_empty() {
            tag_str.push_str(" aggregate=\"true\"");
        }

        // Apply limit only if explicitly specified in the FQL query
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

        if query.options.formatted {
            tag_str.push_str(" formatted-value=\"true\"");
        }

        // Handle custom options from the HashMap
        for (key, value) in &query.options.custom_options {
            // Convert known option names to their FetchXML attribute equivalents
            let attr_name = match key.as_str() {
                "latematerialize" => "latematerialize",
                "aggregatelimit" => "aggregatelimit",
                "useraworderby" => "useraworderby",
                "datasource" => "datasource",
                "options" => "options",
                "outputformat" => "output-format",
                "mapping" => "mapping",
                // Pass through other options as-is (for forward compatibility)
                _ => key.as_str(),
            };

            // Handle boolean-like values
            if value == "true" || value == "false" {
                tag_str.push_str(&format!(" {}=\"{}\"", attr_name, value));
            } else {
                // Handle string/numeric values
                tag_str.push_str(&format!(" {}=\"{}\"", attr_name, self.escape_xml(value)));
            }
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

        // Generate group by attributes first
        if !query.group_by.is_empty() {
            self.generate_group_by_attributes(&query.group_by)?;
        }

        // Generate aggregation attributes
        if !query.aggregations.is_empty() {
            self.generate_aggregation_attributes(&query.aggregations, &entity.name)?;
        }

        // Generate regular attributes
        if !query.attributes.is_empty() {
            self.generate_attributes(&query.attributes)?;
        }

        // Generate filters that belong to the main entity (no entity alias or matching main entity alias)
        let main_entity_filters: Vec<&Filter> = query
            .filters
            .iter()
            .filter(|filter| {
                match filter {
                    Filter::Condition { entity_alias, .. } => {
                        entity_alias.is_none() || entity_alias.as_ref() == entity.alias.as_ref()
                    }
                    Filter::And(_) | Filter::Or(_) => true, // Complex filters stay at main level for now
                }
            })
            .collect();

        if !main_entity_filters.is_empty() {
            self.generate_filters_by_ref(&main_entity_filters)?;
        }

        // Generate having filters
        if let Some(having_filter) = &query.having {
            self.generate_having_filter(having_filter)?;
        }

        // Generate joins (link-entity elements)
        if !query.joins.is_empty() {
            self.generate_joins(&query.joins, &query.filters)?;
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

    /// Generate group by attributes
    fn generate_group_by_attributes(&mut self, group_by: &[String]) -> Result<()> {
        for attr_name in group_by {
            let attr_attrs = vec![
                ("name", attr_name.as_str()),
                ("groupby", "true"),
                ("alias", attr_name.as_str()),
            ];
            self.add_self_closing_tag("attribute", &attr_attrs);
        }
        Ok(())
    }

    /// Generate aggregation attributes
    fn generate_aggregation_attributes(
        &mut self,
        aggregations: &[Aggregation],
        entity_name: &str,
    ) -> Result<()> {
        for agg in aggregations {
            let mut attr_attrs = Vec::new();

            // For count() without an attribute, use the primary key of the entity
            let attr_name = if let Some(attribute) = &agg.attribute {
                attribute.as_str()
            } else {
                // Generate primary key name using entity name + "id" pattern
                &format!("{}id", entity_name)
            };

            attr_attrs.push(("name", attr_name));

            // Add aggregate function
            let aggregate_func = match agg.function {
                AggregationFunction::Count => "count",
                AggregationFunction::Sum => "sum",
                AggregationFunction::Average => "avg",
                AggregationFunction::Min => "min",
                AggregationFunction::Max => "max",
            };
            attr_attrs.push(("aggregate", aggregate_func));

            // Add alias - use specified alias or generate default
            let alias = if let Some(alias) = &agg.alias {
                alias.as_str()
            } else {
                // Generate default alias for aggregation functions
                aggregate_func
            };
            attr_attrs.push(("alias", alias));

            self.add_self_closing_tag("attribute", &attr_attrs);
        }
        Ok(())
    }

    /// Generate having filter wrapped in a filter element
    fn generate_having_filter(&mut self, having_filter: &Filter) -> Result<()> {
        self.add_opening_tag("filter", &[("type", "and")]);
        self.indent();
        self.generate_filter(having_filter)?;
        self.unindent();
        self.add_closing_tag("filter");
        Ok(())
    }

    /// Generate filter elements from references
    fn generate_filters_by_ref(&mut self, filters: &[&Filter]) -> Result<()> {
        if filters.len() == 1 {
            // If there's only one filter and it's already a logical grouping (And/Or),
            // generate it directly without additional wrapping
            match &filters[0] {
                Filter::And(_) | Filter::Or(_) => {
                    self.generate_filter(filters[0])?;
                }
                _ => {
                    // Single condition - wrap in filter
                    self.add_opening_tag("filter", &[("type", "and")]);
                    self.indent();
                    self.generate_filter(filters[0])?;
                    self.unindent();
                    self.add_closing_tag("filter");
                }
            }
        } else {
            // Multiple filters - wrap them all in an AND filter
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
            Filter::Condition {
                attribute,
                operator,
                value,
                entity_alias: _,
            } => {
                let op_str = self.operator_to_xml(operator, value);

                match operator {
                    FilterOperator::Null | FilterOperator::NotNull => {
                        let condition_str = format!(
                            "<condition attribute=\"{}\" operator=\"{}\" />",
                            self.escape_xml(attribute),
                            op_str
                        );
                        self.add_line(&condition_str);
                    }
                    FilterOperator::Between => {
                        // Handle between operator - different formats based on syntax used
                        match value {
                            FilterValue::Range(start, end) => {
                                // List syntax: between [val1, val2] - use separate value elements
                                self.add_opening_tag(
                                    "condition",
                                    &[("attribute", attribute.as_str()), ("operator", op_str)],
                                );
                                self.indent();

                                let start_str = self.value_to_xml_with_date_prefix(start, false)?;
                                let end_str = self.value_to_xml_with_date_prefix(end, false)?;

                                self.add_line(&format!(
                                    "<value>{}</value>",
                                    self.escape_xml(&start_str)
                                ));
                                self.add_line(&format!(
                                    "<value>{}</value>",
                                    self.escape_xml(&end_str)
                                ));

                                self.unindent();
                                self.add_closing_tag("condition");
                            }
                            FilterValue::RangeTraditional(start, end) => {
                                // Traditional syntax: between val1 and val2 - use comma-separated value
                                let start_str = self.value_to_xml_with_date_prefix(start, false)?;
                                let end_str = self.value_to_xml_with_date_prefix(end, false)?;
                                let value_str = format!("{},{}", start_str, end_str);
                                let condition_str = format!(
                                    "<condition attribute=\"{}\" operator=\"{}\" value=\"{}\" />",
                                    self.escape_xml(attribute),
                                    op_str,
                                    self.escape_xml(&value_str)
                                );
                                self.add_line(&condition_str);
                            }
                            _ => {
                                let value_str = self.value_to_xml_with_date_prefix(value, false)?;
                                let condition_str = format!(
                                    "<condition attribute=\"{}\" operator=\"{}\" value=\"{}\" />",
                                    self.escape_xml(attribute),
                                    op_str,
                                    self.escape_xml(&value_str)
                                );
                                self.add_line(&condition_str);
                            }
                        }
                    }
                    FilterOperator::Like | FilterOperator::NotLike => {
                        // For LIKE operators, automatically wrap string values with wildcards
                        let value_str = match value {
                            FilterValue::String(s) => {
                                if !s.starts_with('%') && !s.ends_with('%') {
                                    format!("%{}%", s)
                                } else {
                                    s.clone()
                                }
                            }
                            _ => self.value_to_xml(value)?,
                        };
                        let condition_str = format!(
                            "<condition attribute=\"{}\" operator=\"{}\" value=\"{}\" />",
                            self.escape_xml(attribute),
                            op_str,
                            self.escape_xml(&value_str)
                        );
                        self.add_line(&condition_str);
                    }
                    FilterOperator::In | FilterOperator::NotIn => {
                        // For IN operators, use separate value elements
                        match value {
                            FilterValue::List(values) => {
                                self.add_opening_tag(
                                    "condition",
                                    &[("attribute", attribute.as_str()), ("operator", op_str)],
                                );
                                self.indent();

                                for val in values {
                                    let val_str = self.value_to_xml(val)?;
                                    self.add_line(&format!(
                                        "<value>{}</value>",
                                        self.escape_xml(&val_str)
                                    ));
                                }

                                self.unindent();
                                self.add_closing_tag("condition");
                            }
                            _ => {
                                // Fallback to single value
                                let value_str = self.value_to_xml(value)?;
                                let condition_str = format!(
                                    "<condition attribute=\"{}\" operator=\"{}\" value=\"{}\" />",
                                    self.escape_xml(attribute),
                                    op_str,
                                    self.escape_xml(&value_str)
                                );
                                self.add_line(&condition_str);
                            }
                        }
                    }
                    _ => {
                        let value_str = self.value_to_xml(value)?;
                        let condition_str = format!(
                            "<condition attribute=\"{}\" operator=\"{}\" value=\"{}\" />",
                            self.escape_xml(attribute),
                            op_str,
                            self.escape_xml(&value_str)
                        );
                        self.add_line(&condition_str);
                    }
                }
            }
            Filter::And(filters) => {
                self.add_opening_tag("filter", &[("type", "and")]);
                self.indent();
                for filter in filters {
                    self.generate_filter(filter)?;
                }
                self.unindent();
                self.add_closing_tag("filter");
            }
            Filter::Or(filters) => {
                self.add_opening_tag("filter", &[("type", "or")]);
                self.indent();
                for filter in filters {
                    self.generate_filter(filter)?;
                }
                self.unindent();
                self.add_closing_tag("filter");
            }
        }
        Ok(())
    }

    /// Generate link-entity elements for joins
    fn generate_joins(&mut self, joins: &[Join], query_filters: &[Filter]) -> Result<()> {
        for join in joins {
            self.generate_join(join, query_filters)?;
        }
        Ok(())
    }

    /// Generate a single join
    fn generate_join(&mut self, join: &Join, query_filters: &[Filter]) -> Result<()> {
        let mut link_attrs = vec![("name", join.entity.name.as_str())];

        if let Some(alias) = &join.entity.alias {
            link_attrs.push(("alias", alias.as_str()));
        }

        link_attrs.push(("from", join.on_condition.from_attribute.as_str()));
        link_attrs.push(("to", join.on_condition.to_attribute.as_str()));

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

        // Collect filters that belong to this join (by entity alias)
        let mut join_filters = join.filters.iter().collect::<Vec<_>>();

        // Add entity-qualified filters from the main query that belong to this join
        if let Some(join_alias) = &join.entity.alias {
            for filter in query_filters {
                if let Filter::Condition { entity_alias, .. } = filter
                    && entity_alias.as_ref() == Some(join_alias)
                {
                    join_filters.push(filter);
                }
            }
        }

        // Generate filters for joined entity
        if !join_filters.is_empty() {
            self.generate_filters_by_ref(&join_filters)?;
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
                }
                OrderDirection::Ascending => {
                    order_attrs.push(("descending", "false"));
                }
            }

            self.add_self_closing_tag("order", &order_attrs);
        }
        Ok(())
    }

    /// Convert filter operator to FetchXML operator string
    fn operator_to_xml(&self, operator: &FilterOperator, value: &FilterValue) -> &'static str {
        // Use special date operators for date values
        if matches!(value, FilterValue::Date(_)) {
            match operator {
                FilterOperator::Equal => "on",
                FilterOperator::NotEqual => "not-on",
                FilterOperator::GreaterThan => "on-or-after",
                FilterOperator::GreaterThanOrEqual => "on-or-after",
                FilterOperator::LessThan => "on-or-before",
                FilterOperator::LessThanOrEqual => "on-or-before",
                _ => {
                    // For other operators with dates, use standard mapping
                    match operator {
                        FilterOperator::Like => "like",
                        FilterOperator::NotLike => "not-like",
                        FilterOperator::BeginsWith => "begins-with",
                        FilterOperator::EndsWith => "ends-with",
                        FilterOperator::In => "in",
                        FilterOperator::NotIn => "not-in",
                        FilterOperator::Between => "between",
                        FilterOperator::Null => "null",
                        FilterOperator::NotNull => "not-null",
                        _ => "eq", // fallback
                    }
                }
            }
        } else {
            // Standard operators for non-date values
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
    }

    /// Convert filter value to XML value string
    fn value_to_xml(&self, value: &FilterValue) -> Result<String> {
        self.value_to_xml_with_date_prefix(value, true)
    }

    /// Convert filter value to XML value string with option to control date prefix
    fn value_to_xml_with_date_prefix(
        &self,
        value: &FilterValue,
        add_date_prefix: bool,
    ) -> Result<String> {
        match value {
            FilterValue::String(s) => Ok(self.escape_xml(s)),
            FilterValue::Number(n) => Ok(n.to_string()),
            FilterValue::Integer(i) => Ok(i.to_string()),
            FilterValue::Boolean(b) => Ok(if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }),
            FilterValue::Date(d) => Ok(if add_date_prefix {
                format!("@{}", d)
            } else {
                d.clone()
            }),
            FilterValue::Null => Ok(String::new()),
            FilterValue::List(values) => {
                let str_values: Result<Vec<String>> = values
                    .iter()
                    .map(|v| self.value_to_xml_with_date_prefix(v, add_date_prefix))
                    .collect();
                Ok(str_values?.join(","))
            }
            FilterValue::Range(start, end) => {
                let start_str = self.value_to_xml_with_date_prefix(start, add_date_prefix)?;
                let end_str = self.value_to_xml_with_date_prefix(end, add_date_prefix)?;
                Ok(format!("{},{}", start_str, end_str))
            }
            FilterValue::RangeTraditional(start, end) => {
                let start_str = self.value_to_xml_with_date_prefix(start, add_date_prefix)?;
                let end_str = self.value_to_xml_with_date_prefix(end, add_date_prefix)?;
                Ok(format!("{},{}", start_str, end_str))
            }
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
