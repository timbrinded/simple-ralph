use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::Terminal;
use serde::Deserialize;
use std::time::Duration;

use crate::app::App;
use crate::claude;
use crate::prd;
use crate::prompt;
use crate::tui;

/// Maximum number of retry attempts for transient API errors
const MAX_RETRIES: u32 = 5;
/// Base delay for exponential backoff (doubles each retry)
const BASE_RETRY_DELAY_SECS: u64 = 5;

/// JSON schema for structured build iteration output
const BUILD_OUTPUT_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "task_number": {"type": "integer"},
    "status": {"type": "string", "enum": ["completed", "in_progress", "blocked", "skipped"]},
    "summary": {"type": "string"},
    "prd_complete": {"type": "boolean"}
  },
  "required": ["task_number", "status", "summary", "prd_complete"]
}"#;

/// Structured output from a build iteration
#[derive(Debug, Deserialize)]
pub struct BuildIterationOutput {
    pub task_number: i32,
    pub status: String,
    pub summary: String,
    pub prd_complete: bool,
}

/// Claude Code's JSON output wrapper when using --output-format json
#[derive(Debug, Deserialize)]
struct ClaudeJsonOutput {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    output_type: String,
    is_error: bool,
    structured_output: Option<BuildIterationOutput>,
    // Other fields (duration_ms, session_id, usage, etc.) are ignored
}

/// Result of attempting to run Claude
enum ClaudeResult {
    /// Successfully got structured output
    Success(BuildIterationOutput),
    /// Claude reported an error in the response
    ClaudeError(String),
    /// Transient error that should be retried (API 500, empty output, etc.)
    TransientError(String),
    /// Parse error or other non-retryable failure
    ParseError(String),
    /// User interrupted the process
    Interrupted,
}

/// Check if stderr indicates a retryable API error
fn is_retryable_error(stderr: &str) -> bool {
    let stderr_lower = stderr.to_lowercase();
    stderr_lower.contains("500")
        || stderr_lower.contains("502")
        || stderr_lower.contains("503")
        || stderr_lower.contains("504")
        || stderr_lower.contains("internal server error")
        || stderr_lower.contains("service unavailable")
        || stderr_lower.contains("bad gateway")
        || stderr_lower.contains("gateway timeout")
        || stderr_lower.contains("overloaded")
        || stderr_lower.contains("rate limit")
}

/// Default max turns per Claude session (generous for complex tasks, catches infinite loops)
const DEFAULT_MAX_TURNS: u32 = 200;

/// Run Claude and wait for output, handling keyboard events
/// Returns the result of the Claude invocation
fn run_claude_iteration<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    prompt: &str,
    max_turns: u32,
) -> ClaudeResult {
    let mut child = claude::launch_claude_with_options(&claude::ClaudeOptions {
        prompt,
        bypass_permissions: true,
        output_format: Some("json"),
        json_schema: Some(BUILD_OUTPUT_SCHEMA),
        max_turns: Some(max_turns),
        ..Default::default()
    });

    while child.try_wait().expect("Failed to check child").is_none() {
        terminal.draw(|f| app.draw(f)).expect("Failed to draw");
        app.advance_spinner();

        if event::poll(Duration::from_millis(100)).expect("Poll failed")
            && let Event::Key(key) = event::read().expect("Failed to read event")
        {
            match (key.code, key.modifiers) {
                // Ctrl+C: kill Claude and quit immediately
                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                    child.kill().expect("Failed to kill Claude");
                    app.should_quit = true;
                    app.set_status("Interrupted by user");
                    return ClaudeResult::Interrupted;
                }
                // q/Q: quit after Claude finishes
                (KeyCode::Char('q') | KeyCode::Char('Q'), _) => {
                    app.should_quit = true;
                    app.set_status("Will quit after Claude finishes this loop... (r=resume)");
                }
                // r/R: resume (cancel quit)
                (KeyCode::Char('r') | KeyCode::Char('R'), _) => {
                    app.should_quit = false;
                    app.set_status("Resumed. Waiting for Claude...");
                }
                // Left/Right: navigate between iteration logs
                (KeyCode::Left, _) => {
                    app.prev_log();
                }
                (KeyCode::Right, _) => {
                    app.next_log();
                }
                // Up/Down: scroll within current log
                (KeyCode::Up, _) => {
                    app.scroll_up(1);
                }
                (KeyCode::Down, _) => {
                    app.scroll_down(1);
                }
                (KeyCode::PageUp, _) => {
                    app.scroll_up(10);
                }
                (KeyCode::PageDown, _) => {
                    app.scroll_down(10);
                }
                _ => {}
            }
        }
    }

    let output = child.wait_with_output().expect("Failed to get output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for empty output (often indicates API error)
    if stdout.trim().is_empty() {
        if is_retryable_error(&stderr) {
            return ClaudeResult::TransientError(format!("API error: {}", stderr.trim()));
        } else if !stderr.trim().is_empty() {
            return ClaudeResult::TransientError(format!(
                "Empty output with stderr: {}",
                stderr.trim()
            ));
        } else {
            return ClaudeResult::TransientError("Empty output from Claude".to_string());
        }
    }

    // Parse JSON wrapper and extract structured_output
    match serde_json::from_str::<ClaudeJsonOutput>(&stdout) {
        Ok(wrapper) => {
            if let Some(result) = wrapper.structured_output {
                ClaudeResult::Success(result)
            } else if wrapper.is_error {
                // Check if this is a retryable API error
                if is_retryable_error(&stdout) {
                    ClaudeResult::TransientError(format!("Claude API error:\n{}", stdout))
                } else {
                    ClaudeResult::ClaudeError(stdout.to_string())
                }
            } else {
                ClaudeResult::ParseError(format!("No structured output:\n{}", stdout))
            }
        }
        Err(e) => {
            ClaudeResult::ParseError(format!("Parse error: {}\n\nRaw output:\n{}", e, stdout))
        }
    }
}

