use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

use crate::claude::{ClaudeOptions, launch_claude_with_options};
use crate::plan::{
    app::{InputMode, PlanApp},
    phases::PlanPhase,
    prompts::{build_continuation_prompt, build_initial_prompt, build_resume_prompt},
    protocol::{PLAN_RESPONSE_SCHEMA, PlanResponse},
    session::{PlanSession, SessionError},
};
use crate::tui;

#[derive(Error, Debug)]
pub enum PlanError {
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Claude returned invalid output (not JSON):\n{0}")]
    InvalidOutput(String),

    #[error("Output file already exists. Use --resume to continue or --force to overwrite.")]
    OutputExists,
}

/// Run the plan command - multi-turn PRD generation
pub fn run(
    output: &str,
    resume: bool,
    force: bool,
    request: Option<&str>,
) -> Result<(), PlanError> {
    // Check if output file exists
    let output_path = Path::new(output);
    if output_path.exists() && !resume && !force {
        return Err(PlanError::OutputExists);
    }

    // Ensure output directory exists
    if let Some(parent) = output_path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?;
    }

    // Load or create session
    let mut session = PlanSession::load_or_create(output, resume, force)?;

    // Initialize TUI
    let mut terminal = tui::init_terminal();
    let mut app = PlanApp::new();

    // If no description provided via CLI, show idea input screen first
    let user_request: String = if let Some(desc) = request {
        desc.to_string()
    } else {
        collect_idea(&mut terminal, &mut app)?;
        if app.should_quit {
            tui::restore_terminal();
            return Ok(());
        }
        app.idea_input.clone()
    };

    // Build initial prompt
    let initial_prompt = if session.is_fresh() {
        build_initial_prompt(&user_request)
    } else {
        build_resume_prompt(session.turn_count, &session.last_phase.to_string())
    };

    app.status = format!("Starting plan session: {}", session.id);
    app.turn_count = session.turn_count;

    // Main loop
    loop {
        terminal.draw(|f| app.draw(f)).expect("Failed to draw");

        // Build prompt for this turn
        let prompt = if session.is_fresh() {
            initial_prompt.clone()
        } else if !app.answers.is_empty() {
            build_continuation_prompt(&app.take_answers())
        } else {
            "Continue with the PRD generation.".to_string()
        };

        // Launch Claude
        app.status = "Invoking Claude...".to_string();
        terminal.draw(|f| app.draw(f)).expect("Failed to draw");

        // Always use --session-id to ensure we resume the correct session
        // (using -c alone would continue the "last" session, which might not be ours
        // if the user ran other claude commands in between)
        let opts = ClaudeOptions {
            prompt: &prompt,
            session_id: Some(&session.id),
            continue_session: false, // --session-id handles resumption
            json_schema: Some(PLAN_RESPONSE_SCHEMA),
            bypass_permissions: true,
        };

        let mut child = launch_claude_with_options(&opts);

        app.status = "Waiting for Claude... (q=quit, Ctrl+C=kill)".to_string();

        // Wait for Claude with event handling
        while child.try_wait().expect("Failed to check child").is_none() {
            terminal.draw(|f| app.draw(f)).expect("Failed to draw");

            if event::poll(Duration::from_millis(100)).expect("Poll failed")
                && let Event::Key(key) = event::read().expect("Failed to read event")
            {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                        child.kill().expect("Failed to kill Claude");
                        app.should_quit = true;
                        app.status = "Interrupted by user".to_string();
                        break;
                    }
                    (KeyCode::Char('q') | KeyCode::Char('Q'), _) => {
                        app.should_quit = true;
                        app.status = "Will quit after Claude finishes...".to_string();
                    }
                    (KeyCode::Up, _) => app.scroll_up(1),
                    (KeyCode::Down, _) => app.scroll_down(1),
                    _ => {}
                }
            }
        }

        if app.should_quit {
            session.save()?;
            break;
        }

        // Get Claude's output
        let output_result = child.wait_with_output().expect("Failed to get output");
        let stdout = String::from_utf8_lossy(&output_result.stdout);
        let stderr = String::from_utf8_lossy(&output_result.stderr);

        // Log the raw output
        app.push_log(stdout.to_string());

        // Parse JSON response
        let response: PlanResponse = match serde_json::from_str(&stdout) {
            Ok(r) => r,
            Err(e) => {
                // Check if it looks like JSON at all
                let trimmed = stdout.trim();
                if !trimmed.starts_with('{') {
                    // Not JSON - this is an InvalidOutput error
                    app.status = "Claude returned non-JSON output".to_string();
                    let error_detail = if stderr.is_empty() {
                        stdout.to_string()
                    } else {
                        format!("stdout: {}\nstderr: {}", stdout, stderr)
                    };
                    app.push_log(format!(
                        "ERROR: Expected JSON but got plain text.\n\nRaw output:\n{}",
                        error_detail
                    ));
                    tui::restore_terminal();
                    return Err(PlanError::InvalidOutput(error_detail));
                }

                // Malformed JSON - log and try to continue
                app.status = format!("Failed to parse Claude response: {}", e);
                app.push_log(format!("Parse error: {}\n\nRaw output:\n{}", e, stdout));
                session.advance(PlanPhase::Working);
                session.save()?;
                continue;
            }
        };

        // Update app state from response
        app.update_from_response(&response);
        session.advance(response.phase);

        // Merge any context
        if let Some(context) = response.context {
            session.merge_context(context);
        }

        // Save session state
        session.save()?;

        // Handle phase-specific logic
        match response.phase {
            PlanPhase::Complete => {
                // PRD is ready - write to output file
                if let Some(prd) = response.prd {
                    let prd_json = serde_json::to_string_pretty(&prd)?;
                    let mut file = std::fs::File::create(output)?;
                    file.write_all(prd_json.as_bytes())?;

                    app.status = format!("PRD written to {}", output);
                    app.push_log(format!("PRD generated successfully!\n\n{}", prd_json));

                    // Cleanup session file on success
                    let _ = session.cleanup();
                }
                terminal.draw(|f| app.draw(f)).expect("Failed to draw");

                // Wait for user to acknowledge
                wait_for_key(&mut terminal, &mut app)?;
                break;
            }
            PlanPhase::Asking => {
                // Claude needs input - show questions and collect answers
                if let Some(questions) = response.questions {
                    app.set_questions(questions);
                    collect_answers(&mut terminal, &mut app)?;

                    if app.should_quit {
                        session.save()?;
                        break;
                    }

                    // Only proceed if user explicitly submitted
                    if !app.should_submit {
                        // User didn't submit (maybe navigated away) - save and break
                        session.save()?;
                        break;
                    }

                    // Store answers in session
                    for answer in &app.answers {
                        session.add_answer(answer.clone());
                    }

                    // Reset for next round
                    app.reset_submit();
                }
            }
            PlanPhase::Exploring | PlanPhase::Working => {
                // Claude is working autonomously - just update status and continue
                app.status = response.status.unwrap_or_else(|| "Working...".to_string());
            }
        }

        terminal.draw(|f| app.draw(f)).expect("Failed to draw");
    }

    tui::restore_terminal();

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("Ralph Plan Session Complete");
    println!("Session ID: {}", session.id);
    println!("Turns: {}", session.turn_count);
    println!("Final phase: {}", session.last_phase);
    if session.last_phase == PlanPhase::Complete {
        println!("Output: {}", output);
    }

    Ok(())
}

