//! Integration tests for OData query system
//!
//! Tests the complete query building and execution pipeline

use dynamics_cli::api::*;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

/// Test basic query building and URL generation
#[test]
fn test_query_building() {
    let query = QueryBuilder::new("contacts")
        .select(&["firstname", "lastname", "emailaddress1"])
        .filter(Filter::eq("statecode", 0))
        .orderby(OrderBy::desc("createdon"))
        .top(10)
        .build();

    let url = query.to_url("https://test.crm.dynamics.com");

    // Check that URL contains expected components
    assert!(url.contains("contacts"));
    assert!(url.contains("$select=firstname,lastname,emailaddress1"));
    assert!(url.contains("$filter=statecode%20eq%200"));
    assert!(url.contains("$orderby=createdon%20desc"));
    assert!(url.contains("$top=10"));
}

/// Test complex filter building
#[test]
fn test_complex_filters() {
    let complex_filter = Filter::and(vec![
        Filter::eq("statecode", 0),
        Filter::or(vec![
            Filter::contains("firstname", "John"),
            Filter::starts_with("lastname", "Smith")
        ]),
        Filter::not(Filter::eq("emailaddress1", FilterValue::Null))
    ]);

    let query = QueryBuilder::new("contacts")
        .filter(complex_filter)
        .build();

    let params = query.to_query_params();
    let filter_param = params.get("$filter").unwrap();

    assert!(filter_param.contains("statecode eq 0"));
    assert!(filter_param.contains("contains(firstname, 'John')"));
    assert!(filter_param.contains("startswith(lastname, 'Smith')"));
    assert!(filter_param.contains("not (emailaddress1 eq null)"));
}

/// Test query reusability
#[test]
fn test_query_reusability() {
    let base_query = QueryBuilder::new("contacts")
        .select(&["firstname", "lastname"])
        .filter(Filter::eq("statecode", 0))
        .build();

    let recent_query = base_query.clone().with_top(10);
    let limited_query = base_query.clone().with_top(5);  // Avoid skip since it's not supported

    assert_eq!(recent_query.top, Some(10));
    assert_eq!(limited_query.top, Some(5));

    // Base query should be unchanged
    assert_eq!(base_query.top, None);
}

/// Test convenience methods
#[test]
fn test_convenience_methods() {
    let query = QueryBuilder::new("contacts")
        .active_only()
        .newest_first()
        .top(25)
        .build();

    // Should have active filter
    assert!(query.filter.is_some());

    // Should have top limit
    assert_eq!(query.top, Some(25));

    // Should have ordering
    assert!(query.orderby.to_odata_string().is_some());
}

/// Test expand functionality
#[test]
fn test_expand_queries() {
    let query = QueryBuilder::new("contacts")
        .select(&["firstname", "lastname"])
        .expand(&[
            "parentcontactid($select=fullname)",
            "account($select=name,websiteurl)"
        ])
        .build();

    let params = query.to_query_params();
    let expand_param = params.get("$expand").unwrap();

    assert!(expand_param.contains("parentcontactid($select=fullname)"));
    assert!(expand_param.contains("account($select=name,websiteurl)"));
}

/// Test filter value types
#[test]
fn test_filter_value_types() {
    // String filter
    let string_filter = Filter::eq("firstname", "John");
    assert_eq!(string_filter.to_odata_string(), "firstname eq 'John'");

    // Number filter
    let number_filter = Filter::gt("revenue", 100000.50);
    assert_eq!(number_filter.to_odata_string(), "revenue gt 100000.5");

    // Integer filter
    let int_filter = Filter::le("employees", 500);
    assert_eq!(int_filter.to_odata_string(), "employees le 500");

    // Boolean filter
    let bool_filter = Filter::eq("isprivate", true);
    assert_eq!(bool_filter.to_odata_string(), "isprivate eq true");

    // Null filter
    let null_filter = Filter::eq("description", FilterValue::Null);
    assert_eq!(null_filter.to_odata_string(), "description eq null");
}

/// Test quote escaping in filters
#[test]
fn test_quote_escaping() {
    let filter = Filter::contains("firstname", "O'Connor");
    assert_eq!(filter.to_odata_string(), "contains(firstname, 'O''Connor')");

    let complex_filter = Filter::starts_with("company", "Smith's \"Best\" Corp");
    assert_eq!(complex_filter.to_odata_string(), "startswith(company, 'Smith''s \"Best\" Corp')");
}

/// Test multiple ordering
#[test]
fn test_multiple_ordering() {
    let query = QueryBuilder::new("contacts")
        .orderby(OrderBy::asc("lastname"))
        .orderby(OrderBy::desc("createdon"))
        .orderby(OrderBy::asc("firstname"))
        .build();

    let orderby_string = query.orderby.to_odata_string().unwrap();
    assert_eq!(orderby_string, "lastname asc, createdon desc, firstname asc");
}