/// Run the build command - executes PRD tasks in a loop
pub fn run(prd_path: &str, max_loops: u64, max_turns: Option<u32>) {
    let max_turns = max_turns.unwrap_or(DEFAULT_MAX_TURNS);
    let prd = prd::load_prd_from_file(prd_path);
    let completed = prd::load_completed_tasks_from_file(prd_path);
    let remaining = prd.tasks.len();
    let completed_count = completed.map_or(0, |t| t.len());

    let mut terminal = tui::init_terminal();
    let mut app = App::new(&prd.name, remaining, completed_count);

    while !app.should_quit && app.loop_count < max_loops {
        let prd = prd::load_prd_from_file(prd_path);
        let completed = prd::load_completed_tasks_from_file(prd_path);
        app.reload_progress(prd.tasks.len(), completed.map_or(0, |t| t.len()));

        app.increment_loop();
        app.start_loop_timer();
        app.set_status("Spawning Claude...");
        terminal.draw(|f| app.draw(f)).expect("Failed to draw");

        let prompt = prompt::make_prompt(prd_path);

        // Retry loop for transient errors
        let mut retry_count = 0;
        loop {
            if retry_count > 0 {
                let delay = BASE_RETRY_DELAY_SECS * 2u64.pow(retry_count - 1);
                app.set_status(&format!(
                    "Retry {}/{} in {}s... (API error)",
                    retry_count, MAX_RETRIES, delay
                ));
                terminal.draw(|f| app.draw(f)).expect("Failed to draw");

                // Sleep with event polling to stay responsive
                let deadline = std::time::Instant::now() + Duration::from_secs(delay);
                while std::time::Instant::now() < deadline {
                    if event::poll(Duration::from_millis(100)).expect("Poll failed")
                        && let Event::Key(key) = event::read().expect("Failed to read event")
                    {
                        if let (KeyCode::Char('c'), m) = (key.code, key.modifiers) {
                            if m.contains(KeyModifiers::CONTROL) {
                                app.should_quit = true;
                                app.set_status("Interrupted by user");
                                break;
                            }
                        }
                    }
                    terminal.draw(|f| app.draw(f)).expect("Failed to draw");
                    app.advance_spinner();
                }

                if app.should_quit {
                    break;
                }

                app.set_status(&format!("Retrying ({}/{})...", retry_count, MAX_RETRIES));
            } else {
                app.set_status("Waiting for Claude... (q=quit, r=resume, Ctrl+C=kill)");
            }
            terminal.draw(|f| app.draw(f)).expect("Failed to draw");
            app.advance_spinner();

            match run_claude_iteration(&mut terminal, &mut app, &prompt, max_turns) {
                ClaudeResult::Success(result) => {
                    // Format for display
                    let display_log = format!(
                        "Task #{}: {}\nStatus: {}\nSummary: {}",
                        result.task_number,
                        if result.prd_complete {
                            "PRD COMPLETE"
                        } else {
                            ""
                        },
                        result.status,
                        result.summary
                    );
                    app.push_log(display_log);

                    if result.prd_complete {
                        app.set_status("PRD Complete!");
                        app.should_quit = true;
                    } else {
                        let status_msg = format!("Task {} {}", result.task_number, result.status);
                        app.set_status(&status_msg);
                    }
                    break;
                }
                ClaudeResult::ClaudeError(output) => {
                    app.push_log(format!("Claude returned error\n\nRaw output:\n{}", output));
                    app.set_status("Error: Claude reported failure");
                    break;
                }
                ClaudeResult::TransientError(msg) => {
                    retry_count += 1;
                    if retry_count > MAX_RETRIES {
                        app.push_log(format!(
                            "Failed after {} retries\n\nLast error: {}",
                            MAX_RETRIES, msg
                        ));
                        app.set_status("Error: Max retries exceeded");
                        break;
                    }
                    app.push_log(format!("Transient error (will retry): {}", msg));
                    // Continue to next iteration of retry loop
                }
                ClaudeResult::ParseError(msg) => {
                    app.push_log(msg);
                    app.set_status("Warning: Failed to parse Claude output");
                    break;
                }
                ClaudeResult::Interrupted => {
                    // app.should_quit already set
                    break;
                }
            }
        }

        terminal.draw(|f| app.draw(f)).expect("Failed to draw");
    }

    tui::restore_terminal();

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("Ralph Session Complete");
    println!("Loops: {}", app.loop_count);
    println!("Final status: {}", app.status_message);
    if let Some(latest) = app.latest_log() {
        println!("\n─── Last Claude Output ───\n{}", latest);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_build_output() {
        let json = r#"{"task_number": 1, "status": "completed", "summary": "Added auth", "prd_complete": false}"#;
        let result: BuildIterationOutput = serde_json::from_str(json).unwrap();
        assert_eq!(result.task_number, 1);
        assert_eq!(result.status, "completed");
        assert!(!result.prd_complete);
    }

    #[test]
    fn parse_prd_complete_output() {
        let json = r#"{"task_number": 5, "status": "completed", "summary": "Final task", "prd_complete": true}"#;
        let result: BuildIterationOutput = serde_json::from_str(json).unwrap();
        assert!(result.prd_complete);
    }

    #[test]
    fn parse_blocked_status() {
        let json = r#"{"task_number": 2, "status": "blocked", "summary": "Needs API key", "prd_complete": false}"#;
        let result: BuildIterationOutput = serde_json::from_str(json).unwrap();
        assert_eq!(result.status, "blocked");
    }

    #[test]
    fn invalid_json_returns_error() {
        let json = "not valid json";
        let result = serde_json::from_str::<BuildIterationOutput>(json);
        assert!(result.is_err());
    }

    #[test]
    fn build_output_schema_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(BUILD_OUTPUT_SCHEMA).unwrap();
        assert_eq!(parsed["type"], "object");
    }

    // Tests for Claude Code JSON wrapper format
    #[test]
    fn parse_claude_json_wrapper() {
        // This is the ACTUAL format from `claude --output-format json`
        let json = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":100,"structured_output":{"task_number":1,"status":"completed","summary":"Did stuff","prd_complete":false}}"#;
        let wrapper: ClaudeJsonOutput = serde_json::from_str(json).unwrap();
        assert_eq!(wrapper.output_type, "result");
        assert!(!wrapper.is_error);
        let output = wrapper.structured_output.unwrap();
        assert_eq!(output.task_number, 1);
        assert_eq!(output.status, "completed");
    }

    #[test]
    fn parse_claude_wrapper_with_prd_complete() {
        let json = r#"{"type":"result","subtype":"success","is_error":false,"structured_output":{"task_number":5,"status":"completed","summary":"Final","prd_complete":true}}"#;
        let wrapper: ClaudeJsonOutput = serde_json::from_str(json).unwrap();
        let output = wrapper.structured_output.unwrap();
        assert!(output.prd_complete);
    }

    #[test]
    fn parse_claude_wrapper_error_case() {
        let json =
            r#"{"type":"result","subtype":"error","is_error":true,"structured_output":null}"#;
        let wrapper: ClaudeJsonOutput = serde_json::from_str(json).unwrap();
        assert!(wrapper.is_error);
        assert!(wrapper.structured_output.is_none());
    }

    #[test]
    fn parse_real_claude_output_sample() {
        // Exact sample from actual failure - ensures we don't regress
        let json = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":386510,"duration_api_ms":283106,"num_turns":46,"result":"","session_id":"b7e6c276-18db-4a9a-b6ae-6a2ecb2d4a33","total_cost_usd":2.7654437499999998,"usage":{"input_tokens":2},"structured_output":{"task_number":1,"status":"completed","summary":"Created modal","prd_complete":false},"uuid":"f2ff63de-7bba-40fe-9072-0e2073d2c663"}"#;
        let wrapper: ClaudeJsonOutput = serde_json::from_str(json).unwrap();
        assert_eq!(wrapper.output_type, "result");
        assert!(!wrapper.is_error);
        let output = wrapper.structured_output.unwrap();
        assert_eq!(output.task_number, 1);
        assert!(!output.prd_complete);
    }

    // Tests for retryable error detection
    #[test]
    fn retryable_error_500() {
        assert!(is_retryable_error("Error: 500 Internal Server Error"));
        assert!(is_retryable_error("internal server error"));
    }

    #[test]
    fn retryable_error_502() {
        assert!(is_retryable_error("502 Bad Gateway"));
        assert!(is_retryable_error("bad gateway"));
    }

    #[test]
    fn retryable_error_503() {
        assert!(is_retryable_error("503 Service Unavailable"));
        assert!(is_retryable_error("service unavailable"));
    }

    #[test]
    fn retryable_error_504() {
        assert!(is_retryable_error("504 Gateway Timeout"));
        assert!(is_retryable_error("gateway timeout"));
    }

    #[test]
    fn retryable_error_overloaded() {
        assert!(is_retryable_error("API is overloaded"));
    }

    #[test]
    fn retryable_error_rate_limit() {
        assert!(is_retryable_error("rate limit exceeded"));
    }

    #[test]
    fn non_retryable_error() {
        assert!(!is_retryable_error("invalid request"));
        assert!(!is_retryable_error("authentication failed"));
        assert!(!is_retryable_error(""));
    }
}
