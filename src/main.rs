use clap::{Parser, Subcommand};

mod app;
mod claude;
mod commands;
mod plan;
mod prd;
mod prompt;
mod tui;

#[derive(Parser, Debug)]
#[command(name = "ralph")]
#[command(version, about = "Ralph - AI-powered PRD execution and generation", long_about = None)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute tasks from an existing PRD file
    Build {
        /// Path to the PRD JSON file
        #[arg(short, long, default_value = "plans/prd.json")]
        prd_path: String,

        /// Maximum number of loops to run
        #[arg(short = 'l', long)]
        max_loops: Option<u64>,

        /// Maximum agentic turns per Claude session (prevents hung sessions)
        #[arg(short = 't', long)]
        max_turns: Option<u32>,
    },

    /// Generate a new PRD through interactive multi-turn conversation
    Plan {
        /// Output path for the generated PRD
        #[arg(short, long, default_value = "plans/prd.json")]
        output: String,

        /// Resume an interrupted session
        #[arg(short, long)]
        resume: bool,

        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,

        /// Description of what to build (optional)
        #[arg(short = 'd', long)]
        description: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Build {
            prd_path,
            max_loops,
            max_turns,
        }) => {
            commands::build::run(&prd_path, max_loops.unwrap_or(u64::MAX), max_turns);
        }
        Some(Commands::Plan {
            output,
            resume,
            force,
            description,
        }) => {
            if let Err(e) = commands::plan::run(&output, resume, force, description.as_deref()) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            // arg_required_else_help ensures this is unreachable in normal CLI usage
            unreachable!("clap should show help when no subcommand is provided");
        }
    }
}
