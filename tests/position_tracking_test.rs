use dynamics_cli::fql::{parse, tokenize};

#[test]
fn test_position_error_formatting() {
    // Test with a lexer error that has position information
    let input = ".account | .name % value"; // Invalid '%' character

    println!("Testing position tracking with invalid FQL: {}", input);

    // Step 1: Tokenize with positions
    let tokens_result = tokenize(input);
    match tokens_result {
        Ok(_) => panic!("Expected tokenization to fail"),
        Err(e) => {
            println!("Tokenization error with position info:");
            println!("{}", e);

            // Check if this is our custom ParseError with position info
            let error_str = format!("{}", e);
            assert!(
                error_str.contains("error:"),
                "Error should be formatted nicely"
            );
            assert!(
                error_str.contains("line"),
                "Error should contain line information"
            );
            assert!(
                error_str.contains("column"),
                "Error should contain column information"
            );
            assert!(
                error_str.contains("^"),
                "Error should contain position indicator"
            );
        }
    }
}

#[test]
fn test_lexer_position_error() {
    // Test with invalid characters to trigger lexer errors
    let input = ".account | .name = value"; // Single '=' should trigger error

    println!("Testing lexer error with: {}", input);

    let result = tokenize(input);
    match result {
        Ok(_) => panic!("Expected lexer to fail on single '='"),
        Err(e) => {
            println!("Lexer error:");
            println!("{}", e);

            let error_str = format!("{}", e);
            assert!(error_str.contains("error:"), "Should show error prefix");
            assert!(error_str.contains("line"), "Should show line number");
            assert!(error_str.contains("column"), "Should show column number");
            assert!(error_str.contains("^"), "Should show position pointer");
            assert!(error_str.contains("=="), "Should suggest correct operator");
        }
    }
}

#[test]
fn test_parser_position_error() {
    // Test with a lexer error to show another example of position tracking
    let input = ".account | .name $ \"test\""; // Invalid '$' character

    println!("Testing parser error with: {}", input);

    match tokenize(input) {
        Ok(_) => panic!("Expected tokenization to fail"),
        Err(e) => {
            println!("Lexer error with position:");
            println!("{}", e);

            let error_str = format!("{}", e);
            assert!(error_str.contains("error:"), "Should show error prefix");
            assert!(error_str.contains("line"), "Should show line number");
            assert!(error_str.contains("column"), "Should show column number");
            assert!(error_str.contains("^"), "Should show position pointer");
        }
    }
}

#[test]
fn test_successful_parsing_with_positions() {
    // Test that valid queries still work with the position-aware parser
    let input = ".account | .name, .revenue";

    println!("Testing successful parsing with: {}", input);

    match tokenize(input) {
        Ok(tokens) => {
            println!(
                "Tokens: {:?}",
                tokens.iter().map(|t| &t.token).collect::<Vec<_>>()
            );

            match parse(tokens, input) {
                Ok(query) => {
                    println!("Successfully parsed query: {:#?}", query);
                    assert_eq!(query.entity.name, "account");
                    assert!(query.attributes.len() > 0, "Should have attributes");
                }
                Err(e) => panic!("Expected successful parsing, got error: {}", e),
            }
        }
        Err(e) => panic!("Expected successful tokenization, got error: {}", e),
    }
}
