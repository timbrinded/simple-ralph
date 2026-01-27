use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CompletedTask {
    pub category: String,
    pub description: String,
    pub steps: Vec<String>,
    pub completed_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Task {
    pub category: String,
    pub description: String,
    pub steps: Vec<String>,
    pub passes: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Prd {
    pub name: String,
    pub quality_gates: Vec<String>,
    pub tasks: Vec<Task>,
}

pub fn load_completed_tasks_from_file(prd_path: &str) -> Option<Vec<CompletedTask>> {
    let prd_path = std::path::PathBuf::from(prd_path);

    let completed_path = prd_path.parent().unwrap().join("completed.json");

    if !completed_path.exists() {
        // println!("No completed.json file found at {:?}", completed_path);
        return None;
    }

    let file_content = std::fs::read_to_string(&completed_path)
        .unwrap_or_else(|_| panic!("Error reading completed.json at {:?}", completed_path));

    serde_json::from_str(&file_content).unwrap_or_else(|_| {
        panic!(
            "Invalid JSON formatting in completed.json at {:?}",
            completed_path
        )
    })
}

pub fn load_prd_from_file(prd_path: &str) -> Prd {
    let path = std::path::PathBuf::from(prd_path);

    if !path.exists() {
        panic!("PRD file not found at path {}", prd_path);
    }

    let file_content = std::fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Error reading PRD.json at {}", prd_path));
    serde_json::from_str(&file_content)
        .unwrap_or_else(|_| panic!("Invalid JSON formatting in prd {}", prd_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_prd_json() -> &'static str {
        r#"{
            "name": "Test PRD",
            "quality_gates": ["cargo test", "cargo clippy"],
            "tasks": [
                {
                    "category": "feature",
                    "description": "Add login",
                    "steps": ["Create form", "Add validation"],
                    "passes": false
                },
                {
                    "category": "test",
                    "description": "Add tests",
                    "steps": ["Unit tests"],
                    "passes": true
                }
            ]
        }"#
    }

    fn create_test_completed_json() -> &'static str {
        r#"[
            {
                "category": "setup",
                "description": "Initial setup",
                "steps": ["Create project"],
                "completed_at": "2024-01-15"
            }
        ]"#
    }

    #[test]
    fn load_prd_from_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        fs::write(&prd_path, create_test_prd_json()).unwrap();

        let prd = load_prd_from_file(prd_path.to_str().unwrap());
        assert_eq!(prd.name, "Test PRD");
        assert_eq!(prd.quality_gates.len(), 2);
        assert_eq!(prd.tasks.len(), 2);
        assert!(!prd.tasks[0].passes);
        assert!(prd.tasks[1].passes);
    }

    #[test]
    #[should_panic(expected = "PRD file not found")]
    fn load_prd_nonexistent_file_panics() {
        load_prd_from_file("/nonexistent/path/prd.json");
    }

    #[test]
    #[should_panic(expected = "Invalid JSON formatting")]
    fn load_prd_invalid_json_panics() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        fs::write(&prd_path, "not valid json {{{").unwrap();

        load_prd_from_file(prd_path.to_str().unwrap());
    }

    #[test]
    #[should_panic(expected = "Invalid JSON formatting")]
    fn load_prd_wrong_schema_panics() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        fs::write(&prd_path, r#"{"wrong": "schema"}"#).unwrap();

        load_prd_from_file(prd_path.to_str().unwrap());
    }

    #[test]
    fn load_completed_tasks_returns_none_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        fs::write(&prd_path, create_test_prd_json()).unwrap();

        let result = load_completed_tasks_from_file(prd_path.to_str().unwrap());
        assert!(result.is_none());
    }

    #[test]
    fn load_completed_tasks_returns_tasks_when_exists() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        let completed_path = temp_dir.path().join("completed.json");

        fs::write(&prd_path, create_test_prd_json()).unwrap();
        fs::write(&completed_path, create_test_completed_json()).unwrap();

        let result = load_completed_tasks_from_file(prd_path.to_str().unwrap());
        assert!(result.is_some());
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].category, "setup");
        assert_eq!(tasks[0].completed_at, "2024-01-15");
    }

    #[test]
    #[should_panic(expected = "Invalid JSON formatting")]
    fn load_completed_tasks_invalid_json_panics() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        let completed_path = temp_dir.path().join("completed.json");

        fs::write(&prd_path, create_test_prd_json()).unwrap();
        fs::write(&completed_path, "invalid json").unwrap();

        load_completed_tasks_from_file(prd_path.to_str().unwrap());
    }

    #[test]
    fn prd_task_fields() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        fs::write(&prd_path, create_test_prd_json()).unwrap();

        let prd = load_prd_from_file(prd_path.to_str().unwrap());
        let task = &prd.tasks[0];
        assert_eq!(task.category, "feature");
        assert_eq!(task.description, "Add login");
        assert_eq!(task.steps, vec!["Create form", "Add validation"]);
    }
}
