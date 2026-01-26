use serde::{Deserialize, Serialize};

/// Phases are hints for TUI rendering, not a strict state machine.
/// Claude decides which phase is appropriate based on context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanPhase {
    /// Reading/understanding codebase
    Exploring,
    /// Needs user input
    Asking,
    /// Generating requirements/tasks
    Working,
    /// PRD ready
    Complete,
}

impl std::fmt::Display for PlanPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanPhase::Exploring => write!(f, "Exploring"),
            PlanPhase::Asking => write!(f, "Asking"),
            PlanPhase::Working => write!(f, "Working"),
            PlanPhase::Complete => write!(f, "Complete"),
        }
    }
}
