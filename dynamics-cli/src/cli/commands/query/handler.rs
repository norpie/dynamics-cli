//! Query command handler with new API client integration

use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::time::Instant;

use crate::api::ClientManager;
use crate::config::Config;
use crate::fql::{parse, to_fetchxml, to_fetchxml_pretty, tokenize};
use super::{OutputFormat, QueryCommands};

/// Handle the query command with the new streamlined interface
pub async fn handle_query_command(args: QueryCommands) -> Result<()> {
    let client_manager = crate::client_manager();
    // Validate arguments
    if args.query.is_none() && args.file.is_none() {
        anyhow::bail!("Either provide a query string or use --file to specify a query file");
    }

    if args.query.is_some() && args.file.is_some() {
        anyhow::bail!("Cannot specify both query string and --file option");
    }

    // Read query from source
    let query_text = if let Some(query) = args.query {
        query
    } else if let Some(file_path) = args.file {
        if !file_path.exists() {
            anyhow::bail!("Query file does not exist: {}", file_path.display());
        }

        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read query file: {}", file_path.display()))?;

        let trimmed = content.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Query file is empty: {}", file_path.display());
        }

        println!("📄 Reading query from: {}", file_path.display().to_string().cyan());
        trimmed.to_string()
    } else {
        unreachable!("Validation above ensures one of query or file is present");
    };

    println!("📝 Query: {}", query_text.dimmed());
    println!();

    // Parse FQL to FetchXML
    println!("🔄 {}", "Parsing FQL query...".dimmed());
    let start_parse = Instant::now();

    let tokens = tokenize(&query_text)
        .context("Failed to tokenize FQL query")?;

    let ast = parse(tokens, &query_text)
        .context("Failed to parse FQL query")?;

    // Extract entity name from AST for pluralization
    let entity_name = ast.entity.name.clone();

    let fetchxml = if args.pretty && args.dry {
        to_fetchxml_pretty(ast)
    } else {
        to_fetchxml(ast)
    }.context("Failed to generate FetchXML from query")?;

    let parse_duration = start_parse.elapsed();

    if args.stats {
        println!("⏱️  Parse time: {:.2}ms", parse_duration.as_secs_f64() * 1000.0);
    }

    // If dry run, just show the FetchXML
    if args.dry {
        println!("🔍 Generated FetchXML:");
        println!();
        println!("{}", fetchxml);
        return Ok(());
    }

    // For execution, we need an environment
    let current_env = client_manager.get_current_environment().await
        .ok_or_else(|| anyhow::anyhow!(
            "No environment selected. Use 'dynamics-cli auth env select' to choose one."
        ))?;

    println!("🌍 Using environment: {}", current_env.bright_green().bold());

    // Execute query
    println!("🚀 {}", "Executing query...".dimmed());
    let start_exec = Instant::now();

    let client = client_manager.get_client(&current_env).await?;

    // Execute the query using the new API client with entity name
    let result = client.execute_fetchxml(&entity_name, &fetchxml).await
        .context("Failed to execute query")?;

    let exec_duration = start_exec.elapsed();

    if args.stats {
        println!("⏱️  Execution time: {:.2}ms", exec_duration.as_secs_f64() * 1000.0);
        println!("📊 Total time: {:.2}ms", (parse_duration + exec_duration).as_secs_f64() * 1000.0);
        println!();
    }

    // Format and output results
    let formatted_output = format_output(&result, &args.format, args.pretty)?;

    if let Some(output_path) = args.output {
        fs::write(&output_path, &formatted_output)
            .with_context(|| format!("Failed to write output to: {}", output_path.display()))?;
        println!("💾 Results saved to: {}", output_path.display().to_string().bright_green());
    } else {
        println!("📋 Results:");
        println!();
        println!("{}", formatted_output);
    }

    Ok(())
}

/// Format query results according to the specified output format
fn format_output(data: &serde_json::Value, format: &OutputFormat, pretty: bool) -> Result<String> {
    match format {
        OutputFormat::Json => {
            if pretty {
                serde_json::to_string_pretty(data)
                    .context("Failed to format JSON output")
            } else {
                serde_json::to_string(data)
                    .context("Failed to format JSON output")
            }
        }
        OutputFormat::Xml => {
            // Convert JSON to XML representation
            let xml_string = json_to_xml(data)?;
            if pretty {
                // Try to pretty-print XML, fallback to raw if it fails
                match quick_xml::reader::Reader::from_str(&xml_string) {
                    mut reader => {
                        let mut writer = quick_xml::Writer::new_with_indent(Vec::new(), b' ', 2);
                        let mut buf = Vec::new();
                        loop {
                            match reader.read_event_into(&mut buf) {
                                Ok(quick_xml::events::Event::Eof) => break,
                                Ok(event) => writer.write_event(event)?,
                                Err(_) => return Ok(xml_string), // fallback to raw
                            }
                            buf.clear();
                        }
                        Ok(String::from_utf8(writer.into_inner())
                            .unwrap_or(xml_string))
                    }
                }
            } else {
                Ok(xml_string)
            }
        }
        OutputFormat::Csv => {
            json_to_csv(data)
        }
        OutputFormat::FetchXml => {
            // This shouldn't happen in normal execution, but handle it
            Ok("FetchXML format only available with --dry flag".to_string())
        }
    }
}

/// Convert JSON data to XML representation
fn json_to_xml(data: &serde_json::Value) -> Result<String> {
    match data {
        serde_json::Value::Object(obj) => {
            let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<result>\n");
            for (key, value) in obj {
                xml.push_str(&format!("  <{}>{}</{}>\n", key, json_value_to_string(value), key));
            }
            xml.push_str("</result>");
            Ok(xml)
        }
        serde_json::Value::Array(arr) => {
            let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results>\n");
            for (i, item) in arr.iter().enumerate() {
                xml.push_str(&format!("  <item index=\"{}\">{}</item>\n", i, json_value_to_string(item)));
            }
            xml.push_str("</results>");
            Ok(xml)
        }
        _ => Ok(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<value>{}</value>", json_value_to_string(data)))
    }
}

/// Convert JSON data to CSV representation
fn json_to_csv(data: &serde_json::Value) -> Result<String> {
    match data {
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return Ok("No data\n".to_string());
            }

            let mut csv = String::new();

            // Extract headers from first object
            if let Some(serde_json::Value::Object(first_obj)) = arr.first() {
                let headers: Vec<String> = first_obj.keys().cloned().collect();
                csv.push_str(&headers.join(","));
                csv.push('\n');

                // Add data rows
                for item in arr {
                    if let serde_json::Value::Object(obj) = item {
                        let row: Vec<String> = headers.iter()
                            .map(|h| csv_escape(&json_value_to_string(obj.get(h).unwrap_or(&serde_json::Value::Null))))
                            .collect();
                        csv.push_str(&row.join(","));
                        csv.push('\n');
                    }
                }
            }
            Ok(csv)
        }
        serde_json::Value::Object(obj) => {
            let mut csv = String::from("key,value\n");
            for (key, value) in obj {
                csv.push_str(&format!("{},{}\n", csv_escape(key), csv_escape(&json_value_to_string(value))));
            }
            Ok(csv)
        }
        _ => Ok(format!("value\n{}\n", csv_escape(&json_value_to_string(data))))
    }
}

/// Convert a JSON value to a string representation
fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => value.to_string(),
    }
}

/// Escape a string for CSV output
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

