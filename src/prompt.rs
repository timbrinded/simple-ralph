pub fn make_prompt(cfg: &str) -> String {
    format!("@{}{}", cfg, MASTER_PROMPT)
}

const MASTER_PROMPT: &str = r#"

@progress.txt
1.  Find the highest priority feature to work on and work only on that feature.
This should be the one you decide has the highest priority, not necessarily the 1st on the list.
2. Run the repo's quality gates (format/lint/typecheck/build/tests) using project-native commands (e.g., cargo fmt/clippy/test). If a gate is missing, note it.
3. Update the PRD with the work that was done.
4. Move completed tasks: For any task with passes=true in plans/prd.json, move it to plans/completed.json.
Add a completed_at field with today's date (YYYY-MM-DD). Remove the passes field.
Keep only category, description, steps, and completed_at. Skip tasks already in completed.json.
5. Append to the your progress to the progress.txt file.
Use this to leave a note for the next person working in the code base.
6. Make a git commit of that feature.
Only work on a single feature.
If while implementing the feature, you notice the PRD is complete, output <promise>COMPLETE</promise>"#;
