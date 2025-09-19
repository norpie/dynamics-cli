use crate::fql::ast::*;
use crate::fql::lexer::{LocatedToken, ParseError, Token};
use anyhow::Result;

/// Parses a vector of located tokens into an FQL AST with position-aware error messages
///
/// # Arguments
/// * `tokens` - Vector of located tokens from lexer
/// * `input` - Original input string for error formatting
///
/// # Returns
/// * `Ok(Query)` - Parsed query AST on success
/// * `Err(anyhow::Error)` - Parse error with position information
pub fn parse(tokens: Vec<LocatedToken>, input: &str) -> Result<Query> {
    // Extract just the tokens for the regular parser
    let plain_tokens: Vec<Token> = tokens.iter().map(|lt| lt.token.clone()).collect();

    // Try parsing with the regular parser
    let mut parser = Parser::new(plain_tokens);
    match parser.parse_query() {
        Ok(query) => Ok(query),
        Err(_) => {
            // If parsing failed, try to give better error messages
            if tokens.is_empty() {
                return Err(anyhow::anyhow!("Empty input"));
            }

            // Look for common issues and provide position-aware errors
            let mut error_message = "Parse error".to_string();
            let mut error_position = tokens[0].position.clone();

            // Check for incomplete expressions (e.g., ".account | .name ==")
            if let Some(last_token) = tokens.last() {
                if matches!(
                    last_token.token,
                    Token::Equal | Token::NotEqual | Token::GreaterThan | Token::LessThan
                ) {
                    error_message =
                        "Incomplete expression: expected value after operator".to_string();
                    error_position = last_token.position.clone();
                } else if matches!(last_token.token, Token::And | Token::Or) {
                    error_message =
                        "Incomplete expression: expected condition after logical operator"
                            .to_string();
                    error_position = last_token.position.clone();
                }
            }

            let parse_error = ParseError {
                message: error_message,
                position: error_position,
                input: input.to_string(),
            };

            Err(anyhow::anyhow!("{}", parse_error))
        }
    }
}

/// Different types of query sections that can appear after pipes
#[derive(Debug, Clone, PartialEq)]
enum SectionType {
    Attributes,
    Filters,
    Aggregations,
    GroupBy,
    Having,
    OrderBy,
    Joins,
    Limit,
    Page,
    Distinct,
    Options,
}