/// Integration test with real Dynamics 365 (requires credentials)
#[tokio::test]
#[ignore]
async fn test_real_query_execution() {
    // Setup client manager from environment
    let mut manager = ClientManager::from_env()
        .expect("Failed to create ClientManager from environment variables");

    manager.authenticate().await
        .expect("Failed to authenticate");

    let client = manager.get_client(".env")
        .expect("Failed to get authenticated client");

    // Test simple query
    println!("🔍 Testing simple contact query...");
    let simple_result = QueryBuilder::new("contacts")
        .select(&["firstname", "lastname", "createdon"])
        .active_only()
        .top(5)
        .execute(&client)
        .await;

    match simple_result {
        Ok(result) => {
            assert!(result.is_success());
            println!("✅ Simple query succeeded, got {} contacts", result.len());

            if let Some(records) = result.records() {
                for (i, record) in records.iter().enumerate() {
                    println!("  {}. {}", i + 1, record.get("firstname").unwrap_or(&json!("N/A")));
                }
            }
        },
        Err(e) => panic!("Simple query failed: {}", e),
    }

    // Test complex filter query
    println!("🔍 Testing complex filter query...");
    let complex_result = QueryBuilder::new("contacts")
        .select(&["firstname", "lastname", "emailaddress1"])
        .filter(Filter::and(vec![
            Filter::eq("statecode", 0),
            Filter::not(Filter::eq("firstname", FilterValue::Null))
        ]))
        .orderby(OrderBy::desc("createdon"))
        .top(3)
        .execute(&client)
        .await;

    match complex_result {
        Ok(result) => {
            assert!(result.is_success());
            println!("✅ Complex query succeeded, got {} contacts", result.len());
        },
        Err(e) => panic!("Complex query failed: {}", e),
    }

    // Test reusable query pattern
    println!("🔍 Testing reusable query pattern...");
    let base_query = QueryBuilder::new("contacts")
        .select(&["firstname", "lastname"])
        .active_only()
        .build();

    // First variation - recent contacts
    let recent_query = base_query.clone()
        .with_filter(Filter::and(vec![
            base_query.filter.clone().unwrap(),
            Filter::gt("createdon", "2020-01-01")
        ]))
        .with_top(3);

    let recent_result = client.execute_query(&recent_query).await;
    match recent_result {
        Ok(result) => {
            assert!(result.is_success());
            println!("✅ Recent contacts query succeeded, got {} contacts", result.len());
        },
        Err(e) => panic!("Recent contacts query failed: {}", e),
    }

    // Second variation - limited results (avoiding $skip which isn't supported in Dynamics 365)
    let limited_query = base_query.with_top(2);
    let limited_result = client.execute_query(&limited_query).await;
    match limited_result {
        Ok(result) => {
            assert!(result.is_success());
            println!("✅ Limited query succeeded, got {} contacts", result.len());
        },
        Err(e) => panic!("Limited query failed: {}", e),
    }

    // Test proper pagination pattern
    println!("🔍 Testing proper pagination pattern...");
    let paginated_query = QueryBuilder::new("contacts")
        .select(&["firstname", "lastname"])
        .active_only()
        .top(2)  // Small page size to likely get next link
        .build();

    let first_page = client.execute_query(&paginated_query).await;
    match first_page {
        Ok(result) => {
            assert!(result.is_success());
            println!("✅ First page query succeeded, got {} contacts", result.len());

            if result.has_more() {
                println!("🔗 Found next link, testing pagination...");
                if let Ok(Some(second_page)) = result.next_page(&client).await {
                    println!("✅ Second page query succeeded, got {} contacts", second_page.len());
                } else {
                    println!("⚠️ Could not fetch second page");
                }
            } else {
                println!("ℹ️ No more pages available (total results <= page size)");
            }
        },
        Err(e) => panic!("Paginated query failed: {}", e),
    }

    println!("🎉 All query tests passed!");
}

/// Test error handling for malformed queries
#[tokio::test]
#[ignore]
async fn test_query_error_handling() {
    let mut manager = ClientManager::from_env()
        .expect("Failed to create ClientManager from environment variables");

    manager.authenticate().await
        .expect("Failed to authenticate");

    let client = manager.get_client(".env")
        .expect("Failed to get authenticated client");

    // Test query with invalid entity
    println!("🔍 Testing invalid entity query...");
    let invalid_result = QueryBuilder::new("nonexistententity")
        .select(&["field1", "field2"])
        .execute(&client)
        .await;

    match invalid_result {
        Ok(result) => {
            assert!(result.is_error());
            println!("✅ Invalid entity correctly returned error: {:?}", result.error);
        },
        Err(e) => println!("⚠️ Invalid entity query failed at HTTP level: {}", e),
    }

    // Test query with invalid field
    println!("🔍 Testing invalid field query...");
    let invalid_field_result = QueryBuilder::new("contacts")
        .select(&["nonexistentfield"])
        .top(1)
        .execute(&client)
        .await;

    match invalid_field_result {
        Ok(result) => {
            if result.is_error() {
                println!("✅ Invalid field correctly returned error: {:?}", result.error);
            } else {
                println!("⚠️ Invalid field query unexpectedly succeeded");
            }
        },
        Err(e) => println!("⚠️ Invalid field query failed at HTTP level: {}", e),
    }
}

/// Performance test for query building
#[test]
fn test_query_building_performance() {
    use std::time::Instant;

    let start = Instant::now();

    // Build 1000 complex queries
    for i in 0..1000 {
        let _query = QueryBuilder::new("contacts")
            .select(&["firstname", "lastname", "emailaddress1"])
            .filter(Filter::and(vec![
                Filter::eq("statecode", 0),
                Filter::contains("firstname", &format!("Test{}", i)),
                Filter::or(vec![
                    Filter::gt("createdon", "2020-01-01"),
                    Filter::lt("createdon", "2025-01-01")
                ])
            ]))
            .orderby(OrderBy::desc("createdon"))
            .top(10)
            .build();
    }

    let duration = start.elapsed();
    println!("⚡ Built 1000 complex queries in {:?}", duration);

    // Should be very fast (under 100ms)
    assert!(duration < Duration::from_millis(100));
}