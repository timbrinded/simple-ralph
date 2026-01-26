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
