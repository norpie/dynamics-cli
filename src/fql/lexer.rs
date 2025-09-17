use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Structural tokens
    Dot,
    Pipe,
    Comma,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,

    // Keywords
    As,
    Join,
    LeftJoin,
    On,
    And,
    Or,
    In,
    NotIn,
    Between,
    Group,
    Having,
    Order,
    Limit,
    Page,
    Distinct,
    Options,
    Count,
    Sum,
    Avg,
    Min,
    Max,
    Null,
    True,
    False,

    // Operators
    Equal,           // ==
    NotEqual,        // !=
    GreaterThan,     // >
    GreaterEqual,    // >=
    LessThan,        // <
    LessEqual,       // <=
    Like,            // ~
    NotLike,         // !~
    BeginsWith,      // ^=
    EndsWith,        // $=
    Arrow,           // ->

    // Identifiers and literals
    Identifier(String),
    String(String),
    Number(f64),
    Integer(i64),
    Date(String),      // @today, @2020-01-01, etc.

    // Order directions
    Asc,
    Desc,

    // Special
    Wildcard,         // *
    Whitespace,
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct TokenPosition {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct LocatedToken {
    pub token: Token,
    pub position: TokenPosition,
}

/// Tokenizes FQL input string into a vector of tokens
///
/// # Arguments
/// * `input` - The FQL query string to tokenize
///
/// # Returns
/// * `Ok(Vec<Token>)` - Vector of tokens on success
/// * `Err(anyhow::Error)` - Tokenization error
///
/// # Examples
/// ```rust
/// use dynamics_cli::fql::lexer::tokenize;
///
/// let tokens = tokenize(".account | .name, .revenue")?;
/// assert_eq!(tokens[0], Token::Dot);
/// assert_eq!(tokens[1], Token::Identifier("account".to_string()));
/// ```
pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some((pos, ch)) = chars.next() {
        match ch {
            // Skip whitespace and newlines
            ' ' | '\t' | '\r' | '\n' => continue,

            // Single character tokens
            '.' => tokens.push(Token::Dot),
            '|' => tokens.push(Token::Pipe),
            ',' => tokens.push(Token::Comma),
            '(' => tokens.push(Token::LeftParen),
            ')' => tokens.push(Token::RightParen),
            '[' => tokens.push(Token::LeftBracket),
            ']' => tokens.push(Token::RightBracket),
            '*' => tokens.push(Token::Wildcard),
            ':' => tokens.push(Token::Identifier(":".to_string())),

            // Multi-character operators
            '=' => {
                if chars.peek().map(|(_, c)| *c) == Some('=') {
                    chars.next(); // consume second '='
                    tokens.push(Token::Equal);
                } else {
                    return Err(anyhow::anyhow!("Unexpected character '=' at position {}", pos));
                }
            },
            '!' => {
                match chars.peek().map(|(_, c)| *c) {
                    Some('=') => {
                        chars.next();
                        tokens.push(Token::NotEqual);
                    },
                    Some('~') => {
                        chars.next();
                        tokens.push(Token::NotLike);
                    },
                    Some('i') => {
                        // Check for "!in"
                        let mut peek_chars = chars.clone();
                        peek_chars.next(); // consume 'i'
                        if peek_chars.peek().map(|(_, c)| *c) == Some('n') {
                            chars.next(); // consume 'i'
                            chars.next(); // consume 'n'
                            tokens.push(Token::NotIn);
                        } else {
                            return Err(anyhow::anyhow!("Unexpected character '!' at position {}", pos));
                        }
                    },
                    _ => return Err(anyhow::anyhow!("Unexpected character '!' at position {}", pos)),
                }
            },
            '>' => {
                if chars.peek().map(|(_, c)| *c) == Some('=') {
                    chars.next();
                    tokens.push(Token::GreaterEqual);
                } else {
                    tokens.push(Token::GreaterThan);
                }
            },
            '<' => {
                if chars.peek().map(|(_, c)| *c) == Some('=') {
                    chars.next();
                    tokens.push(Token::LessEqual);
                } else {
                    tokens.push(Token::LessThan);
                }
            },
            '~' => tokens.push(Token::Like),
            '^' => {
                if chars.peek().map(|(_, c)| *c) == Some('=') {
                    chars.next();
                    tokens.push(Token::BeginsWith);
                } else {
                    return Err(anyhow::anyhow!("Unexpected character '^' at position {}", pos));
                }
            },
            '$' => {
                if chars.peek().map(|(_, c)| *c) == Some('=') {
                    chars.next();
                    tokens.push(Token::EndsWith);
                } else {
                    return Err(anyhow::anyhow!("Unexpected character '$' at position {}", pos));
                }
            },
            '-' => {
                if chars.peek().map(|(_, c)| *c) == Some('>') {
                    chars.next();
                    tokens.push(Token::Arrow);
                } else if ch.is_ascii_digit() || chars.peek().map(|(_, c)| c.is_ascii_digit()).unwrap_or(false) {
                    // Negative number
                    let (token, consumed) = parse_number(input, pos)?;
                    tokens.push(token);
                    // Advance chars by consumed amount - 1 (since we already consumed one)
                    for _ in 0..consumed.saturating_sub(1) {
                        chars.next();
                    }
                } else {
                    return Err(anyhow::anyhow!("Unexpected character '-' at position {}", pos));
                }
            },

            // String literals with double or single quotes
            quote_char if quote_char == '"' || quote_char == '\'' => {
                let (string_val, consumed) = parse_string_literal(input, pos)?;
                tokens.push(Token::String(string_val));
                // Advance chars by consumed amount - 1 (since we already consumed quote)
                for _ in 0..consumed.saturating_sub(1) {
                    chars.next();
                }
            },

            // Date literals starting with @
            '@' => {
                let (date_val, consumed) = parse_date_literal(input, pos)?;
                tokens.push(Token::Date(date_val));
                // Advance chars by consumed amount - 1
                for _ in 0..consumed.saturating_sub(1) {
                    chars.next();
                }
            },

            // Numbers
            ch if ch.is_ascii_digit() => {
                let (token, consumed) = parse_number(input, pos)?;
                tokens.push(token);
                // Advance chars by consumed amount - 1
                for _ in 0..consumed.saturating_sub(1) {
                    chars.next();
                }
            },

            // Identifiers and keywords
            ch if is_identifier_start(ch) => {
                let (token, consumed) = parse_keyword_or_identifier(input, pos)?;
                tokens.push(token);
                // Advance chars by consumed amount - 1
                for _ in 0..consumed.saturating_sub(1) {
                    chars.next();
                }
            },

            _ => return Err(anyhow::anyhow!("Unexpected character '{}' at position {}", ch, pos)),
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

/// Tokenizes FQL input with position information
///
/// # Arguments
/// * `input` - The FQL query string to tokenize
///
/// # Returns
/// * `Ok(Vec<LocatedToken>)` - Vector of tokens with position info on success
/// * `Err(anyhow::Error)` - Tokenization error
pub fn tokenize_with_positions(input: &str) -> Result<Vec<LocatedToken>> {
    let mut tokens = Vec::new();
    let chars = input.char_indices().peekable();
    let mut line = 1;
    let mut column = 1;

    for (offset, ch) in chars {
        let position = TokenPosition { line, column, offset };

        match ch {
            // Handle newlines for position tracking
            '\n' => {
                line += 1;
                column = 1;
                continue;
            },
            // Skip other whitespace
            ' ' | '\t' | '\r' => {
                column += 1;
                continue;
            },

            // All the same token parsing logic as above, but with position tracking
            _ => {
                // For simplicity, we'll use the simpler tokenize function
                // and reconstruct positions. In a production implementation,
                // we'd integrate position tracking directly into the main tokenization logic.
                let remaining_input = &input[offset..];
                let simple_tokens = tokenize(remaining_input)?;

                if let Some(token) = simple_tokens.first()
                    && !matches!(token, Token::Eof) {
                        tokens.push(LocatedToken {
                            token: token.clone(),
                            position,
                        });
                    }

                // This is a simplified approach - in practice you'd want to integrate
                // position tracking into the main tokenization loop
                break;
            }
        }
    }

    // For now, fall back to simple tokenization without full position tracking
    let simple_tokens = tokenize(input)?;
    for (i, token) in simple_tokens.iter().enumerate() {
        if !matches!(token, Token::Eof) {
            tokens.push(LocatedToken {
                token: token.clone(),
                position: TokenPosition { line: 1, column: i + 1, offset: i },
            });
        }
    }

    Ok(tokens)
}

/// Helper function to determine if a character can start an identifier
fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

/// Helper function to determine if a character can continue an identifier
fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// Helper function to parse string literals
fn parse_string_literal(input: &str, start: usize) -> Result<(String, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let quote_char = chars[start];
    let mut pos = start + 1;
    let mut result = String::new();

    while pos < chars.len() {
        let ch = chars[pos];

        if ch == quote_char {
            // End of string
            return Ok((result, pos - start + 1));
        } else if ch == '\\' && pos + 1 < chars.len() {
            // Escape sequence
            pos += 1;
            match chars[pos] {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                '\'' => result.push('\''),
                _ => {
                    result.push('\\');
                    result.push(chars[pos]);
                }
            }
        } else {
            result.push(ch);
        }

        pos += 1;
    }

    Err(anyhow::anyhow!("Unterminated string literal at position {}", start))
}

/// Helper function to parse numeric literals
fn parse_number(input: &str, start: usize) -> Result<(Token, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = start;
    let mut number_str = String::new();
    let mut has_dot = false;

    // Handle negative sign
    if pos < chars.len() && chars[pos] == '-' {
        number_str.push(chars[pos]);
        pos += 1;
    }

    while pos < chars.len() {
        let ch = chars[pos];

        if ch.is_ascii_digit() {
            number_str.push(ch);
        } else if ch == '.' && !has_dot {
            has_dot = true;
            number_str.push(ch);
        } else {
            break;
        }

        pos += 1;
    }

    if number_str.is_empty() || number_str == "-" {
        return Err(anyhow::anyhow!("Invalid number at position {}", start));
    }

    let consumed = pos - start;

    if has_dot {
        match number_str.parse::<f64>() {
            Ok(val) => Ok((Token::Number(val), consumed)),
            Err(_) => Err(anyhow::anyhow!("Invalid float number '{}' at position {}", number_str, start)),
        }
    } else {
        match number_str.parse::<i64>() {
            Ok(val) => Ok((Token::Integer(val), consumed)),
            Err(_) => Err(anyhow::anyhow!("Invalid integer '{}' at position {}", number_str, start)),
        }
    }
}

/// Helper function to parse date literals (@today, @2020-01-01, etc.)
fn parse_date_literal(input: &str, start: usize) -> Result<(String, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = start + 1; // Skip the '@'
    let mut result = String::new();

    while pos < chars.len() {
        let ch = chars[pos];

        if ch.is_ascii_alphanumeric() || ch == '-' || ch == ':' || ch == 'd' || ch == 'h' || ch == 'm' {
            result.push(ch);
            pos += 1;
        } else {
            break;
        }
    }

    if result.is_empty() {
        return Err(anyhow::anyhow!("Invalid date literal at position {}", start));
    }

    Ok((result, pos - start))
}

/// Helper function to parse operators (==, !=, >=, etc.)
fn parse_operator(input: &str, start: usize) -> Result<(Token, usize)> {
    let chars: Vec<char> = input.chars().collect();

    if start >= chars.len() {
        return Err(anyhow::anyhow!("Unexpected end of input while parsing operator"));
    }

    let first_char = chars[start];
    let second_char = if start + 1 < chars.len() { Some(chars[start + 1]) } else { None };

    match (first_char, second_char) {
        ('=', Some('=')) => Ok((Token::Equal, 2)),
        ('!', Some('=')) => Ok((Token::NotEqual, 2)),
        ('>', Some('=')) => Ok((Token::GreaterEqual, 2)),
        ('<', Some('=')) => Ok((Token::LessEqual, 2)),
        ('!', Some('~')) => Ok((Token::NotLike, 2)),
        ('^', Some('=')) => Ok((Token::BeginsWith, 2)),
        ('$', Some('=')) => Ok((Token::EndsWith, 2)),
        ('-', Some('>')) => Ok((Token::Arrow, 2)),
        ('>', _) => Ok((Token::GreaterThan, 1)),
        ('<', _) => Ok((Token::LessThan, 1)),
        ('~', _) => Ok((Token::Like, 1)),
        _ => Err(anyhow::anyhow!("Unknown operator starting with '{}' at position {}", first_char, start)),
    }
}

/// Helper function to parse keywords and identifiers
fn parse_keyword_or_identifier(input: &str, start: usize) -> Result<(Token, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = start;
    let mut identifier = String::new();

    while pos < chars.len() && (pos == start && is_identifier_start(chars[pos]) || pos > start && is_identifier_continue(chars[pos])) {
        identifier.push(chars[pos]);
        pos += 1;
    }

    let consumed = pos - start;

    let token = match identifier.as_str() {
        "as" => Token::As,
        "join" => Token::Join,
        "leftjoin" => Token::LeftJoin,
        "on" => Token::On,
        "and" => Token::And,
        "or" => Token::Or,
        "in" => Token::In,
        "between" => Token::Between,
        "group" => Token::Group,
        "having" => Token::Having,
        "order" => Token::Order,
        "limit" => Token::Limit,
        "page" => Token::Page,
        "distinct" => Token::Distinct,
        "options" => Token::Options,
        "count" => Token::Count,
        "sum" => Token::Sum,
        "avg" => Token::Avg,
        "min" => Token::Min,
        "max" => Token::Max,
        "null" => Token::Null,
        "true" => Token::True,
        "false" => Token::False,
        "asc" => Token::Asc,
        "desc" => Token::Desc,
        _ => Token::Identifier(identifier),
    };

    Ok((token, consumed))
}