/// Collect the user's idea/description via TUI before starting Claude
fn collect_idea(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut PlanApp,
) -> Result<(), PlanError> {
    app.awaiting_idea = true;

    loop {
        terminal.draw(|f| app.draw(f)).expect("Failed to draw");

        if event::poll(Duration::from_millis(100)).expect("Poll failed")
            && let Event::Key(key) = event::read().expect("Failed to read event")
        {
            match key.code {
                KeyCode::Enter if !app.idea_input.trim().is_empty() => {
                    app.awaiting_idea = false;
                    return Ok(());
                }
                KeyCode::Esc => {
                    app.should_quit = true;
                    app.awaiting_idea = false;
                    return Ok(());
                }
                KeyCode::Char(c) => {
                    app.idea_input.insert(app.idea_cursor, c);
                    app.idea_cursor += 1;
                }
                KeyCode::Backspace if app.idea_cursor > 0 => {
                    app.idea_cursor -= 1;
                    app.idea_input.remove(app.idea_cursor);
                }
                KeyCode::Left if app.idea_cursor > 0 => {
                    app.idea_cursor -= 1;
                }
                KeyCode::Right if app.idea_cursor < app.idea_input.len() => {
                    app.idea_cursor += 1;
                }
                _ => {}
            }
        }
    }
}

