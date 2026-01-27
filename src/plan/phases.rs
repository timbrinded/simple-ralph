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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_exploring() {
        assert_eq!(PlanPhase::Exploring.to_string(), "Exploring");
    }

    #[test]
    fn display_asking() {
        assert_eq!(PlanPhase::Asking.to_string(), "Asking");
    }

    #[test]
    fn display_working() {
        assert_eq!(PlanPhase::Working.to_string(), "Working");
    }

    #[test]
    fn display_complete() {
        assert_eq!(PlanPhase::Complete.to_string(), "Complete");
    }

    #[test]
    fn serde_roundtrip() {
        for phase in [
            PlanPhase::Exploring,
            PlanPhase::Asking,
            PlanPhase::Working,
            PlanPhase::Complete,
        ] {
            let json = serde_json::to_string(&phase).unwrap();
            let deserialized: PlanPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(phase, deserialized);
        }
    }

    #[test]
    fn serde_snake_case_serialization() {
        assert_eq!(
            serde_json::to_string(&PlanPhase::Exploring).unwrap(),
            "\"exploring\""
        );
        assert_eq!(
            serde_json::to_string(&PlanPhase::Asking).unwrap(),
            "\"asking\""
        );
        assert_eq!(
            serde_json::to_string(&PlanPhase::Working).unwrap(),
            "\"working\""
        );
        assert_eq!(
            serde_json::to_string(&PlanPhase::Complete).unwrap(),
            "\"complete\""
        );
    }

    #[test]
    fn serde_deserialization_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<PlanPhase>("\"exploring\"").unwrap(),
            PlanPhase::Exploring
        );
        assert_eq!(
            serde_json::from_str::<PlanPhase>("\"asking\"").unwrap(),
            PlanPhase::Asking
        );
        assert_eq!(
            serde_json::from_str::<PlanPhase>("\"working\"").unwrap(),
            PlanPhase::Working
        );
        assert_eq!(
            serde_json::from_str::<PlanPhase>("\"complete\"").unwrap(),
            PlanPhase::Complete
        );
    }

    #[test]
    fn equality_same_variants() {
        assert_eq!(PlanPhase::Exploring, PlanPhase::Exploring);
        assert_eq!(PlanPhase::Asking, PlanPhase::Asking);
    }

    #[test]
    fn inequality_different_variants() {
        assert_ne!(PlanPhase::Exploring, PlanPhase::Asking);
        assert_ne!(PlanPhase::Working, PlanPhase::Complete);
    }
}