/// Parser state for tracking current position and context
#[derive(Debug)]
struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Parse the main query structure
    fn parse_query(&mut self) -> Result<Query> {
        let mut query = Query {
            entity: Entity {
                name: String::new(),
                alias: None,
            },
            attributes: Vec::new(),
            filters: Vec::new(),
            joins: Vec::new(),
            order: Vec::new(),
            aggregations: Vec::new(),
            group_by: Vec::new(),
            having: None,
            limit: None,
            page: None,
            distinct: false,
            options: QueryOptions::default(),
        };

        // Parse entity selection (required)
        query.entity = self.parse_entity()?;

        // Parse the rest of the query in pipe-separated sections
        while !self.is_at_end() && self.peek() == Some(&Token::Pipe) {
            self.advance(); // consume '|'

            // Skip whitespace tokens if any
            while self.peek() == Some(&Token::Whitespace) || self.peek() == Some(&Token::Newline) {
                self.advance();
            }

            if self.is_at_end() {
                break;
            }

            // Determine section type by lookahead
            let section_type = self.determine_section_type()?;

            match section_type {
                SectionType::Attributes => {
                    let attrs = self.parse_attributes()?;
                    query.attributes.extend(attrs);
                }
                SectionType::Filters => {
                    let filter = self.parse_filter()?;
                    query.filters.push(filter);
                }
                SectionType::Aggregations => {
                    let aggs = self.parse_aggregations()?;
                    query.aggregations.extend(aggs);
                }
                SectionType::GroupBy => {
                    query.group_by = self.parse_group_by()?;
                }
                SectionType::Having => {
                    query.having = self.parse_having()?;
                }
                SectionType::OrderBy => {
                    query.order = self.parse_order_by()?;
                }
                SectionType::Joins => {
                    let joins = self.parse_joins()?;
                    query.joins.extend(joins);
                }
                SectionType::Limit => {
                    query.limit = self.parse_limit()?;
                }
                SectionType::Page => {
                    query.page = self.parse_page()?;
                }
                SectionType::Distinct => {
                    self.advance();
                    query.distinct = true;
                }
                SectionType::Options => {
                    query.options = self.parse_options()?;
                }
            }
        }

        Ok(query)
    }

    /// Determine what type of section follows based on lookahead
    fn determine_section_type(&self) -> Result<SectionType> {
        match self.peek() {
            // Keywords that clearly identify section types
            Some(Token::Group) => Ok(SectionType::GroupBy),
            Some(Token::Having) => Ok(SectionType::Having),
            Some(Token::Order) => Ok(SectionType::OrderBy),
            Some(Token::Join) | Some(Token::LeftJoin) => Ok(SectionType::Joins),
            Some(Token::Limit) => Ok(SectionType::Limit),
            Some(Token::Page) => Ok(SectionType::Page),
            Some(Token::Distinct) => Ok(SectionType::Distinct),
            Some(Token::Options) => Ok(SectionType::Options),

            // Aggregation functions
            Some(Token::Count) | Some(Token::Sum) | Some(Token::Avg) | Some(Token::Min)
            | Some(Token::Max) => Ok(SectionType::Aggregations),

            // Parentheses indicate grouped filter expressions
            Some(Token::LeftParen) => Ok(SectionType::Filters),

            // For dots and identifiers, we need to look ahead to disambiguate
            Some(Token::Dot) => {
                // Look ahead to see if this is a filter (.attr operator value) or attribute (.attr, .attr2)
                self.lookahead_for_filter_or_attribute()
            }

            Some(Token::Wildcard) => {
                // .* is always attributes
                Ok(SectionType::Attributes)
            }

            Some(Token::Identifier(_)) => {
                // Could be a filter condition like "attr > value" or aggregation alias reference
                // Look ahead for operators to determine if it's a filter
                if self.has_operator_ahead() {
                    Ok(SectionType::Filters)
                } else {
                    // No operator found - could be entity-qualified attributes like "c.fullname, c.lastname"
                    Ok(SectionType::Attributes)
                }
            }

            _ => Err(anyhow::anyhow!(
                "Unexpected token in query section: {:?}",
                self.peek()
            )),
        }
    }

    /// Look ahead to determine if a dot starts a filter or attribute
    fn lookahead_for_filter_or_attribute(&self) -> Result<SectionType> {
        let mut pos = self.current + 1; // Skip the dot

        // Skip the attribute name
        if pos < self.tokens.len() && matches!(self.tokens[pos], Token::Identifier(_)) {
            pos += 1;
        } else {
            return Ok(SectionType::Attributes); // Invalid, but default to attributes
        }

        // Check what comes after the attribute name
        if pos < self.tokens.len() {
            match &self.tokens[pos] {
                // These indicate a filter condition
                Token::Equal
                | Token::NotEqual
                | Token::GreaterThan
                | Token::GreaterEqual
                | Token::LessThan
                | Token::LessEqual
                | Token::Like
                | Token::NotLike
                | Token::BeginsWith
                | Token::EndsWith
                | Token::In
                | Token::NotIn
                | Token::Between => Ok(SectionType::Filters),
                // These indicate attribute selection
                Token::Comma | Token::Pipe | Token::As | Token::Eof => Ok(SectionType::Attributes),
                // Default to attributes if ambiguous
                _ => Ok(SectionType::Attributes),
            }
        } else {
            Ok(SectionType::Attributes)
        }
    }

    /// Check if there's an operator ahead in the current position
    fn has_operator_ahead(&self) -> bool {
        let mut pos = self.current;

        // Skip identifier
        if pos < self.tokens.len() && matches!(self.tokens[pos], Token::Identifier(_)) {
            pos += 1;
        }

        // Handle entity-qualified identifiers like "a.activityid"
        if pos < self.tokens.len() && matches!(self.tokens[pos], Token::Dot) {
            pos += 1; // Skip dot
            // Skip second identifier
            if pos < self.tokens.len() && matches!(self.tokens[pos], Token::Identifier(_)) {
                pos += 1;
            }
        }

        // Check for operator
        if pos < self.tokens.len() {
            matches!(
                self.tokens[pos],
                Token::Equal
                    | Token::NotEqual
                    | Token::GreaterThan
                    | Token::GreaterEqual
                    | Token::LessThan
                    | Token::LessEqual
                    | Token::Like
                    | Token::NotLike
                    | Token::BeginsWith
                    | Token::EndsWith
                    | Token::In
                    | Token::NotIn
                    | Token::Between
            )
        } else {
            false
        }
    }

    /// Parse entity selection (.account, .contact, etc.)
    fn parse_entity(&mut self) -> Result<Entity> {
        self.expect(Token::Dot)?;

        let name = match self.advance() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => return Err(anyhow::anyhow!("Expected entity name after '.'")),
        };

        let mut alias = None;
        if self.peek() == Some(&Token::As) {
            self.advance(); // consume 'as'
            if let Some(Token::Identifier(alias_name)) = self.advance() {
                alias = Some(alias_name.clone());
            } else {
                return Err(anyhow::anyhow!("Expected alias name after 'as'"));
            }
        }

        Ok(Entity { name, alias })
    }

    /// Parse attribute list (.name, .revenue, etc.)
    fn parse_attributes(&mut self) -> Result<Vec<Attribute>> {
        let mut attributes = Vec::new();

        // Check for .* pattern (dot followed by wildcard)
        if self.peek() == Some(&Token::Dot) {
            let lookahead = self.current + 1;
            if lookahead < self.tokens.len() && self.tokens[lookahead] == Token::Wildcard {
                self.advance(); // consume '.'
                self.advance(); // consume '*'
                return Ok(vec![Attribute {
                    name: "*".to_string(),
                    alias: None,
                    entity_alias: None,
                }]);
            }
        }

        // Parse first attribute
        attributes.push(self.parse_attribute()?);

        // Parse additional attributes separated by commas
        while self.peek() == Some(&Token::Comma) {
            self.advance(); // consume ','
            attributes.push(self.parse_attribute()?);
        }

        Ok(attributes)
    }

    /// Parse a single attribute with optional alias
    fn parse_attribute(&mut self) -> Result<Attribute> {
        // Handle entity alias prefix (e.g., a.name)
        let mut entity_alias = None;
        let name;

        if let Some(Token::Identifier(first_part)) = self.peek() {
            let first_part = first_part.clone();
            self.advance();

            if self.peek() == Some(&Token::Dot) {
                // This is an entity alias
                entity_alias = Some(first_part);
                self.advance(); // consume '.'

                if let Some(Token::Identifier(attr_name)) = self.advance() {
                    name = attr_name.clone();
                } else {
                    return Err(anyhow::anyhow!(
                        "Expected attribute name after entity alias"
                    ));
                }
            } else {
                // This was just the attribute name
                name = first_part;
            }
        } else if self.peek() == Some(&Token::Dot) {
            self.advance(); // consume '.'
            if let Some(Token::Identifier(attr_name)) = self.advance() {
                name = attr_name.clone();
            } else {
                return Err(anyhow::anyhow!("Expected attribute name after '.'"));
            }
        } else {
            return Err(anyhow::anyhow!("Expected attribute specification"));
        }

        // Check for alias
        let mut alias = None;
        if self.peek() == Some(&Token::As) {
            self.advance(); // consume 'as'
            if let Some(Token::Identifier(alias_name)) = self.advance() {
                alias = Some(alias_name.clone());
            } else {
                return Err(anyhow::anyhow!("Expected alias name after 'as'"));
            }
        }

        Ok(Attribute {
            name,
            alias,
            entity_alias,
        })
    }

    /// Parse a single filter condition
    fn parse_filter(&mut self) -> Result<Filter> {
        self.parse_filter_expression()
    }

    /// Parse filter expressions with AND/OR
    fn parse_filter_expression(&mut self) -> Result<Filter> {
        let mut left = self.parse_filter_term()?;

        while let Some(token) = self.peek() {
            match token {
                Token::And => {
                    self.advance();
                    let right = self.parse_filter_term()?;
                    left = Filter::And(vec![left, right]);
                }
                Token::Or => {
                    self.advance();
                    let right = self.parse_filter_term()?;
                    left = Filter::Or(vec![left, right]);
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// Parse a single filter term (condition or parenthesized expression)
    fn parse_filter_term(&mut self) -> Result<Filter> {
        if self.peek() == Some(&Token::LeftParen) {
            self.advance(); // consume '('
            let expr = self.parse_filter_expression()?;
            self.expect(Token::RightParen)?;
            Ok(expr)
        } else {
            self.parse_filter_condition()
        }
    }

    /// Parse a basic filter condition (attribute operator value)
    fn parse_filter_condition(&mut self) -> Result<Filter> {
        // Parse attribute reference
        let mut entity_alias = None;
        let attribute = if self.peek() == Some(&Token::Dot) {
            self.advance(); // consume '.'
            if let Some(Token::Identifier(name)) = self.advance() {
                name.clone()
            } else {
                return Err(anyhow::anyhow!("Expected attribute name"));
            }
        } else if let Some(Token::Identifier(first_part)) = self.peek() {
            let first_part = first_part.clone();
            self.advance();

            if self.peek() == Some(&Token::Dot) {
                entity_alias = Some(first_part);
                self.advance(); // consume '.'
                if let Some(Token::Identifier(attr_name)) = self.advance() {
                    attr_name.clone()
                } else {
                    return Err(anyhow::anyhow!(
                        "Expected attribute name after entity alias"
                    ));
                }
            } else {
                first_part
            }
        } else {
            return Err(anyhow::anyhow!("Expected attribute in filter condition"));
        };

        // Parse operator
        let operator = self.parse_filter_operator()?;

        // Parse value and handle special cases
        let (operator, value) = if self.peek() == Some(&Token::Null) {
            self.advance(); // consume 'null'
            match operator {
                FilterOperator::Equal => (FilterOperator::Null, FilterValue::Null),
                FilterOperator::NotEqual => (FilterOperator::NotNull, FilterValue::Null),
                _ => return Err(anyhow::anyhow!("Invalid operator with null value")),
            }
        } else if operator == FilterOperator::Between {
            // Parse "between value1 and value2" or "between [value1, value2]"
            if self.peek() == Some(&Token::LeftBracket) {
                // List syntax: between [value1, value2] - use separate value elements in XML
                let list_value = self.parse_filter_value()?;
                if let FilterValue::List(values) = list_value {
                    if values.len() == 2 {
                        // Use a special marker to indicate this should use separate value elements
                        (
                            operator,
                            FilterValue::Range(
                                Box::new(values[0].clone()),
                                Box::new(values[1].clone()),
                            ),
                        )
                    } else {
                        return Err(anyhow::anyhow!(
                            "Between operator with list syntax requires exactly 2 values"
                        ));
                    }
                } else {
                    return Err(anyhow::anyhow!("Expected list for between operator"));
                }
            } else {
                // Traditional syntax: between value1 and value2 - use comma-separated in single attribute
                let start_value = self.parse_filter_value()?;
                self.expect(Token::And)?;
                let end_value = self.parse_filter_value()?;
                // Use a special marker to distinguish traditional vs list syntax
                (
                    operator,
                    FilterValue::RangeTraditional(Box::new(start_value), Box::new(end_value)),
                )
            }
        } else {
            (operator, self.parse_filter_value()?)
        };

        Ok(Filter::Condition {
            attribute,
            operator,
            value,
            entity_alias,
        })
    }

    /// Parse join clauses
    fn parse_joins(&mut self) -> Result<Vec<Join>> {
        let mut joins = Vec::new();

        while let Some(token) = self.peek() {
            match token {
                Token::Join | Token::LeftJoin => {
                    joins.push(self.parse_join()?);
                }
                _ => break,
            }
        }

        Ok(joins)
    }

    /// Parse a single join
    fn parse_join(&mut self) -> Result<Join> {
        // Parse join type
        let join_type = match self.advance() {
            Some(Token::Join) => JoinType::Inner,
            Some(Token::LeftJoin) => JoinType::Left,
            _ => return Err(anyhow::anyhow!("Expected 'join' or 'leftjoin'")),
        };

        // Parse opening parenthesis
        self.expect(Token::LeftParen)?;

        // Parse joined entity
        let entity = self.parse_entity()?;

        // Parse 'on' keyword
        self.expect(Token::On)?;

        // Parse join condition with entity context
        let on_condition = self.parse_join_condition_with_entity(&entity.name)?;

        // Parse optional attributes and filters within the join
        let mut attributes = Vec::new();
        let mut filters = Vec::new();

        while !self.is_at_end() && self.peek() != Some(&Token::RightParen) {
            if self.peek() == Some(&Token::Pipe) {
                self.advance(); // consume '|'

                match self.peek() {
                    Some(Token::Dot) => {
                        // Check if this is an attribute selection or a filter condition
                        let mut lookahead = self.current + 1;
                        let mut is_filter = false;

                        // Look ahead to see if this is a filter (has an operator after the attribute)
                        while lookahead < self.tokens.len() {
                            match &self.tokens[lookahead] {
                                Token::Equal
                                | Token::NotEqual
                                | Token::GreaterThan
                                | Token::GreaterEqual
                                | Token::LessThan
                                | Token::LessEqual
                                | Token::Like
                                | Token::NotLike
                                | Token::BeginsWith
                                | Token::EndsWith
                                | Token::In
                                | Token::NotIn
                                | Token::Between => {
                                    is_filter = true;
                                    break;
                                }
                                Token::Comma | Token::RightParen => break,
                                _ => lookahead += 1,
                            }
                        }

                        if is_filter {
                            let filter = self.parse_filter()?;
                            filters.push(filter);
                        } else {
                            let attrs = self.parse_attributes()?;
                            attributes.extend(attrs);
                        }
                    }
                    Some(Token::Wildcard) => {
                        let attrs = self.parse_attributes()?;
                        attributes.extend(attrs);
                    }
                    _ => {
                        let filter = self.parse_filter()?;
                        filters.push(filter);
                    }
                }
            } else {
                break;
            }
        }

        // Parse closing parenthesis
        self.expect(Token::RightParen)?;

        Ok(Join {
            entity,
            join_type,
            on_condition,
            filters,
            attributes,
        })
    }

    /// Parse join condition with explicit syntax: source_entity.source_field -> target_entity.target_field
    fn parse_join_condition_with_entity(&mut self, _entity_name: &str) -> Result<JoinCondition> {
        // Parse source entity alias and attribute (e.g., "c.contactid")
        let from_entity_alias = if let Some(Token::Identifier(alias)) = self.peek() {
            let alias = alias.clone();
            self.advance();

            self.expect(Token::Dot)?;
            Some(alias)
        } else {
            return Err(anyhow::anyhow!(
                "Expected source entity alias in join condition"
            ));
        };

        let from_attribute = match self.advance() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Expected source attribute name in join condition"
                ));
            }
        };

        // Expect arrow (->)
        self.expect(Token::Arrow)?;

        // Parse target entity alias and attribute (e.g., "account.primarycontactid")
        let to_entity_alias = if let Some(Token::Identifier(alias)) = self.peek() {
            let alias = alias.clone();
            self.advance();

            self.expect(Token::Dot)?;
            Some(alias)
        } else {
            return Err(anyhow::anyhow!(
                "Expected target entity alias in join condition"
            ));
        };

        let to_attribute = match self.advance() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Expected target attribute name in join condition"
                ));
            }
        };

        Ok(JoinCondition {
            from_attribute,
            to_attribute,
            from_entity_alias,
            to_entity_alias,
        })
    }

    /// Parse aggregation functions
    fn parse_aggregations(&mut self) -> Result<Vec<Aggregation>> {
        let mut aggregations = Vec::new();

        // Parse first aggregation
        aggregations.push(self.parse_single_aggregation()?);

        // Parse additional aggregations separated by commas
        while self.peek() == Some(&Token::Comma) {
            self.advance(); // consume ','
            aggregations.push(self.parse_single_aggregation()?);
        }

        Ok(aggregations)
    }

    /// Parse a single aggregation function
    fn parse_single_aggregation(&mut self) -> Result<Aggregation> {
        let function = match self.advance() {
            Some(Token::Count) => AggregationFunction::Count,
            Some(Token::Sum) => AggregationFunction::Sum,
            Some(Token::Avg) => AggregationFunction::Average,
            Some(Token::Min) => AggregationFunction::Min,
            Some(Token::Max) => AggregationFunction::Max,
            _ => return Err(anyhow::anyhow!("Expected aggregation function")),
        };

        self.expect(Token::LeftParen)?;

        let mut attribute = None;
        let mut entity_alias = None;

        // Parse optional attribute for aggregation
        if self.peek() != Some(&Token::RightParen) {
            if self.peek() == Some(&Token::Dot) {
                self.advance(); // consume '.'
                if let Some(Token::Identifier(attr_name)) = self.advance() {
                    attribute = Some(attr_name.clone());
                }
            } else if let Some(Token::Identifier(first_part)) = self.peek() {
                let first_part = first_part.clone();
                self.advance();

                if self.peek() == Some(&Token::Dot) {
                    entity_alias = Some(first_part);
                    self.advance(); // consume '.'
                    if let Some(Token::Identifier(attr_name)) = self.advance() {
                        attribute = Some(attr_name.clone());
                    }
                } else {
                    attribute = Some(first_part);
                }
            }
        }

        self.expect(Token::RightParen)?;

        // Parse optional alias
        let mut alias = None;
        if self.peek() == Some(&Token::As) {
            self.advance(); // consume 'as'
            if let Some(Token::Identifier(alias_name)) = self.advance() {
                alias = Some(alias_name.clone());
            }
        }

        Ok(Aggregation {
            function,
            attribute,
            alias,
            entity_alias,
        })
    }

    /// Parse group by clause
    fn parse_group_by(&mut self) -> Result<Vec<String>> {
        self.expect(Token::Group)?;
        self.expect(Token::LeftParen)?;

        let mut attributes = Vec::new();

        // Parse first attribute
        self.expect(Token::Dot)?;
        if let Some(Token::Identifier(attr_name)) = self.advance() {
            attributes.push(attr_name.clone());
        } else {
            return Err(anyhow::anyhow!("Expected attribute name in group by"));
        }

        // Parse additional attributes
        while self.peek() == Some(&Token::Comma) {
            self.advance(); // consume ','
            self.expect(Token::Dot)?;
            if let Some(Token::Identifier(attr_name)) = self.advance() {
                attributes.push(attr_name.clone());
            } else {
                return Err(anyhow::anyhow!("Expected attribute name in group by"));
            }
        }

        self.expect(Token::RightParen)?;
        Ok(attributes)
    }

    /// Parse having clause
    fn parse_having(&mut self) -> Result<Option<Filter>> {
        self.expect(Token::Having)?;
        self.expect(Token::LeftParen)?;

        let filter = self.parse_filter_expression()?;

        self.expect(Token::RightParen)?;
        Ok(Some(filter))
    }

    /// Parse order by clause
    fn parse_order_by(&mut self) -> Result<Vec<OrderBy>> {
        self.expect(Token::Order)?;
        self.expect(Token::LeftParen)?;

        let mut order_items = Vec::new();

        // Parse first order item
        order_items.push(self.parse_order_item()?);

        // Parse additional order items
        while self.peek() == Some(&Token::Comma) {
            self.advance(); // consume ','
            order_items.push(self.parse_order_item()?);
        }

        self.expect(Token::RightParen)?;
        Ok(order_items)
    }

    /// Parse a single order by item
    fn parse_order_item(&mut self) -> Result<OrderBy> {
        // Parse attribute - could be .attribute or just alias name
        let attribute = if self.peek() == Some(&Token::Dot) {
            // Regular attribute reference like .revenue
            self.advance(); // consume '.'
            match self.advance() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => {
                    return Err(anyhow::anyhow!(
                        "Expected attribute name after '.' in order by"
                    ));
                }
            }
        } else if let Some(Token::Identifier(name)) = self.peek() {
            // Alias reference like avg_revenue (from aggregation alias)
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err(anyhow::anyhow!(
                "Expected attribute name or alias in order by"
            ));
        };

        // Parse optional direction
        let direction = match self.peek() {
            Some(Token::Asc) => {
                self.advance();
                OrderDirection::Ascending
            }
            Some(Token::Desc) => {
                self.advance();
                OrderDirection::Descending
            }
            _ => OrderDirection::Ascending, // default
        };

        Ok(OrderBy {
            attribute,
            direction,
            entity_alias: None,
        })
    }

    /// Parse limit clause
    fn parse_limit(&mut self) -> Result<Option<u32>> {
        self.expect(Token::Limit)?;
        self.expect(Token::LeftParen)?;

        let limit = match self.advance() {
            Some(Token::Integer(n)) => *n as u32,
            _ => return Err(anyhow::anyhow!("Expected integer in limit clause")),
        };

        self.expect(Token::RightParen)?;
        Ok(Some(limit))
    }

    /// Parse page clause
    fn parse_page(&mut self) -> Result<Option<(u32, u32)>> {
        self.expect(Token::Page)?;
        self.expect(Token::LeftParen)?;

        let page_number = match self.advance() {
            Some(Token::Integer(n)) => *n as u32,
            _ => return Err(anyhow::anyhow!("Expected page number in page clause")),
        };

        self.expect(Token::Comma)?;

        let page_size = match self.advance() {
            Some(Token::Integer(n)) => *n as u32,
            _ => return Err(anyhow::anyhow!("Expected page size in page clause")),
        };

        self.expect(Token::RightParen)?;
        Ok(Some((page_number, page_size)))
    }

    /// Parse options clause
    fn parse_options(&mut self) -> Result<QueryOptions> {
        self.expect(Token::Options)?;
        self.expect(Token::LeftParen)?;

        let mut options = QueryOptions::default();

        // Parse first option
        self.parse_option(&mut options)?;

        // Parse additional options
        while self.peek() == Some(&Token::Comma) {
            self.advance(); // consume ','
            self.parse_option(&mut options)?;
        }

        self.expect(Token::RightParen)?;
        Ok(options)
    }

    /// Parse a single option
    fn parse_option(&mut self, options: &mut QueryOptions) -> Result<()> {
        let option_name = match self.advance() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => return Err(anyhow::anyhow!("Expected option name")),
        };

        // Expect ':' separator
        if self
            .peek()
            .map(|t| matches!(t, Token::Identifier(s) if s == ":"))
            .unwrap_or(false)
        {
            self.advance(); // consume ':'
        } else {
            return Err(anyhow::anyhow!("Expected ':' after option name"));
        }

        // Parse option value
        match option_name.as_str() {
            "nolock" => match self.advance() {
                Some(Token::True) => options.no_lock = true,
                Some(Token::False) => options.no_lock = false,
                _ => return Err(anyhow::anyhow!("Expected boolean value for nolock option")),
            },
            "returntotalrecordcount" => match self.advance() {
                Some(Token::True) => options.return_total_record_count = true,
                Some(Token::False) => options.return_total_record_count = false,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Expected boolean value for returntotalrecordcount option"
                    ));
                }
            },
            "formatted" => match self.advance() {
                Some(Token::True) => options.formatted = true,
                Some(Token::False) => options.formatted = false,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Expected boolean value for formatted option"
                    ));
                }
            },
            _ => {
                // Custom option - parse as string
                let value = match self.advance() {
                    Some(Token::String(s)) => s.clone(),
                    Some(Token::Identifier(s)) => s.clone(),
                    Some(Token::True) => "true".to_string(),
                    Some(Token::False) => "false".to_string(),
                    _ => return Err(anyhow::anyhow!("Expected value for option {}", option_name)),
                };
                options.custom_options.insert(option_name, value);
            }
        }

        Ok(())
    }

    /// Parse filter value (string, number, date, list, etc.)
    fn parse_filter_value(&mut self) -> Result<FilterValue> {
        match self.advance() {
            Some(Token::String(s)) => Ok(FilterValue::String(s.clone())),
            Some(Token::Number(n)) => Ok(FilterValue::Number(*n)),
            Some(Token::Integer(i)) => Ok(FilterValue::Integer(*i)),
            Some(Token::True) => Ok(FilterValue::Boolean(true)),
            Some(Token::False) => Ok(FilterValue::Boolean(false)),
            Some(Token::Null) => Ok(FilterValue::Null),
            Some(Token::Date(d)) => Ok(FilterValue::Date(d.clone())),
            Some(Token::LeftBracket) => {
                // Parse list value
                let mut values = Vec::new();

                if self.peek() != Some(&Token::RightBracket) {
                    values.push(self.parse_filter_value()?);

                    while self.peek() == Some(&Token::Comma) {
                        self.advance(); // consume ','
                        values.push(self.parse_filter_value()?);
                    }
                }

                self.expect(Token::RightBracket)?;
                Ok(FilterValue::List(values))
            }
            _ => Err(anyhow::anyhow!("Expected filter value")),
        }
    }

    /// Parse filter operator
    fn parse_filter_operator(&mut self) -> Result<FilterOperator> {
        match self.advance() {
            Some(Token::Equal) => Ok(FilterOperator::Equal),
            Some(Token::NotEqual) => Ok(FilterOperator::NotEqual),
            Some(Token::GreaterThan) => Ok(FilterOperator::GreaterThan),
            Some(Token::GreaterEqual) => Ok(FilterOperator::GreaterThanOrEqual),
            Some(Token::LessThan) => Ok(FilterOperator::LessThan),
            Some(Token::LessEqual) => Ok(FilterOperator::LessThanOrEqual),
            Some(Token::Like) => Ok(FilterOperator::Like),
            Some(Token::NotLike) => Ok(FilterOperator::NotLike),
            Some(Token::BeginsWith) => Ok(FilterOperator::BeginsWith),
            Some(Token::EndsWith) => Ok(FilterOperator::EndsWith),
            Some(Token::In) => Ok(FilterOperator::In),
            Some(Token::NotIn) => Ok(FilterOperator::NotIn),
            Some(Token::Between) => Ok(FilterOperator::Between),
            _ => Err(anyhow::anyhow!("Expected filter operator")),
        }
    }

    /// Helper: Check if current token matches expected token
    fn expect(&mut self, expected: Token) -> Result<()> {
        if let Some(token) = self.advance() {
            if std::mem::discriminant(token) == std::mem::discriminant(&expected) {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Expected {:?}, found {:?}",
                    expected,
                    token
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "Expected {:?}, found end of input",
                expected
            ))
        }
    }

    /// Helper: Peek at current token without consuming
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    /// Helper: Advance to next token
    fn advance(&mut self) -> Option<&Token> {
        if self.current < self.tokens.len() {
            self.current += 1;
            self.tokens.get(self.current - 1)
        } else {
            None
        }
    }

    /// Helper: Check if we're at end of tokens
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || matches!(self.peek(), Some(Token::Eof))
    }
}
