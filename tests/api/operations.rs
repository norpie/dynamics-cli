use anyhow::Result;
use dynamics_cli::api::{Operation, Operations, ClientManager};
use serde_json::json;

#[tokio::test]
async fn test_operation_api_design() -> Result<()> {
    // Create individual operations
    let create_op = Operation::create("contacts", json!({
        "firstname": "John",
        "lastname": "Doe",
        "emailaddress1": "john.doe@example.com"
    }));

    let update_op = Operation::update("accounts", "guid-123", json!({
        "name": "Updated Company Name"
    }));

    let delete_op = Operation::delete("leads", "guid-456");

    // Test operation properties
    assert_eq!(create_op.entity(), "contacts");
    assert_eq!(create_op.http_method(), "POST");
    assert_eq!(create_op.operation_type(), "create");

    assert_eq!(update_op.entity(), "accounts");
    assert_eq!(update_op.http_method(), "PATCH");
    assert_eq!(update_op.operation_type(), "update");

    assert_eq!(delete_op.entity(), "leads");
    assert_eq!(delete_op.http_method(), "DELETE");
    assert_eq!(delete_op.operation_type(), "delete");

    // Create Operations collection using builder pattern
    let ops = Operations::new()
        .create("contacts", json!({
            "firstname": "Jane",
            "lastname": "Smith"
        }))
        .update("accounts", "guid-789", json!({
            "website": "https://example.com"
        }))
        .delete("leads", "guid-999")
        .add(create_op); // Can also add existing operations

    assert_eq!(ops.len(), 4);
    assert!(!ops.is_empty());

    // Test conversions
    let single_ops = Operations::from(update_op);
    assert_eq!(single_ops.len(), 1);

    let vec_ops = Operations::from(vec![delete_op]);
    assert_eq!(vec_ops.len(), 1);

    // Test iteration
    let op_count = ops.operations().len();
    assert_eq!(op_count, 4);

    for operation in &ops {
        println!("Operation: {} on {}", operation.operation_type(), operation.entity());
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Requires real credentials
async fn test_operation_execution_placeholder() -> Result<()> {
    let mut manager = ClientManager::from_env()?;
    manager.authenticate().await?;

    // This currently returns placeholder results, but shows the API
    let create_op = Operation::create("contacts", json!({
        "firstname": "Test",
        "lastname": "Contact"
    }));

    // Individual execution would look like this:
    // let result = create_op.execute(&client).await?;
    // assert!(result.is_success());

    // Batch execution would look like this:
    let ops = Operations::new()
        .create("contacts", json!({"firstname": "John"}))
        .create("contacts", json!({"firstname": "Jane"}));

    // let results = ops.execute(&client).await?;
    // assert_eq!(results.len(), 2);

    Ok(())
}