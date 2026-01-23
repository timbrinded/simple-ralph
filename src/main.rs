use clap::Parser;
use crossterm::event::{Event, KeyCode, poll, read};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
mod claude;
mod prompt;

static SHOULD_QUIT: AtomicBool = AtomicBool::new(false);
static LOOP_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    // Include description that this should be prd json
    name: Option<String>,
}

fn main() {
    let args = Args::parse();

    let cfg = args.name.as_deref().unwrap_or("plans/prd.json");
    let exit_clause = "<promise>COMPLETE</promise>";

    loop {
        let prompt = prompt::make_prompt(cfg);
        let handle = std::thread::spawn(move || claude::launch_claude(&prompt));

        println!(
            "Starting Coding loop #{} (type 'f' to finish after this loop, 'r' to resume)",
            LOOP_COUNT.fetch_add(1, Ordering::SeqCst)
        );

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg} [{elapsed}]")
                .expect("invalid template"),
        );
        spinner.set_message("Waiting for Claude...");

        enable_raw_mode().expect("Failed to enable raw mode");

        while !handle.is_finished() {
            if poll(Duration::from_millis(100)).expect("Poll failed") {
                if let Event::Key(key_event) = read().expect("Failed to read event") {
                    match key_event.code {
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            SHOULD_QUIT.store(true, Ordering::SeqCst);
                            spinner.set_message("Finishing after this command... (R to resume)");
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            SHOULD_QUIT.store(false, Ordering::SeqCst);
                            spinner.set_message("Waiting for Claude...");
                        }
                        _ => {}
                    }
                }
            }
            spinner.tick();
        }

        disable_raw_mode().expect("Failed to disable raw mode");
        spinner.finish_and_clear();

        let result = handle.join().unwrap();
        println!("Output: {}", result.trim());

        if SHOULD_QUIT.load(Ordering::SeqCst) {
            println!("Termination signal received. Exiting...");
            break;
        }

        if result
            .to_ascii_lowercase()
            .contains(exit_clause.to_ascii_lowercase().as_str())
        {
            break;
        }
    }

    println!("Completed greeting loop.");
}
