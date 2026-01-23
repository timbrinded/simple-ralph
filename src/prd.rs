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

    let file_content = std::fs::read_to_string(&completed_path).expect(&format!(
        "Error reading completed.json at {:?}",
        completed_path
    ));

    serde_json::from_str(&file_content).expect(&format!(
        "Invalid JSON formatting in completed.json at {:?}",
        completed_path
    ))
}

pub fn load_prd_from_file(prd_path: &str) -> Prd {
    let path = std::path::PathBuf::from(prd_path);

    if !path.exists() {
        panic!("PRD file not found at path {}", prd_path);
    }

    let file_content =
        std::fs::read_to_string(path).expect(&format!("Error reading PRD.json at {}", prd_path));
    serde_json::from_str(&file_content)
        .expect(&format!("Invalid JSON formatting in prd {}", prd_path))
}
