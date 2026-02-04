use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

use crate::app::App;
use crate::claude;
use crate::prd;
use crate::prompt;
use crate::tui;

/// Run the build command - executes PRD tasks in a loop
pub fn run(prd_path: &str, max_loops: u64) {
    let exit_clause = "<promise>COMPLETE</promise>";

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
        let mut child = claude::launch_claude(&prompt);

        app.set_status("Waiting for Claude... (q=quit, r=resume, Ctrl+C=kill)");

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
                        break;
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
        app.push_log(stdout.to_string());

        if let Some(latest) = app.latest_log()
            && latest
                .to_ascii_lowercase()
                .contains(&exit_clause.to_ascii_lowercase())
        {
            app.set_status("PRD Complete!");
            app.should_quit = true;
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
