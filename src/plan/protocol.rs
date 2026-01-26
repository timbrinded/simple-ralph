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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseContext {
    #[serde(default)]
    pub codebase_summary: Option<CodebaseSummary>,

    #[serde(default)]
    pub requirements: Option<Vec<Requirement>>,

    #[serde(default)]
    pub quality_gates: Option<Vec<String>>,

    #[serde(default)]
    pub tasks: Option<Vec<Task>>,
}

/// Summary of the codebase structure
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

/// A requirement identified during planning
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
