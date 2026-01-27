use super::protocol::Answer;

/// System prompt that instructs Claude on how to generate PRDs
pub const SYSTEM_PROMPT: &str = r#"You are Ralph, an AI assistant that generates Product Requirement Documents (PRDs) for software projects.

## Your Response Format

You MUST respond with valid JSON matching this schema. Your response should be ONLY the JSON object, with no markdown code fences or other formatting.

{
  "phase": "exploring" | "asking" | "working" | "complete",
  "status": "optional status message",
  "questions": [...],  // when phase is "asking"
  "context": {...},    // accumulated findings
  "prd": {...}         // when phase is "complete"
}

## Phase Guidelines

### Phase: exploring
Use when you need to understand the codebase before proceeding.
- Read key files to understand project structure
- Identify languages, frameworks, and patterns
- Set status to describe what you're learning

### Phase: asking
Use when you genuinely need user input to proceed.
- Only ask questions when the answer significantly affects the PRD
- Skip this phase for well-defined, specific tasks
- Group related questions together (max 4 per turn)
- Each question needs: id, category, text, allow_freeform
- Optionally include options for multiple choice

Question categories: "scope", "technical", "quality", "priority"

### Phase: working
Use when you're generating requirements and tasks.
- Set status to describe what you're creating
- Populate context.requirements and context.tasks as you work

### Phase: complete
Use when the PRD is ready.
- Include the full prd object with name, quality_gates, and tasks
- Each task needs: category, description, steps

## Important Rules

1. **Skip unnecessary phases** - For clear, specific tasks, go directly to working or complete
2. **Don't over-ask** - Only ask questions when truly needed. "Add a logout button" doesn't need 10 questions.
3. **Be efficient** - A simple task might complete in 1-2 turns
4. **Match project conventions** - Use the same testing/build tools the project already uses

## Task Format

Each task in the PRD should have:
- category: The type of work (e.g., "feature", "bugfix", "refactor", "test", "docs")
- description: What needs to be done
- steps: Specific implementation steps
- passes: Always false initially (set to true when complete)

## Quality Gates

Include quality gates appropriate for the project:
- Use the project's existing test/lint/build commands
- Common gates: "cargo test", "cargo clippy", "cargo fmt --check"
"#;

/// Build the initial prompt for a new planning session
pub fn build_initial_prompt(user_request: &str) -> String {
    format!(
        r#"{SYSTEM_PROMPT}

## User Request

{user_request}

Begin by exploring the codebase to understand the project structure, then proceed based on your judgment."#
    )
}

/// Build a continuation prompt with user answers
pub fn build_continuation_prompt(answers: &[Answer]) -> String {
    if answers.is_empty() {
        return "Continue with the PRD generation.".to_string();
    }

    let mut prompt = String::from("User provided the following answers:\n\n");

    for answer in answers {
        prompt.push_str(&format!("- {}: {}\n", answer.question_id, answer.value));
    }

    prompt.push_str("\nContinue with the PRD generation based on these answers.");
    prompt
}

/// Build a prompt to resume an interrupted session
pub fn build_resume_prompt(turn_count: u32, last_phase: &str) -> String {
    format!(
        r#"This is a resumed session. Previous state:
- Turns completed: {turn_count}
- Last phase: {last_phase}

Continue from where we left off. Respond with your current phase and any questions or the final PRD."#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_contains_phase_keywords() {
        assert!(SYSTEM_PROMPT.contains("exploring"));
        assert!(SYSTEM_PROMPT.contains("asking"));
        assert!(SYSTEM_PROMPT.contains("working"));
        assert!(SYSTEM_PROMPT.contains("complete"));
    }

    #[test]
    fn system_prompt_contains_json_format() {
        assert!(SYSTEM_PROMPT.contains("JSON"));
        assert!(SYSTEM_PROMPT.contains("phase"));
    }

    #[test]
    fn build_initial_prompt_includes_user_request() {
        let request = "Add user authentication";
        let prompt = build_initial_prompt(request);
        assert!(prompt.contains(request));
        assert!(prompt.contains(SYSTEM_PROMPT));
        assert!(prompt.contains("User Request"));
    }

    #[test]
    fn build_continuation_prompt_empty_answers() {
        let prompt = build_continuation_prompt(&[]);
        assert_eq!(prompt, "Continue with the PRD generation.");
    }

    #[test]
    fn build_continuation_prompt_with_answers() {
        let answers = vec![
            Answer {
                question_id: "q1".to_string(),
                value: "React".to_string(),
            },
            Answer {
                question_id: "q2".to_string(),
                value: "PostgreSQL".to_string(),
            },
        ];
        let prompt = build_continuation_prompt(&answers);
        assert!(prompt.contains("q1: React"));
        assert!(prompt.contains("q2: PostgreSQL"));
        assert!(prompt.contains("User provided the following answers"));
        assert!(prompt.contains("Continue with the PRD generation based on these answers"));
    }

    #[test]
    fn build_resume_prompt_includes_turn_count() {
        let prompt = build_resume_prompt(5, "asking");
        assert!(prompt.contains("Turns completed: 5"));
        assert!(prompt.contains("Last phase: asking"));
        assert!(prompt.contains("resumed session"));
    }

    #[test]
    fn build_resume_prompt_different_phases() {
        let prompt = build_resume_prompt(0, "exploring");
        assert!(prompt.contains("Last phase: exploring"));

        let prompt = build_resume_prompt(10, "working");
        assert!(prompt.contains("Turns completed: 10"));
        assert!(prompt.contains("Last phase: working"));
    }
}
