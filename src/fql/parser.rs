use anyhow::Result;
use crate::fql::ast::*;
use crate::fql::lexer::Token;

/// Parses a vector of tokens into an FQL AST
///
/// # Arguments
/// * `tokens` - Vector of tokens from lexer
///
/// # Returns
/// * `Ok(Query)` - Parsed query AST on success
/// * `Err(anyhow::Error)` - Parse error
///
/// # Examples
/// ```rust
/// use dynamics_cli::fql::{tokenize, parse};
///
/// let tokens = tokenize(".account | .name, .revenue")?;
/// let query = parse(tokens)?;
/// assert_eq!(query.entity.name, "account");
/// ```
pub fn parse(tokens: Vec<Token>) -> Result<Query> {
    let mut parser = Parser::new(tokens);
    parser.parse_query()
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
            entity: Entity { name: String::new(), alias: None },
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

            match self.peek() {
                Some(Token::Dot) | Some(Token::Wildcard) => {
                    // Attribute selection
                    let attrs = self.parse_attributes()?;
                    query.attributes.extend(attrs);
                },
                Some(Token::Join) | Some(Token::LeftJoin) => {
                    // Join clause
                    let joins = self.parse_joins()?;
                    query.joins.extend(joins);
                },
                Some(Token::Order) => {
                    // Order by clause
                    query.order = self.parse_order_by()?;
                },
                Some(Token::Limit) => {
                    // Limit clause
                    query.limit = self.parse_limit()?;
                },
                Some(Token::Page) => {
                    // Page clause
                    query.page = self.parse_page()?;
                },
                Some(Token::Group) => {
                    // Group by clause
                    query.group_by = self.parse_group_by()?;
                },
                Some(Token::Having) => {
                    // Having clause
                    query.having = self.parse_having()?;
                },
                Some(Token::Distinct) => {
                    // Distinct modifier
                    self.advance();
                    query.distinct = true;
                },
                Some(Token::Options) => {
                    // Options clause
                    query.options = self.parse_options()?;
                },
                Some(Token::Count) | Some(Token::Sum) | Some(Token::Avg) | Some(Token::Min) | Some(Token::Max) => {
                    // Aggregation functions
                    let aggs = self.parse_aggregations()?;
                    query.aggregations.extend(aggs);
                },
                _ => {
                    // Filter expression
                    let filter = self.parse_filter()?;
                    query.filters.push(filter);
                }
            }
        }

        Ok(query)
    }

    /// Parse entity selection (.account, .contact, etc.)
    fn parse_entity(&mut self) -> Result<Entity> {
        self.expect(Token::Dot)?;

        let name = match self.advance() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => return Err(anyhow::anyhow!("Expected entity name after '.'"))
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
                    return Err(anyhow::anyhow!("Expected attribute name after entity alias"));
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

        Ok(Attribute { name, alias, entity_alias })
    }

    /// Parse filter conditions
    fn parse_filters(&mut self) -> Result<Vec<Filter>> {
        let mut filters = Vec::new();

        let filter = self.parse_filter_expression()?;
        filters.push(filter);

        Ok(filters)
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
                },
                Token::Or => {
                    self.advance();
                    let right = self.parse_filter_term()?;
                    left = Filter::Or(vec![left, right]);
                },
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
                    return Err(anyhow::anyhow!("Expected attribute name after entity alias"));
                }
            } else {
                first_part
            }
        } else {
            return Err(anyhow::anyhow!("Expected attribute in filter condition"));
        };

        // Parse operator
        let operator = self.parse_filter_operator()?;

        // Parse value
        let value = self.parse_filter_value()?;

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
                },
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

        // Parse join condition
        let on_condition = self.parse_join_condition()?;

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
                                Token::Equal | Token::NotEqual | Token::GreaterThan | Token::GreaterEqual |
                                Token::LessThan | Token::LessEqual | Token::Like | Token::NotLike |
                                Token::BeginsWith | Token::EndsWith | Token::In | Token::NotIn | Token::Between => {
                                    is_filter = true;
                                    break;
                                },
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
                    },
                    Some(Token::Wildcard) => {
                        let attrs = self.parse_attributes()?;
                        attributes.extend(attrs);
                    },
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

    /// Parse join condition (on .attr or on .attr1 -> .attr2)
    fn parse_join_condition(&mut self) -> Result<JoinCondition> {
        // Parse first attribute
        self.expect(Token::Dot)?;
        let from_attribute = match self.advance() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => return Err(anyhow::anyhow!("Expected attribute name in join condition")),
        };

        // Check for arrow (->) indicating explicit target attribute
        if self.peek() == Some(&Token::Arrow) {
            self.advance(); // consume '->'
            self.expect(Token::Dot)?;
            let to_attribute = match self.advance() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => return Err(anyhow::anyhow!("Expected target attribute name after '->'")),
            };

            Ok(JoinCondition {
                from_attribute,
                to_attribute,
                from_entity_alias: None,
                to_entity_alias: None,
            })
        } else {
            // Simple join - same attribute name on both entities
            Ok(JoinCondition {
                from_attribute: from_attribute.clone(),
                to_attribute: from_attribute,
                from_entity_alias: None,
                to_entity_alias: None,
            })
        }
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
        // Parse attribute
        self.expect(Token::Dot)?;
        let attribute = match self.advance() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => return Err(anyhow::anyhow!("Expected attribute name in order by")),
        };

        // Parse optional direction
        let direction = match self.peek() {
            Some(Token::Asc) => {
                self.advance();
                OrderDirection::Ascending
            },
            Some(Token::Desc) => {
                self.advance();
                OrderDirection::Descending
            },
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
        if self.peek().map(|t| matches!(t, Token::Identifier(s) if s == ":")).unwrap_or(false) {
            self.advance(); // consume ':'
        } else {
            return Err(anyhow::anyhow!("Expected ':' after option name"));
        }

        // Parse option value
        match option_name.as_str() {
            "nolock" => {
                match self.advance() {
                    Some(Token::True) => options.no_lock = true,
                    Some(Token::False) => options.no_lock = false,
                    _ => return Err(anyhow::anyhow!("Expected boolean value for nolock option")),
                }
            },
            "returntotalrecordcount" => {
                match self.advance() {
                    Some(Token::True) => options.return_total_record_count = true,
                    Some(Token::False) => options.return_total_record_count = false,
                    _ => return Err(anyhow::anyhow!("Expected boolean value for returntotalrecordcount option")),
                }
            },
            "formatted" => {
                match self.advance() {
                    Some(Token::True) => options.formatted = true,
                    Some(Token::False) => options.formatted = false,
                    _ => return Err(anyhow::anyhow!("Expected boolean value for formatted option")),
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
            },
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
                Err(anyhow::anyhow!("Expected {:?}, found {:?}", expected, token))
            }
        } else {
            Err(anyhow::anyhow!("Expected {:?}, found end of input", expected))
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

    /// Helper: Get current token and advance
    fn consume(&mut self) -> Option<Token> {
        self.advance().cloned()
    }
}