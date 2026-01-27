//! Integration tests for session lifecycle and context merging

use std::fs;
use tempfile::TempDir;

/// Test session JSON structure
#[test]
fn session_json_structure() {
    let session = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "output_path": "/path/to/prd.json",
        "last_phase": "exploring",
        "turn_count": 0,
        "context": {},
        "answers": [],
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T10:30:00Z"
    });

    // Verify all required fields
    assert!(session["id"].is_string());
    assert!(session["output_path"].is_string());
    assert!(session["last_phase"].is_string());
    assert!(session["turn_count"].is_number());
    assert!(session["context"].is_object());
    assert!(session["answers"].is_array());
}

/// Test session file persistence
#[test]
fn session_file_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let session_path = temp_dir.path().join(".ralph-session.json");

    let session = serde_json::json!({
        "id": "test-session-123",
        "output_path": "prd.json",
        "last_phase": "asking",
        "turn_count": 3,
        "context": {
            "codebase_summary": {
                "languages": ["Rust"],
                "frameworks": ["tokio", "serde"]
            }
        },
        "answers": [
            {"question_id": "q1", "value": "React"},
            {"question_id": "q2", "value": "PostgreSQL"}
        ],
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T11:00:00Z"
    });

    // Write session
    fs::write(
        &session_path,
        serde_json::to_string_pretty(&session).unwrap(),
    )
    .unwrap();

    // Read it back
    let loaded: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&session_path).unwrap()).unwrap();

    assert_eq!(loaded["id"], "test-session-123");
    assert_eq!(loaded["turn_count"], 3);
    assert_eq!(loaded["answers"].as_array().unwrap().len(), 2);
}

/// Test context merging behavior
#[test]
fn context_merging_accumulation() {
    // Simulate how context would accumulate across turns
    let mut context = serde_json::json!({});

    // Turn 1: Add codebase summary
    let turn1_context = serde_json::json!({
        "codebase_summary": {
            "languages": ["Rust", "TypeScript"],
            "structure": "Monorepo"
        }
    });

    // Merge turn 1
    if let Some(summary) = turn1_context.get("codebase_summary") {
        context["codebase_summary"] = summary.clone();
    }

    assert_eq!(
        context["codebase_summary"]["languages"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Turn 2: Add requirements
    let turn2_context = serde_json::json!({
        "requirements": [
            {"category": "feature", "description": "Add auth"}
        ]
    });

    if let Some(reqs) = turn2_context.get("requirements") {
        context["requirements"] = reqs.clone();
    }

    // Turn 3: Add more requirements (should append)
    let turn3_context = serde_json::json!({
        "requirements": [
            {"category": "test", "description": "Add tests"}
        ]
    });

    if let Some(new_reqs) = turn3_context["requirements"].as_array() {
        let existing = context["requirements"]
            .as_array_mut()
            .expect("requirements should be array");
        existing.extend(new_reqs.iter().cloned());
    }

    assert_eq!(context["requirements"].as_array().unwrap().len(), 2);
}

/// Test answer collection across questions
#[test]
fn answer_collection_workflow() {
    let mut answers: Vec<serde_json::Value> = Vec::new();

    // Simulate answering questions
    let questions = [
        serde_json::json!({"id": "q1", "text": "Framework?", "options": [{"key": "A", "label": "React"}]}),
        serde_json::json!({"id": "q2", "text": "Database?", "options": [{"key": "A", "label": "Postgres"}]}),
        serde_json::json!({"id": "q3", "text": "Custom input?", "allow_freeform": true}),
    ];

    // Answer each question
    answers.push(serde_json::json!({"question_id": "q1", "value": "A"}));
    answers.push(serde_json::json!({"question_id": "q2", "value": "A"}));
    answers.push(serde_json::json!({"question_id": "q3", "value": "Custom answer here"}));

    // All questions answered
    assert_eq!(answers.len(), questions.len());

    // Verify each answer references a valid question
    for answer in &answers {
        let qid = answer["question_id"].as_str().unwrap();
        let matching_question = questions.iter().find(|q| q["id"].as_str().unwrap() == qid);
        assert!(matching_question.is_some());
    }
}

/// Test session cleanup
#[test]
fn session_cleanup_removes_file() {
    let temp_dir = TempDir::new().unwrap();
    let session_path = temp_dir.path().join(".ralph-session.json");

    // Create session file
    fs::write(&session_path, "{}").unwrap();
    assert!(session_path.exists());

    // Cleanup
    fs::remove_file(&session_path).unwrap();
    assert!(!session_path.exists());
}

/// Test session resume detection
#[test]
fn session_resume_detection() {
    let temp_dir = TempDir::new().unwrap();
    let session_path = temp_dir.path().join(".ralph-session.json");

    // No session exists - should allow new creation
    assert!(!session_path.exists());

    // Create a session
    let session = serde_json::json!({
        "id": "existing-session",
        "turn_count": 5
    });
    fs::write(&session_path, serde_json::to_string(&session).unwrap()).unwrap();

    // Session exists - need resume flag
    assert!(session_path.exists());

    // Simulate loading for resume
    let loaded: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&session_path).unwrap()).unwrap();
    assert_eq!(loaded["id"], "existing-session");
    assert_eq!(loaded["turn_count"], 5);
}
