//! C# mapping file parser
//!
//! Extracts field mappings from C# Dynamics 365 migration mapping files.
//! These files contain InternalMapping methods that map source fields to target fields.

use std::collections::HashMap;
use regex::Regex;

/// Parse C# field mappings from file content
///
/// Extracts source->target field mappings from InternalMapping method body.
/// Skips commented lines and handles various C# syntax patterns.
///
/// # Example
/// ```csharp
/// protected override TargetEntity InternalMapping(SourceEntity source, MigrationOptions options)
/// {
///     return new TargetEntity(source.Id)
///     {
///         nrq_Name = source.cgk_name,              // Simple mapping
///         nrq_Date = source.cgk_date,              // Another one
///         //nrq_Skipped = source.cgk_skipped,      // Commented out (ignored)
///         nrq_Fund = FundXRef.GetTargetReference(source.cgk_fundid?.Id), // Complex (extracts cgk_fundid)
///     };
/// }
/// ```
///
/// Returns: HashMap<source_field, target_field>
pub fn parse_cs_field_mappings(content: &str) -> Result<HashMap<String, String>, String> {
    let mut mappings = HashMap::new();

    // Find InternalMapping method
    let method_start = content.find("InternalMapping")
        .ok_or("InternalMapping method not found in file")?;

    // Find the return statement with new TargetEntity
    let return_pos = content[method_start..].find("return new ")
        .ok_or("Return statement not found in InternalMapping")?;

    // Extract from return statement onwards
    let body_start = method_start + return_pos;

    // Find the object initializer braces
    let init_start = content[body_start..].find('{')
        .ok_or("Object initializer not found")?;

    // Find matching closing brace (simplified - assumes well-formed code)
    let init_body = &content[body_start + init_start..];

    // Regex patterns for different mapping styles:
    // 1. Simple: target = source.field
    // 2. Cast: target = (Type?)source.field
    // 3. Method call: target = Method(source.field)
    // 4. Chained: target = source.field?.Property
    // 5. Conditional: target = condition ? source.field : null

    // Main pattern: captures target field name and extracts source field
    // Matches lines like:
    //   nrq_Target = source.cgk_source,
    //   nrq_Target = (Type?)source.cgk_source,
    //   nrq_Target = Method(source.cgk_source?.Id),
    let pattern = Regex::new(
        r"(?m)^\s*(\w+)\s*=\s*.*?source\w*\.(\w+)"
    ).unwrap();

    // Split into lines to check for comments
    let lines: Vec<&str> = init_body.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        // Skip commented lines
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }

        // Try to match the pattern
        if let Some(captures) = pattern.captures(line) {
            let target_field = captures.get(1).unwrap().as_str().to_string();
            let source_field = captures.get(2).unwrap().as_str().to_string();

            // Skip system fields (commonly overridden but not user fields)
            if target_field == "OverriddenCreatedOn" || source_field == "Id" {
                continue;
            }

            // Skip if target starts with comment on same line
            if trimmed.starts_with("//") {
                continue;
            }

            log::debug!("Parsed mapping: {} -> {}", source_field, target_field);
            mappings.insert(source_field, target_field);
        }
    }

    if mappings.is_empty() {
        return Err("No field mappings found in file. Check file format.".to_string());
    }

    log::info!("Parsed {} field mappings from C# file", mappings.len());
    Ok(mappings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_mappings() {
        let content = r#"
        protected override nrq_Deadline InternalMapping(cgk_deadline sourceDeadline, MigrationOptions options)
        {
            return new nrq_Deadline(sourceDeadline.Id)
            {
                nrq_Name = sourceDeadline.cgk_name,
                nrq_Date = sourceDeadline.cgk_date,
                nrq_CommissionId = sourceDeadline.cgk_commissionid,
            };
        }
        "#;

        let result = parse_cs_field_mappings(content).unwrap();
        assert_eq!(result.get("cgk_name"), Some(&"nrq_Name".to_string()));
        assert_eq!(result.get("cgk_date"), Some(&"nrq_Date".to_string()));
        assert_eq!(result.get("cgk_commissionid"), Some(&"nrq_CommissionId".to_string()));
    }

    #[test]
    fn test_skip_commented_lines() {
        let content = r#"
        protected override Target InternalMapping(Source source, MigrationOptions options)
        {
            return new Target(source.Id)
            {
                nrq_Active = source.cgk_active,
                //nrq_Skipped = source.cgk_skipped,
                nrq_Other = source.cgk_other,
            };
        }
        "#;

        let result = parse_cs_field_mappings(content).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains_key("cgk_active"));
        assert!(result.contains_key("cgk_other"));
        assert!(!result.contains_key("cgk_skipped"));
    }

    #[test]
    fn test_complex_expressions() {
        let content = r#"
        protected override Target InternalMapping(Source source, MigrationOptions options)
        {
            return new Target(source.Id)
            {
                nrq_Fund = FundXRef.GetTargetReference(source.cgk_fundid?.Id),
                nrq_Status = (int?)source.cgk_status,
                nrq_President = source.cgk_presidentid,
            };
        }
        "#;

        let result = parse_cs_field_mappings(content).unwrap();
        assert_eq!(result.get("cgk_fundid"), Some(&"nrq_Fund".to_string()));
        assert_eq!(result.get("cgk_status"), Some(&"nrq_Status".to_string()));
        assert_eq!(result.get("cgk_presidentid"), Some(&"nrq_President".to_string()));
    }

    #[test]
    fn test_no_internal_mapping_method() {
        let content = "public class SomeClass { }";
        let result = parse_cs_field_mappings(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("InternalMapping"));
    }
}
