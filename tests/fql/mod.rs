/// FQL (FetchXML Query Language) parser tests
///
/// This module contains comprehensive tests for the FQL parser,
/// organized by functionality area.

pub mod basic_queries;
pub mod filtering;
pub mod joins;
pub mod aggregation;
pub mod ordering_pagination;
pub mod advanced_features;
pub mod integration_examples;

use anyhow::Result;
use dynamics_cli::fql::{parse, to_fetchxml, tokenize};

/// Normalize XML for comparison by removing extra whitespace and newlines
pub fn normalize_xml(xml: &str) -> String {
    xml.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("")
        .replace("> <", "><")
        .replace(" />", "/>")
}

/// Test helper to run the complete FQL parsing pipeline
pub fn test_fql_to_xml(fql: &str, expected_xml: &str) -> Result<()> {
    println!("Testing FQL: {}", fql);

    // Step 1: Tokenize
    let tokens = tokenize(fql)?;
    println!(
        "Tokens: {:?}",
        tokens.iter().map(|t| &t.token).collect::<Vec<_>>()
    );

    // Step 2: Parse to AST
    let ast = parse(tokens, fql)?;
    println!("AST: {:?}", ast);

    // Step 3: Generate XML
    let xml = to_fetchxml(ast)?;
    println!("Generated XML:\n{}", xml);
    println!("Expected XML:\n{}", expected_xml);

    // Compare XML with normalized whitespace
    let normalized_generated = normalize_xml(&xml);
    let normalized_expected = normalize_xml(expected_xml);

    if normalized_generated != normalized_expected {
        eprintln!("XML mismatch!");
        eprintln!("Generated (normalized): {}", normalized_generated);
        eprintln!("Expected (normalized):  {}", normalized_expected);
        return Err(anyhow::anyhow!("Generated XML does not match expected XML"));
    }

    Ok(())
}