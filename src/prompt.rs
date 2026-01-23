pub fn make_prompt(prd_path: &str) -> String {
    format!("@{}{}", prd_path, MASTER_PROMPT)
}

const MASTER_PROMPT: &str = r#"

@progress.txt
1.  Find the highest priority feature to work on and work only on that feature.
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
  - If while implementing the feature, you notice the PRD is now complete (with no tasks remaining), output <promise>COMPLETE</promise>
"#;

const _REGRETS_PROMPT: &str = r#"
hello
"#;
