use anyhow::Result;
use dynamics_cli::api::{Operation, Operations, ClientManager};
use serde_json::json;
use uuid::Uuid;

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
#[ignore] // Requires real credentials and WILL HIT THE CRM
async fn test_contact_crud_lifecycle() -> Result<()> {
    // Set up authenticated client
    let mut manager = ClientManager::from_env()?;
    manager.authenticate().await?;
    let client = manager.get_current_client()?;

    // Generate a unique contact for this test
    let contact_id = Uuid::new_v4();
    let test_email = format!("test-contact-{}@dynamics-cli-test.com", contact_id);

    println!("🧪 Starting CRUD lifecycle test for contact: {}", contact_id);

    // 1. CREATE: Create a new contact
    println!("📝 Step 1: Creating contact...");
    let create_result = Operation::create("contacts", json!({
        "firstname": "TestFirst",
        "lastname": "TestLast",
        "emailaddress1": test_email,
        "description": format!("Test contact created by dynamics-cli integration test: {}", contact_id)
    })).execute(&client).await?;

    assert!(create_result.is_success(), "Create operation failed: {:?}", create_result.error);

    // Extract the created contact ID
    let created_contact_guid = create_result.data
        .as_ref()
        .and_then(|d| d.get("contactid"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| anyhow::anyhow!("No contact ID returned from create operation"))?;

    println!("✅ Contact created with ID: {}", created_contact_guid);

    // 2. UPDATE: Update the contact's information
    println!("✏️  Step 2: Updating contact...");
    let update_result = Operation::update("contacts", created_contact_guid, json!({
        "firstname": "UpdatedFirst",
        "jobtitle": "Test Manager",
        "telephone1": "+1-555-123-4567"
    })).execute(&client).await?;

    assert!(update_result.is_success(), "Update operation failed: {:?}", update_result.error);
    println!("✅ Contact updated successfully");

    // 3. UPSERT: Skip for now since alternate keys may not be configured
    println!("⏭️  Step 3: Skipping upsert test (alternate keys not configured in this environment)");

    // 4. BATCH OPERATIONS: Perform multiple operations in a single batch
    println!("📦 Step 4: Testing batch operations...");
    let batch_ops = Operations::new()
        .update("contacts", created_contact_guid, json!({
            "description": format!("Updated via batch operation: {}", contact_id),
            "preferredcontactmethodcode": 1 // Email preferred
        }))
        .create("contacts", json!({
            "firstname": "BatchTest",
            "lastname": "Contact",
            "emailaddress1": format!("batch-test-{}@dynamics-cli-test.com", Uuid::new_v4()),
            "description": "Contact created via batch operation"
        }));

    let batch_results = batch_ops.execute(&client).await?;
    assert_eq!(batch_results.len(), 2, "Expected 2 batch operation results");

    for (i, result) in batch_results.iter().enumerate() {
        assert!(result.is_success(), "Batch operation {} failed: {:?}", i + 1, result.error);
    }

    // Extract the ID of the contact created in the batch
    let batch_created_contact_guid = batch_results[1].data
        .as_ref()
        .and_then(|d| d.get("contactid"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| anyhow::anyhow!("No contact ID returned from batch create operation"))?;

    println!("✅ Batch operations completed successfully");

    // 5. DELETE: Clean up - delete both test contacts
    println!("🗑️  Step 5: Cleaning up test contacts...");
    let cleanup_ops = Operations::new()
        .delete("contacts", created_contact_guid)
        .delete("contacts", batch_created_contact_guid);

    let cleanup_results = cleanup_ops.execute(&client).await?;

    for (i, result) in cleanup_results.iter().enumerate() {
        assert!(result.is_success(), "Cleanup operation {} failed: {:?}", i + 1, result.error);
    }

    println!("✅ Test contacts deleted successfully");

    // 6. PARALLEL EXECUTION TEST: Create and immediately delete multiple contacts
    println!("⚡ Step 6: Testing parallel execution...");
    let parallel_ops = Operations::new()
        .create("contacts", json!({
            "firstname": "Parallel1",
            "lastname": "Test",
            "emailaddress1": format!("parallel1-{}@dynamics-cli-test.com", Uuid::new_v4())
        }))
        .create("contacts", json!({
            "firstname": "Parallel2",
            "lastname": "Test",
            "emailaddress1": format!("parallel2-{}@dynamics-cli-test.com", Uuid::new_v4())
        }))
        .create("contacts", json!({
            "firstname": "Parallel3",
            "lastname": "Test",
            "emailaddress1": format!("parallel3-{}@dynamics-cli-test.com", Uuid::new_v4())
        }));

    let parallel_results = parallel_ops.execute_parallel(&client).await?;
    assert_eq!(parallel_results.len(), 3, "Expected 3 parallel operation results");

    // Extract contact IDs for cleanup
    let mut parallel_contact_ids = Vec::new();
    for result in &parallel_results {
        assert!(result.is_success(), "Parallel operation failed: {:?}", result.error);
        if let Some(contact_id) = result.data.as_ref()
            .and_then(|d| d.get("contactid"))
            .and_then(|id| id.as_str()) {
            parallel_contact_ids.push(contact_id);
        }
    }

    // Clean up parallel test contacts
    let parallel_cleanup = Operations::new()
        .delete("contacts", parallel_contact_ids[0])
        .delete("contacts", parallel_contact_ids[1])
        .delete("contacts", parallel_contact_ids[2]);

    let parallel_cleanup_results = parallel_cleanup.execute(&client).await?;
    for result in parallel_cleanup_results {
        assert!(result.is_success(), "Parallel cleanup failed: {:?}", result.error);
    }

    println!("✅ Parallel execution test completed successfully");

    println!("🎉 All CRUD lifecycle tests passed!");
    Ok(())
}