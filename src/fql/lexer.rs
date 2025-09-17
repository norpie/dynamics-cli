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

/// Error type for parsing errors with position information
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub position: TokenPosition,
    pub input: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "error: {}", self.message)?;

        // Find the line in the input
        let lines: Vec<&str> = self.input.lines().collect();
        if self.position.line > 0 && self.position.line <= lines.len() {
            let line = lines[self.position.line - 1];
            writeln!(f, "  --> line {}, column {}", self.position.line, self.position.column)?;
            writeln!(f, "   |")?;
            writeln!(f, "{:3} | {}", self.position.line, line)?;
            writeln!(f, "   | {}{}", " ".repeat(self.position.column.saturating_sub(1)), "^")?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

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
    let located_tokens = tokenize_with_positions(input)?;
    Ok(located_tokens.into_iter().map(|lt| lt.token).collect())
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
    let mut lexer = Lexer::new(input);
    lexer.tokenize()
}

struct Lexer {
    input: String,
    position: usize,
    line: usize,
    column: usize,
    tokens: Vec<LocatedToken>,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            position: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
        }
    }

    fn tokenize(&mut self) -> Result<Vec<LocatedToken>> {
        while let Some((pos, ch)) = self.advance() {
            let position = TokenPosition { line: self.line, column: self.column, offset: pos };

            match ch {
                // Skip whitespace and newlines, but track position
                ' ' | '\t' | '\r' => {
                    self.column += 1;
                    continue;
                },
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                    continue;
                },

                // Single character tokens
                '.' => self.add_token(Token::Dot, position, 1),
                '|' => self.add_token(Token::Pipe, position, 1),
                ',' => self.add_token(Token::Comma, position, 1),
                '(' => self.add_token(Token::LeftParen, position, 1),
                ')' => self.add_token(Token::RightParen, position, 1),
                '[' => self.add_token(Token::LeftBracket, position, 1),
                ']' => self.add_token(Token::RightBracket, position, 1),
                '*' => self.add_token(Token::Wildcard, position, 1),
                ':' => self.add_token(Token::Identifier(":".to_string()), position, 1),

                // Multi-character operators
                '=' => {
                    if self.peek_char() == Some('=') {
                        self.advance(); // consume second '='
                        self.add_token(Token::Equal, position, 2);
                    } else {
                        return self.error("Unexpected character '=', did you mean '=='?", position);
                    }
                },
                '!' => {
                    match self.peek_char() {
                        Some('=') => {
                            self.advance();
                            self.add_token(Token::NotEqual, position, 2);
                        },
                        Some('~') => {
                            self.advance();
                            self.add_token(Token::NotLike, position, 2);
                        },
                        Some('i') => {
                            // Check for "!in"
                            if self.peek_ahead(2) == "in" {
                                self.advance(); // consume 'i'
                                self.advance(); // consume 'n'
                                self.add_token(Token::NotIn, position, 3);
                            } else {
                                return self.error("Unexpected character '!', expected '!=', '!~', or '!in'", position);
                            }
                        },
                        _ => return self.error("Unexpected character '!', expected '!=', '!~', or '!in'", position),
                    }
                },
                '>' => {
                    if self.peek_char() == Some('=') {
                        self.advance();
                        self.add_token(Token::GreaterEqual, position, 2);
                    } else {
                        self.add_token(Token::GreaterThan, position, 1);
                    }
                },
                '<' => {
                    if self.peek_char() == Some('=') {
                        self.advance();
                        self.add_token(Token::LessEqual, position, 2);
                    } else {
                        self.add_token(Token::LessThan, position, 1);
                    }
                },
                '~' => self.add_token(Token::Like, position, 1),
                '^' => {
                    if self.peek_char() == Some('=') {
                        self.advance();
                        self.add_token(Token::BeginsWith, position, 2);
                    } else {
                        return self.error("Unexpected character '^', did you mean '^='?", position);
                    }
                },
                '$' => {
                    if self.peek_char() == Some('=') {
                        self.advance();
                        self.add_token(Token::EndsWith, position, 2);
                    } else {
                        return self.error("Unexpected character '$', did you mean '$='?", position);
                    }
                },
                '-' => {
                    if self.peek_char() == Some('>') {
                        self.advance();
                        self.add_token(Token::Arrow, position, 2);
                    } else if ch.is_ascii_digit() || self.peek_char().map_or(false, |c| c.is_ascii_digit()) {
                        // Negative number
                        let (token, consumed) = self.parse_number(pos)?;
                        self.add_token(token, position, consumed);
                        // Skip the characters we consumed (minus the current one)
                        for _ in 1..consumed {
                            self.advance();
                        }
                    } else {
                        return self.error("Unexpected character '-', did you mean '->' or a negative number?", position);
                    }
                },

                // String literals with double or single quotes
                quote_char if quote_char == '"' || quote_char == '\'' => {
                    let (string_val, consumed) = self.parse_string_literal(pos)?;
                    self.add_token(Token::String(string_val), position, consumed);
                    // Skip the characters we consumed (minus the current one)
                    for _ in 1..consumed {
                        self.advance();
                    }
                },

                // Date literals starting with @
                '@' => {
                    let (date_val, consumed) = self.parse_date_literal(pos)?;
                    self.add_token(Token::Date(date_val), position, consumed);
                    // Skip the characters we consumed (minus the current one)
                    for _ in 1..consumed {
                        self.advance();
                    }
                },

                // Numbers
                ch if ch.is_ascii_digit() => {
                    let (token, consumed) = self.parse_number(pos)?;
                    self.add_token(token, position, consumed);
                    // Skip the characters we consumed (minus the current one)
                    for _ in 1..consumed {
                        self.advance();
                    }
                },

                // Identifiers and keywords
                ch if is_identifier_start(ch) => {
                    let (token, consumed) = self.parse_keyword_or_identifier(pos)?;
                    self.add_token(token, position, consumed);
                    // Skip the characters we consumed (minus the current one)
                    for _ in 1..consumed {
                        self.advance();
                    }
                },

                _ => return self.error(&format!("Unexpected character '{}'", ch), position),
            }
        }

        self.tokens.push(LocatedToken {
            token: Token::Eof,
            position: TokenPosition { line: self.line, column: self.column, offset: self.input.len() }
        });

        Ok(std::mem::take(&mut self.tokens))
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        if self.position >= self.input.len() {
            return None;
        }

        let remaining = &self.input[self.position..];
        if let Some(ch) = remaining.chars().next() {
            let pos = self.position;
            self.position += ch.len_utf8();
            Some((pos, ch))
        } else {
            None
        }
    }

    fn peek_char(&self) -> Option<char> {
        if self.position >= self.input.len() {
            return None;
        }
        self.input[self.position..].chars().next()
    }

    fn peek_ahead(&self, n: usize) -> &str {
        let remaining = &self.input[self.position..];
        if remaining.len() >= n {
            &remaining[..n]
        } else {
            remaining
        }
    }

    fn add_token(&mut self, token: Token, position: TokenPosition, consumed: usize) {
        self.tokens.push(LocatedToken { token, position });
        self.column += consumed;
    }

    fn error<T>(&self, message: &str, position: TokenPosition) -> Result<T> {
        Err(anyhow::anyhow!(ParseError {
            message: message.to_string(),
            position,
            input: self.input.clone(),
        }))
    }

    fn parse_number(&self, start: usize) -> Result<(Token, usize)> {
        parse_number(&self.input, start)
    }

    fn parse_string_literal(&self, start: usize) -> Result<(String, usize)> {
        parse_string_literal(&self.input, start)
    }

    fn parse_date_literal(&self, start: usize) -> Result<(String, usize)> {
        parse_date_literal(&self.input, start)
    }

    fn parse_keyword_or_identifier(&self, start: usize) -> Result<(Token, usize)> {
        parse_keyword_or_identifier(&self.input, start)
    }
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

    Err(anyhow::anyhow!("Unterminated string literal"))
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
        return Err(anyhow::anyhow!("Invalid number"));
    }

    let consumed = pos - start;

    if has_dot {
        match number_str.parse::<f64>() {
            Ok(val) => Ok((Token::Number(val), consumed)),
            Err(_) => Err(anyhow::anyhow!("Invalid float number '{}'", number_str)),
        }
    } else {
        match number_str.parse::<i64>() {
            Ok(val) => Ok((Token::Integer(val), consumed)),
            Err(_) => Err(anyhow::anyhow!("Invalid integer '{}'", number_str)),
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
        return Err(anyhow::anyhow!("Invalid date literal"));
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
        _ => Err(anyhow::anyhow!("Unknown operator starting with '{}'", first_char)),
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