/// Collect answers from the user via TUI
/// Requires explicit Ctrl+Enter to submit all answers
fn collect_answers(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut PlanApp,
) -> Result<(), PlanError> {
    app.reset_submit();

    loop {
        terminal.draw(|f| app.draw(f)).expect("Failed to draw");

        if event::poll(Duration::from_millis(100)).expect("Poll failed")
            && let Event::Key(key) = event::read().expect("Failed to read event")
        {
            match app.input_mode {
                InputMode::Editing => {
                    match key.code {
                        KeyCode::Esc => {
                            app.exit_editing();
                        }
                        KeyCode::Enter => {
                            // Submit freeform answer and move to next question
                            app.submit_answer();
                            app.exit_editing();
                            if app.current_question + 1 < app.questions.len() {
                                app.next_question();
                            }
                            // Don't auto-submit - wait for Ctrl+Enter
                        }
                        KeyCode::Backspace => {
                            app.delete_char();
                        }
                        KeyCode::Left => {
                            app.move_cursor_left();
                        }
                        KeyCode::Right => {
                            app.move_cursor_right();
                        }
                        KeyCode::Char(c) => {
                            app.enter_char(c);
                        }
                        _ => {}
                    }
                }
                InputMode::Normal => {
                    match (key.code, key.modifiers) {
                        // Ctrl+C: quit immediately
                        (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                            app.should_quit = true;
                            return Ok(());
                        }
                        // Ctrl+Enter: submit all answers (only when all answered)
                        (KeyCode::Enter, m) if m.contains(KeyModifiers::CONTROL) => {
                            if app.all_answered() {
                                app.should_submit = true;
                                return Ok(());
                            }
                            // Flash status to indicate not ready
                            app.status = format!(
                                "Answer all questions first ({}/{})",
                                app.answered_count(),
                                app.questions.len()
                            );
                        }
                        // q/Q: quit
                        (KeyCode::Char('q') | KeyCode::Char('Q'), _) => {
                            app.should_quit = true;
                            return Ok(());
                        }
                        // i: enter editing mode for freeform input
                        (KeyCode::Char('i'), _) => {
                            if let Some(q) = app.current_question()
                                && (q.allow_freeform || q.options.is_none())
                            {
                                app.enter_editing();
                            }
                        }
                        // Up/Down: navigate options
                        (KeyCode::Up, _) => {
                            app.prev_option();
                        }
                        (KeyCode::Down, _) => {
                            app.next_option();
                        }
                        // Tab: next question
                        (KeyCode::Tab, _) => {
                            if app.current_question + 1 < app.questions.len() {
                                app.next_question();
                            }
                        }
                        // Shift+Tab: previous question
                        (KeyCode::BackTab, _) => {
                            app.prev_question();
                        }
                        // Enter: submit answer for current question, move to next
                        (KeyCode::Enter, _) => {
                            app.submit_answer();
                            if app.current_question + 1 < app.questions.len() {
                                app.next_question();
                            }
                            // Don't auto-submit when on last question - wait for Ctrl+Enter
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Wait for user to press any key
fn wait_for_key(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut PlanApp,
) -> Result<(), PlanError> {
    app.status = "PRD complete! Press any key to exit...".to_string();
    terminal.draw(|f| app.draw(f)).expect("Failed to draw");

    loop {
        if event::poll(Duration::from_millis(100)).expect("Poll failed")
            && let Event::Key(_) = event::read().expect("Failed to read event")
        {
            return Ok(());
        }
    }
}
