use serde::{Deserialize, Serialize};

use super::phases::PlanPhase;

/// The single schema used for ALL Claude responses during plan mode.
/// The `phase` field tells ralph what to render.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlanResponse {
    /// Current workflow phase - ralph uses this to determine TUI state
    pub phase: PlanPhase,

    /// Status message to display (optional)
    #[serde(default)]
    pub status: Option<String>,

    /// Questions for the user (when phase requires input)
    #[serde(default)]
    pub questions: Option<Vec<Question>>,

    /// Accumulated context/findings from Claude's work
    #[serde(default)]
    pub context: Option<PhaseContext>,

    /// The final PRD (only present when phase == Complete)
    #[serde(default)]
    pub prd: Option<FinalPrd>,
}

/// A question for the user with optional multiple-choice options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// Unique identifier for this question
    pub id: String,

    /// Category: "scope", "technical", "quality", etc.
    pub category: String,

    /// The question text itself
    pub text: String,

    /// Why this question matters (context for the user)
    #[serde(default)]
    pub context: Option<String>,

    /// A/B/C/D style options (if any)
    #[serde(default)]
    pub options: Option<Vec<QuestionOption>>,

    /// Can user type a custom answer?
    #[serde(default)]
    pub allow_freeform: bool,
}

/// A selectable option for a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// "A", "B", "C", etc.
    pub key: String,

    /// Short label
    pub label: String,

    /// Longer explanation
    #[serde(default)]
    pub description: Option<String>,
}

/// Context accumulated during exploration/working phases
/// Uses serde_json::Value for flexible fields since Claude may return
/// arbitrary structures. This is intermediate state - only the `prd` matters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseContext {
    /// Codebase analysis - accepts any structure Claude provides
    #[serde(default)]
    pub codebase_summary: Option<serde_json::Value>,

    /// Requirements - accepts any structure (Vec, object, etc.)
    #[serde(default)]
    pub requirements: Option<serde_json::Value>,

    #[serde(default)]
    pub quality_gates: Option<Vec<String>>,

    #[serde(default)]
    pub tasks: Option<Vec<Task>>,

    /// Claude sometimes includes findings as a string
    #[serde(default)]
    pub findings: Option<String>,
}

/// Summary of the codebase structure (ideal format - used for testing/documentation)
/// PhaseContext uses serde_json::Value for flexibility, but this documents the expected shape.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseSummary {
    #[serde(default)]
    pub languages: Option<Vec<String>>,

    #[serde(default)]
    pub frameworks: Option<Vec<String>>,

    #[serde(default)]
    pub structure: Option<String>,

    #[serde(default)]
    pub key_files: Option<Vec<String>>,
}

/// A requirement identified during planning (ideal format - used for testing/documentation)
/// PhaseContext uses serde_json::Value for flexibility, but this documents the expected shape.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub priority: Option<String>,
}

/// A task in the final PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub category: String,
    pub description: String,
    pub steps: Vec<String>,
    #[serde(default)]
    pub passes: bool,
}

/// The final PRD output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalPrd {
    pub name: String,
    pub quality_gates: Vec<String>,
    pub tasks: Vec<Task>,
}

/// An answer to a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub question_id: String,
    pub value: String,
}

