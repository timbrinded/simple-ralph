use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

use super::phases::PlanPhase;
use super::protocol::{Answer, PhaseContext};

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Failed to read session file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse session file: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error(
        "Session file exists but --resume not specified. Use --resume to continue or --force to overwrite."
    )]
    SessionExists,
}

/// Persistent session state for multi-turn PRD generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSession {
    /// Unique session identifier (used with --session-id)
    pub id: String,

    /// Output file path for the final PRD
    pub output_path: String,

    /// Current phase (informational - Claude controls actual phase)
    pub last_phase: PlanPhase,

    /// Number of turns completed
    pub turn_count: u32,

    /// Accumulated context from all phases
    #[serde(default)]
    pub context: PhaseContext,

    /// All collected answers
    #[serde(default)]
    pub answers: Vec<Answer>,

    /// Session creation time
    pub created_at: DateTime<Utc>,

    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl PlanSession {
    /// Create a new session for the given output path
    pub fn new(output_path: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            output_path: output_path.to_string(),
            last_phase: PlanPhase::Exploring,
            turn_count: 0,
            context: PhaseContext::default(),
            answers: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the session file path for a given output path
    pub fn session_file_path(output_path: &str) -> PathBuf {
        let output = Path::new(output_path);
        let parent = output.parent().unwrap_or(Path::new("."));
        parent.join(".ralph-session.json")
    }

    /// Load an existing session or create a new one
    pub fn load_or_create(
        output_path: &str,
        resume: bool,
        force: bool,
    ) -> Result<Self, SessionError> {
        let session_path = Self::session_file_path(output_path);

        if session_path.exists() {
            if resume {
                // Load existing session
                let content = std::fs::read_to_string(&session_path)?;
                let session: PlanSession = serde_json::from_str(&content)?;
                Ok(session)
            } else if force {
                // Delete old session file before creating new to avoid Claude session ID conflicts
                let _ = std::fs::remove_file(&session_path);
                Ok(Self::new(output_path))
            } else {
                // Session exists but neither resume nor force specified
                Err(SessionError::SessionExists)
            }
        } else {
            // No existing session, create new
            Ok(Self::new(output_path))
        }
    }

    /// Save the session to disk
    pub fn save(&self) -> Result<(), SessionError> {
        let session_path = Self::session_file_path(&self.output_path);

        // Ensure parent directory exists
        if let Some(parent) = session_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&session_path, content)?;
        Ok(())
    }

    /// Update the session with a new phase and increment turn count
    pub fn advance(&mut self, phase: PlanPhase) {
        self.last_phase = phase;
        self.turn_count += 1;
        self.updated_at = Utc::now();
    }

    /// Add an answer to the session
    pub fn add_answer(&mut self, answer: Answer) {
        self.answers.push(answer);
        self.updated_at = Utc::now();
    }

    /// Merge context from a response
    pub fn merge_context(&mut self, context: PhaseContext) {
        // Merge codebase summary (replace if newer)
        if context.codebase_summary.is_some() {
            self.context.codebase_summary = context.codebase_summary;
        }

        // Merge requirements (append new ones)
        if let Some(reqs) = context.requirements {
            let existing = self.context.requirements.get_or_insert_with(Vec::new);
            existing.extend(reqs);
        }

        // Merge quality gates (replace if newer)
        if context.quality_gates.is_some() {
            self.context.quality_gates = context.quality_gates;
        }

        // Merge tasks (replace if newer)
        if context.tasks.is_some() {
            self.context.tasks = context.tasks;
        }

        self.updated_at = Utc::now();
    }

    /// Delete the session file
    pub fn cleanup(&self) -> Result<(), std::io::Error> {
        let session_path = Self::session_file_path(&self.output_path);
        if session_path.exists() {
            std::fs::remove_file(session_path)?;
        }
        Ok(())
    }

    /// Check if this is a fresh session (no turns yet)
    pub fn is_fresh(&self) -> bool {
        self.turn_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_session_has_uuid() {
        let session = PlanSession::new("/tmp/prd.json");
        assert!(!session.id.is_empty());
        // UUID v4 format check (basic)
        assert!(session.id.contains('-'));
        assert_eq!(session.id.len(), 36);
    }

    #[test]
    fn new_session_starts_fresh() {
        let session = PlanSession::new("/tmp/prd.json");
        assert!(session.is_fresh());
        assert_eq!(session.turn_count, 0);
        assert_eq!(session.last_phase, PlanPhase::Exploring);
        assert!(session.answers.is_empty());
    }

    #[test]
    fn new_session_stores_output_path() {
        let session = PlanSession::new("/custom/path/prd.json");
        assert_eq!(session.output_path, "/custom/path/prd.json");
    }

    #[test]
    fn session_file_path_calculation() {
        let path = PlanSession::session_file_path("/some/dir/prd.json");
        assert_eq!(path.to_str().unwrap(), "/some/dir/.ralph-session.json");
    }

    #[test]
    fn session_file_path_current_dir() {
        let path = PlanSession::session_file_path("prd.json");
        // When there's no parent dir, Path returns "" which becomes "." joined with filename
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            ".ralph-session.json"
        );
    }

    #[test]
    fn advance_increments_turn_and_updates_phase() {
        let mut session = PlanSession::new("/tmp/prd.json");
        assert_eq!(session.turn_count, 0);
        assert_eq!(session.last_phase, PlanPhase::Exploring);

        session.advance(PlanPhase::Asking);
        assert_eq!(session.turn_count, 1);
        assert_eq!(session.last_phase, PlanPhase::Asking);

        session.advance(PlanPhase::Working);
        assert_eq!(session.turn_count, 2);
        assert_eq!(session.last_phase, PlanPhase::Working);
    }

    #[test]
    fn add_answer_stores_answer() {
        let mut session = PlanSession::new("/tmp/prd.json");
        assert!(session.answers.is_empty());

        session.add_answer(Answer {
            question_id: "q1".to_string(),
            value: "React".to_string(),
        });
        assert_eq!(session.answers.len(), 1);
        assert_eq!(session.answers[0].question_id, "q1");

        session.add_answer(Answer {
            question_id: "q2".to_string(),
            value: "PostgreSQL".to_string(),
        });
        assert_eq!(session.answers.len(), 2);
    }

    #[test]
    fn merge_context_replaces_codebase_summary() {
        let mut session = PlanSession::new("/tmp/prd.json");
        assert!(session.context.codebase_summary.is_none());

        let context = PhaseContext {
            codebase_summary: Some(super::super::protocol::CodebaseSummary {
                languages: Some(vec!["Rust".to_string()]),
                frameworks: None,
                structure: None,
                key_files: None,
            }),
            ..Default::default()
        };
        session.merge_context(context);
        assert!(session.context.codebase_summary.is_some());
        assert_eq!(
            session.context.codebase_summary.as_ref().unwrap().languages,
            Some(vec!["Rust".to_string()])
        );
    }

    #[test]
    fn merge_context_appends_requirements() {
        let mut session = PlanSession::new("/tmp/prd.json");

        let context1 = PhaseContext {
            requirements: Some(vec![super::super::protocol::Requirement {
                category: "feature".to_string(),
                description: "Add auth".to_string(),
                priority: None,
            }]),
            ..Default::default()
        };
        session.merge_context(context1);
        assert_eq!(session.context.requirements.as_ref().unwrap().len(), 1);

        let context2 = PhaseContext {
            requirements: Some(vec![super::super::protocol::Requirement {
                category: "test".to_string(),
                description: "Add tests".to_string(),
                priority: None,
            }]),
            ..Default::default()
        };
        session.merge_context(context2);
        assert_eq!(session.context.requirements.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        let prd_path_str = prd_path.to_str().unwrap();

        let mut session = PlanSession::new(prd_path_str);
        session.advance(PlanPhase::Asking);
        session.add_answer(Answer {
            question_id: "q1".to_string(),
            value: "test value".to_string(),
        });

        session.save().unwrap();

        // Load it back
        let loaded = PlanSession::load_or_create(prd_path_str, true, false).unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.turn_count, 1);
        assert_eq!(loaded.last_phase, PlanPhase::Asking);
        assert_eq!(loaded.answers.len(), 1);
    }

    #[test]
    fn load_or_create_without_resume_or_force_errors() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        let prd_path_str = prd_path.to_str().unwrap();

        // Create and save a session
        let session = PlanSession::new(prd_path_str);
        session.save().unwrap();

        // Try to load without resume or force
        let result = PlanSession::load_or_create(prd_path_str, false, false);
        assert!(matches!(result, Err(SessionError::SessionExists)));
    }

    #[test]
    fn load_or_create_with_force_creates_new() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        let prd_path_str = prd_path.to_str().unwrap();

        // Create and save a session with some turns
        let mut session = PlanSession::new(prd_path_str);
        session.advance(PlanPhase::Asking);
        session.advance(PlanPhase::Working);
        let old_id = session.id.clone();
        session.save().unwrap();

        // Force create new session
        let new_session = PlanSession::load_or_create(prd_path_str, false, true).unwrap();
        assert_ne!(new_session.id, old_id);
        assert!(new_session.is_fresh());
        assert_eq!(new_session.turn_count, 0);
    }

    #[test]
    fn load_or_create_without_existing_creates_new() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        let prd_path_str = prd_path.to_str().unwrap();

        // No existing session file
        let session = PlanSession::load_or_create(prd_path_str, false, false).unwrap();
        assert!(session.is_fresh());
    }

    #[test]
    fn cleanup_removes_session_file() {
        let temp_dir = TempDir::new().unwrap();
        let prd_path = temp_dir.path().join("prd.json");
        let prd_path_str = prd_path.to_str().unwrap();
        let session_path = PlanSession::session_file_path(prd_path_str);

        let session = PlanSession::new(prd_path_str);
        session.save().unwrap();
        assert!(session_path.exists());

        session.cleanup().unwrap();
        assert!(!session_path.exists());
    }

    #[test]
    fn cleanup_handles_missing_file() {
        let session = PlanSession::new("/tmp/nonexistent/prd.json");
        // Should not error even if file doesn't exist
        let result = session.cleanup();
        assert!(result.is_ok());
    }

    #[test]
    fn is_fresh_returns_false_after_advance() {
        let mut session = PlanSession::new("/tmp/prd.json");
        assert!(session.is_fresh());

        session.advance(PlanPhase::Exploring);
        assert!(!session.is_fresh());
    }

    #[test]
    fn timestamps_are_set() {
        let session = PlanSession::new("/tmp/prd.json");
        assert!(session.created_at <= session.updated_at);
    }
}
