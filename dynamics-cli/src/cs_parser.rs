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
///     var entity = new TargetEntity(source.Id)
///     {
///         nrq_Name = source.cgk_name,              // Simple mapping
///         nrq_Date = source.cgk_date,              // Another one
///         //nrq_Skipped = source.cgk_skipped,      // Commented out (ignored)
///         nrq_Fund = FundXRef.GetTargetReference(source.cgk_fundid?.Id), // Complex (extracts cgk_fundid)
///     };
///     return entity;
/// }
/// ```
///
/// Returns: HashMap<source_field, target_field>
pub fn parse_cs_field_mappings(content: &str) -> Result<HashMap<String, String>, String> {
    let mut mappings = HashMap::new();

    // Find InternalMapping method signature
    let method_start = content.find("InternalMapping")
        .ok_or("InternalMapping method not found in file")?;

    // Extract source parameter name from method signature
    // Pattern: InternalMapping(Type sourceName, ...)
    let sig_pattern = Regex::new(r"InternalMapping\s*\(\s*\w+\s+(\w+)\s*,").unwrap();
    let source_var_name = if let Some(caps) = sig_pattern.captures(&content[method_start..]) {
        caps.get(1).unwrap().as_str()
    } else {
        return Err("Could not extract source parameter name from InternalMapping signature".to_string());
    };

    log::debug!("Detected source variable name: {}", source_var_name);

    // Find object initializer - could be from "return new" or "var x = new"
    let method_body_start = content[method_start..].find('{')
        .ok_or("Method body not found")?;
    let method_body = &content[method_start + method_body_start..];

    // Look for "new TargetType(" followed by object initializer
    let new_pattern = Regex::new(r"new\s+\w+\s*\([^)]*\)\s*\{").unwrap();
    let init_start = if let Some(mat) = new_pattern.find(method_body) {
        mat.start()
    } else {
        return Err("Object initializer not found in method body".to_string());
    };

    // Find the opening brace of the initializer
    let init_brace = method_body[init_start..].find('{')
        .ok_or("Object initializer brace not found")?;
    let init_body = &method_body[init_start + init_brace..];

    // Build regex pattern with captured source variable name
    // Matches: targetField = ...sourceVarName.sourceField...
    // This handles various patterns:
    // - Simple: sourceVar.field
    // - Cast: (Type?)sourceVar.field
    // - Method: Method(sourceVar.field)
    // - XRef: XRef.GetTargetReference(sourceVar.field?.Id)
    // - Cast: sourceVar.field?.Cast<Type>()
    let field_pattern = Regex::new(&format!(
        r"(?m)^\s*(\w+)\s*=\s*.*?{}\.(\w+)",
        regex::escape(source_var_name)
    )).unwrap();

    // Split into lines to check for comments
    let lines: Vec<&str> = init_body.lines().collect();

    for line in lines.iter() {
        // Skip commented lines
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }

        // Try to match the pattern
        if let Some(captures) = field_pattern.captures(line) {
            let target_field = captures.get(1).unwrap().as_str().to_string();
            let source_field = captures.get(2).unwrap().as_str().to_string();

            // Skip system fields (commonly overridden but not user fields)
            if target_field == "OverriddenCreatedOn" || source_field == "Id" {
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

    #[test]
    fn test_variable_assignment_pattern() {
        let content = r#"
        protected override nrq_Request InternalMapping(cgk_request sourceRequest, MigrationOptions options)
        {
            var request = new nrq_Request(sourceRequest.Id)
            {
                nrq_Name = sourceRequest.cgk_name,
                nrq_Date = sourceRequest.cgk_date,
            };
            return request;
        }
        "#;

        let result = parse_cs_field_mappings(content).unwrap();
        assert_eq!(result.get("cgk_name"), Some(&"nrq_Name".to_string()));
        assert_eq!(result.get("cgk_date"), Some(&"nrq_Date".to_string()));
    }

    #[test]
    fn test_xref_patterns() {
        let content = r#"
        protected override Target InternalMapping(Source sourceEntity, MigrationOptions options)
        {
            return new Target(sourceEntity.Id)
            {
                nrq_Fund = FundXRef.GetTargetReference(sourceEntity.cgk_fundid?.Id),
                nrq_Category = CategoryXRef.GetTargetReference(sourceEntity.cgk_categoryid?.Id),
                nrq_Simple = sourceEntity.cgk_simple,
            };
        }
        "#;

        let result = parse_cs_field_mappings(content).unwrap();
        assert_eq!(result.get("cgk_fundid"), Some(&"nrq_Fund".to_string()));
        assert_eq!(result.get("cgk_categoryid"), Some(&"nrq_Category".to_string()));
        assert_eq!(result.get("cgk_simple"), Some(&"nrq_Simple".to_string()));
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_cast_and_method_call_patterns() {
        let content = r#"
        protected override Target InternalMapping(Source src, MigrationOptions options)
        {
            var entity = new Target(src.Id)
            {
                nrq_Status = (int?)src.cgk_status,
                nrq_Project = src.cgk_projectid?.Cast<nrq_Project>(),
                nrq_Review = ConvertReview(src.cgk_review),
                nrq_Type = ConvertSubmissionType(src.vaf_type),
            };
            return entity;
        }
        "#;

        let result = parse_cs_field_mappings(content).unwrap();
        assert_eq!(result.get("cgk_status"), Some(&"nrq_Status".to_string()));
        assert_eq!(result.get("cgk_projectid"), Some(&"nrq_Project".to_string()));
        assert_eq!(result.get("cgk_review"), Some(&"nrq_Review".to_string()));
        assert_eq!(result.get("vaf_type"), Some(&"nrq_Type".to_string()));
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_mixed_patterns_with_comments() {
        let content = r#"
        protected override nrq_Entity InternalMapping(cgk_entity sourceEntity, MigrationOptions options)
        {
            var result = new nrq_Entity(sourceEntity.Id)
            {
                nrq_Active = sourceEntity.cgk_active,
                //nrq_Disabled = sourceEntity.cgk_disabled,
                nrq_FundId = FundXRef.GetTargetReference(sourceEntity.cgk_fundid?.Id),
                //nrq_Skipped = CategoryXRef.Get(sourceEntity.cgk_skipped),
                nrq_Amount = (decimal?)sourceEntity.cgk_amount,
                nrq_Name = sourceEntity.cgk_name,
                OverriddenCreatedOn = sourceEntity.CreatedOn,
            };
            return result;
        }
        "#;

        let result = parse_cs_field_mappings(content).unwrap();
        assert_eq!(result.len(), 4); // Should skip commented lines and OverriddenCreatedOn
        assert_eq!(result.get("cgk_active"), Some(&"nrq_Active".to_string()));
        assert_eq!(result.get("cgk_fundid"), Some(&"nrq_FundId".to_string()));
        assert_eq!(result.get("cgk_amount"), Some(&"nrq_Amount".to_string()));
        assert_eq!(result.get("cgk_name"), Some(&"nrq_Name".to_string()));
        assert!(!result.contains_key("cgk_disabled"));
        assert!(!result.contains_key("cgk_skipped"));
        assert!(!result.contains_key("CreatedOn"));
    }

    #[test]
    fn test_real_requests_file() {
        // Test with a subset of the real Requests.cs pattern
        let content = r#"
        public class Requests : DataverseToDataverseMapping<cgk_request, nrq_Request>
        {
            protected override nrq_Request InternalMapping(cgk_request sourceRequest, MigrationOptions options)
            {
                var request = new nrq_Request(sourceRequest.Id)
                {
                    nrq_AccountId = sourceRequest.cgk_accountid,
                    //nrq_ActiveTransferRequest
                    nrq_Adaption = sourceRequest.vaf_adaptatie,
                    nrq_Amountofepisodes = sourceRequest.cgk_amountofepisodes,
                    nrq_CategoryId = CategoryXRef.GetTargetReference(sourceRequest.cgk_categoryid?.Id),
                    nrq_Commission = sourceRequest.cgk_commissiondecision,
                    nrq_ProjectId = sourceRequest.cgk_folderid?.Cast<nrq_Project>(),
                    nrq_Review = ConvertReview(sourceRequest.cgk_review),
                    nrq_SubmissionType = ConvertSubmissionType(sourceRequest.vaf_typeindiening),
                    OverriddenCreatedOn = sourceRequest.CreatedOn,
                };
                return request;
            }
        }
        "#;

        let result = parse_cs_field_mappings(content).unwrap();

        // Verify key mappings are extracted correctly
        assert!(result.len() >= 8, "Should have at least 8 mappings, got {}", result.len());
        assert_eq!(result.get("cgk_accountid"), Some(&"nrq_AccountId".to_string()));
        assert_eq!(result.get("vaf_adaptatie"), Some(&"nrq_Adaption".to_string()));
        assert_eq!(result.get("cgk_amountofepisodes"), Some(&"nrq_Amountofepisodes".to_string()));
        assert_eq!(result.get("cgk_categoryid"), Some(&"nrq_CategoryId".to_string()));
        assert_eq!(result.get("cgk_commissiondecision"), Some(&"nrq_Commission".to_string()));
        assert_eq!(result.get("cgk_folderid"), Some(&"nrq_ProjectId".to_string()));
        assert_eq!(result.get("cgk_review"), Some(&"nrq_Review".to_string()));
        assert_eq!(result.get("vaf_typeindiening"), Some(&"nrq_SubmissionType".to_string()));

        // Should not include commented or system fields
        assert!(!result.contains_key("CreatedOn"));
    }
}