/// JSON schema string for --json-schema flag
pub const PLAN_RESPONSE_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["phase"],
  "properties": {
    "phase": {
      "type": "string",
      "enum": ["exploring", "asking", "working", "complete"]
    },
    "status": { "type": "string" },
    "questions": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["id", "category", "text", "allow_freeform"],
        "properties": {
          "id": { "type": "string" },
          "category": { "type": "string" },
          "text": { "type": "string" },
          "context": { "type": "string" },
          "options": {
            "type": "array",
            "items": {
              "type": "object",
              "required": ["key", "label"],
              "properties": {
                "key": { "type": "string" },
                "label": { "type": "string" },
                "description": { "type": "string" }
              }
            }
          },
          "allow_freeform": { "type": "boolean" }
        }
      }
    },
    "context": { "type": "object" },
    "prd": {
      "type": "object",
      "required": ["name", "quality_gates", "tasks"],
      "properties": {
        "name": { "type": "string" },
        "quality_gates": { "type": "array", "items": { "type": "string" } },
        "tasks": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["category", "description", "steps"],
            "properties": {
              "category": { "type": "string" },
              "description": { "type": "string" },
              "steps": { "type": "array", "items": { "type": "string" } },
              "passes": { "type": "boolean" }
            }
          }
        }
      }
    }
  }
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_exploring_response() {
        let json = r#"{"phase": "exploring", "status": "Reading files..."}"#;
        let response: PlanResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.phase, PlanPhase::Exploring);
        assert_eq!(response.status, Some("Reading files...".to_string()));
        assert!(response.questions.is_none());
        assert!(response.prd.is_none());
    }

    #[test]
    fn parse_asking_response_with_questions() {
        let json = r#"{
            "phase": "asking",
            "status": "Need clarification",
            "questions": [{
                "id": "q1",
                "category": "scope",
                "text": "What framework?",
                "allow_freeform": true,
                "options": [
                    {"key": "A", "label": "React"},
                    {"key": "B", "label": "Vue", "description": "Progressive framework"}
                ]
            }]
        }"#;
        let response: PlanResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.phase, PlanPhase::Asking);
        let questions = response.questions.unwrap();
        assert_eq!(questions.len(), 1);
        assert_eq!(questions[0].id, "q1");
        assert_eq!(questions[0].category, "scope");
        assert!(questions[0].allow_freeform);
        let opts = questions[0].options.as_ref().unwrap();
        assert_eq!(opts.len(), 2);
        assert_eq!(opts[0].key, "A");
        assert_eq!(
            opts[1].description,
            Some("Progressive framework".to_string())
        );
    }

    #[test]
    fn parse_complete_response_with_prd() {
        let json = r#"{
            "phase": "complete",
            "prd": {
                "name": "Test PRD",
                "quality_gates": ["cargo test", "cargo clippy"],
                "tasks": [{
                    "category": "feature",
                    "description": "Add login",
                    "steps": ["Create form", "Add validation"],
                    "passes": false
                }]
            }
        }"#;
        let response: PlanResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.phase, PlanPhase::Complete);
        let prd = response.prd.unwrap();
        assert_eq!(prd.name, "Test PRD");
        assert_eq!(prd.quality_gates.len(), 2);
        assert_eq!(prd.tasks.len(), 1);
        assert!(!prd.tasks[0].passes);
    }

    #[test]
    fn parse_minimal_response_phase_only() {
        let json = r#"{"phase": "working"}"#;
        let response: PlanResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.phase, PlanPhase::Working);
        assert!(response.status.is_none());
        assert!(response.questions.is_none());
        assert!(response.context.is_none());
        assert!(response.prd.is_none());
    }

    #[test]
    fn question_serialization_roundtrip() {
        let question = Question {
            id: "q1".to_string(),
            category: "technical".to_string(),
            text: "Which database?".to_string(),
            context: Some("Important for scalability".to_string()),
            options: Some(vec![QuestionOption {
                key: "A".to_string(),
                label: "PostgreSQL".to_string(),
                description: None,
            }]),
            allow_freeform: false,
        };
        let json = serde_json::to_string(&question).unwrap();
        let deserialized: Question = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, question.id);
        assert_eq!(deserialized.context, question.context);
    }

    #[test]
    fn question_option_without_description() {
        let json = r#"{"key": "A", "label": "Option A"}"#;
        let opt: QuestionOption = serde_json::from_str(json).unwrap();
        assert_eq!(opt.key, "A");
        assert_eq!(opt.label, "Option A");
        assert!(opt.description.is_none());
    }

    #[test]
    fn phase_context_defaults() {
        let context = PhaseContext::default();
        assert!(context.codebase_summary.is_none());
        assert!(context.requirements.is_none());
        assert!(context.quality_gates.is_none());
        assert!(context.tasks.is_none());
    }

    #[test]
    fn phase_context_from_empty_json() {
        let json = r#"{}"#;
        let context: PhaseContext = serde_json::from_str(json).unwrap();
        assert!(context.codebase_summary.is_none());
        assert!(context.requirements.is_none());
    }

    #[test]
    fn plan_response_schema_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(PLAN_RESPONSE_SCHEMA).unwrap();
        assert_eq!(parsed["type"], "object");
        assert!(
            parsed["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("phase"))
        );
    }

    #[test]
    fn answer_serialization() {
        let answer = Answer {
            question_id: "q1".to_string(),
            value: "Option A".to_string(),
        };
        let json = serde_json::to_string(&answer).unwrap();
        assert!(json.contains("q1"));
        assert!(json.contains("Option A"));
    }

    #[test]
    fn malformed_json_missing_phase_fails() {
        let json = r#"{"status": "test"}"#;
        let result: Result<PlanResponse, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_phase_value_fails() {
        let json = r#"{"phase": "invalid_phase"}"#;
        let result: Result<PlanResponse, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn codebase_summary_partial_fields() {
        let json = r#"{"languages": ["Rust", "Python"]}"#;
        let summary: CodebaseSummary = serde_json::from_str(json).unwrap();
        assert_eq!(
            summary.languages,
            Some(vec!["Rust".to_string(), "Python".to_string()])
        );
        assert!(summary.frameworks.is_none());
        assert!(summary.structure.is_none());
    }

    #[test]
    fn requirement_with_optional_priority() {
        let json = r#"{"category": "feature", "description": "Add auth"}"#;
        let req: Requirement = serde_json::from_str(json).unwrap();
        assert!(req.priority.is_none());

        let json_with_priority =
            r#"{"category": "feature", "description": "Add auth", "priority": "high"}"#;
        let req_with_priority: Requirement = serde_json::from_str(json_with_priority).unwrap();
        assert_eq!(req_with_priority.priority, Some("high".to_string()));
    }

    #[test]
    fn task_passes_defaults_to_false() {
        let json = r#"{"category": "test", "description": "Add tests", "steps": ["step1"]}"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert!(!task.passes);
    }

    #[test]
    fn phase_context_accepts_arbitrary_requirements_object() {
        // Claude sometimes returns requirements as an object instead of array
        let json = r#"{
            "requirements": {"region": "us-east", "framework": "axum"},
            "codebase_summary": {"languages": ["Rust"], "extra_field": true}
        }"#;
        let context: PhaseContext = serde_json::from_str(json).unwrap();
        assert!(context.requirements.is_some());
        assert!(context.codebase_summary.is_some());
    }

    #[test]
    fn complete_response_with_arbitrary_context_parses() {
        // The actual error case: context has object requirements, but prd is valid
        let json = r#"{
            "phase": "complete",
            "context": {
                "requirements": {"region": "us-east", "framework": "axum"},
                "findings": "Found existing auth module"
            },
            "prd": {
                "name": "Test PRD",
                "quality_gates": ["cargo test"],
                "tasks": [{
                    "category": "feature",
                    "description": "Add feature",
                    "steps": ["Step 1"]
                }]
            }
        }"#;
        let response: PlanResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.phase, PlanPhase::Complete);
        // The prd should be parsed correctly despite arbitrary context
        let prd = response.prd.unwrap();
        assert_eq!(prd.name, "Test PRD");
        // Context is present and parsed leniently
        let ctx = response.context.unwrap();
        assert!(ctx.requirements.is_some());
        assert_eq!(ctx.findings, Some("Found existing auth module".to_string()));
    }
}
