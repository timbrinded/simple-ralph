pub fn make_prompt(prd_path: &str) -> String {
    format!("@{}{}", prd_path, MASTER_PROMPT)
}

const MASTER_PROMPT: &str = r#"

@progress.txt
1. Find the highest priority feature to work on and work only on that feature.
   - This should be the one you decide has the highest priority, not necessarily the 1st on the list.
   - If you need to see what completed tasks were written you can check completed.json for completed tasks.
2. Run the repo's quality gates (format/lint/typecheck/build/tests) using project-native commands. If a gate is missing, note it.
3. Update the PRD with the work that was done.
4. Append to the your progress to the progress.txt file.
   - Use this to leave a note for the next person working in the code base.
5. Move completed tasks: For any task with passes=true in the PRD JSON file, move it to completed.json in the same directory.
   - Add a completed_at field with today's date (YYYY-MM-DD). Remove the passes field.
   - Keep only category, description, steps, and completed_at. Skip tasks already in completed.json.
6. Make a git commit of that feature.
   - Only work on a single feature.

After completing your work, output a JSON summary with:
- task_number: The task number you worked on (1-indexed from the PRD)
- status: "completed" if done, "in_progress" if partially done, "blocked" if stuck, "skipped" if not applicable
- summary: Brief description of what you did
- prd_complete: true if all PRD tasks are now done, false otherwise
"#;

const _REGRETS_PROMPT: &str = r#"
hello
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_prompt_includes_prd_path() {
        let prompt = make_prompt("/path/to/prd.json");
        assert!(prompt.starts_with("@/path/to/prd.json"));
    }

    #[test]
    fn make_prompt_includes_progress_reference() {
        let prompt = make_prompt("prd.json");
        assert!(prompt.contains("@progress.txt"));
    }

    #[test]
    fn make_prompt_includes_master_instructions() {
        let prompt = make_prompt("prd.json");
        assert!(prompt.contains("Find the highest priority feature"));
        assert!(prompt.contains("quality gates"));
        assert!(prompt.contains("git commit"));
    }

    #[test]
    fn make_prompt_includes_completed_json_reference() {
        let prompt = make_prompt("prd.json");
        assert!(prompt.contains("completed.json"));
    }

    #[test]
    fn master_prompt_contains_json_output_instructions() {
        assert!(MASTER_PROMPT.contains("output a JSON summary"));
        assert!(MASTER_PROMPT.contains("prd_complete"));
    }
}
