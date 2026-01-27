//! Integration tests for PRD file lifecycle

use std::fs;
use tempfile::TempDir;

/// Test complete PRD file lifecycle
#[test]
fn prd_lifecycle_create_load_verify() {
    let temp_dir = TempDir::new().unwrap();
    let prd_path = temp_dir.path().join("prd.json");

    // Create a valid PRD
    let prd_content = r#"{
        "name": "Integration Test PRD",
        "quality_gates": ["cargo test", "cargo clippy", "cargo fmt --check"],
        "tasks": [
            {
                "category": "feature",
                "description": "Add user authentication",
                "steps": ["Create login form", "Add JWT validation", "Implement logout"],
                "passes": false
            },
            {
                "category": "test",
                "description": "Add unit tests for auth",
                "steps": ["Test login", "Test JWT expiry"],
                "passes": false
            }
        ]
    }"#;

    fs::write(&prd_path, prd_content).unwrap();

    // Verify the file exists and is readable
    assert!(prd_path.exists());
    let read_content = fs::read_to_string(&prd_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&read_content).unwrap();

    assert_eq!(parsed["name"], "Integration Test PRD");
    assert_eq!(parsed["quality_gates"].as_array().unwrap().len(), 3);
    assert_eq!(parsed["tasks"].as_array().unwrap().len(), 2);
}

/// Test PRD with completed tasks
#[test]
fn prd_with_completed_json() {
    let temp_dir = TempDir::new().unwrap();
    let prd_path = temp_dir.path().join("prd.json");
    let completed_path = temp_dir.path().join("completed.json");

    // Create PRD with remaining tasks
    let prd_content = r#"{
        "name": "Test PRD",
        "quality_gates": ["cargo test"],
        "tasks": [
            {
                "category": "feature",
                "description": "Remaining task",
                "steps": ["Step 1"],
                "passes": false
            }
        ]
    }"#;

    // Create completed.json with finished tasks
    let completed_content = r#"[
        {
            "category": "setup",
            "description": "Project setup",
            "steps": ["Init repo", "Add deps"],
            "completed_at": "2024-01-15"
        },
        {
            "category": "feature",
            "description": "Basic structure",
            "steps": ["Create modules"],
            "completed_at": "2024-01-16"
        }
    ]"#;

    fs::write(&prd_path, prd_content).unwrap();
    fs::write(&completed_path, completed_content).unwrap();

    // Verify both files exist
    assert!(prd_path.exists());
    assert!(completed_path.exists());

    // Verify completed tasks
    let completed: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&completed_path).unwrap()).unwrap();
    assert_eq!(completed.as_array().unwrap().len(), 2);
    assert_eq!(completed[0]["completed_at"], "2024-01-15");
}

/// Test PRD validation - missing required fields
#[test]
fn prd_validation_missing_fields() {
    let invalid_prds = vec![
        // Missing name
        r#"{"quality_gates": [], "tasks": []}"#,
        // Missing quality_gates
        r#"{"name": "Test", "tasks": []}"#,
        // Missing tasks
        r#"{"name": "Test", "quality_gates": []}"#,
    ];

    for invalid in invalid_prds {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(invalid);
        // These parse as JSON but don't match our schema
        assert!(parsed.is_ok());
        let value = parsed.unwrap();
        // At least one required field should be missing
        let has_all = value.get("name").is_some()
            && value.get("quality_gates").is_some()
            && value.get("tasks").is_some();
        assert!(!has_all);
    }
}

/// Test task state progression
#[test]
fn task_passes_state_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let prd_path = temp_dir.path().join("prd.json");

    // Create PRD with tasks in different states
    let prd_content = r#"{
        "name": "State Test",
        "quality_gates": ["cargo test"],
        "tasks": [
            {"category": "a", "description": "Not started", "steps": ["1"], "passes": false},
            {"category": "b", "description": "In progress", "steps": ["1"], "passes": false},
            {"category": "c", "description": "Completed", "steps": ["1"], "passes": true}
        ]
    }"#;

    fs::write(&prd_path, prd_content).unwrap();

    let prd: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&prd_path).unwrap()).unwrap();
    let tasks = prd["tasks"].as_array().unwrap();

    // Count passes states
    let completed_count = tasks.iter().filter(|t| t["passes"] == true).count();
    let pending_count = tasks.iter().filter(|t| t["passes"] == false).count();

    assert_eq!(completed_count, 1);
    assert_eq!(pending_count, 2);
}
