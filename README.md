# ralph

A CLI tool for PRD-driven AI development workflows. Uses Claude Code to execute tasks from a PRD file or generate new PRDs through interactive conversation.

![simple-ralph](simple-ralph.png)

Based on [Anthropic's harness pattern for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents).

## Requirements

- [Claude Code CLI](https://github.com/anthropics/claude-code) installed and available as `claude` in PATH
- Rust toolchain (for building from source)
- Git (for automatic commits during build loops)

## Installation

```bash
cargo build --release
# Binary will be at target/release/ralph
```

Or install directly:

```bash
cargo install --path .
```

## Commands

### `ralph build` — Execute PRD Tasks

Runs an iterative loop where Claude Code works through tasks in a PRD file.

```bash
ralph build [OPTIONS]

Options:
  -p, --prd-path <PATH>  Path to PRD JSON file [default: plans/prd.json]
  -l, --max-loops <N>    Maximum iterations to run [default: unlimited]
```

**Example:**
```bash
ralph build --prd-path plans/prd.json --max-loops 10
```

**What happens:**
1. Loads PRD and any previously completed tasks
2. Invokes Claude Code with a prompt referencing the PRD
3. Claude works on the highest priority incomplete task
4. Claude runs quality gates, updates progress, commits changes
5. Repeats until all tasks complete or max loops reached

**TUI Controls:**
- `q` / `Q` — Queue stop after current loop finishes
- `r` / `R` — Resume (cancel queued stop)
- `Ctrl+C` — Kill Claude immediately
- `←` / `→` — Navigate between iteration logs
- `↑` / `↓` / `PgUp` / `PgDn` — Scroll current log

### `ralph plan` — Generate a PRD

Interactive multi-turn conversation to generate a new PRD file.

```bash
ralph plan [OPTIONS]

Options:
  -o, --output <PATH>       Output path for PRD [default: plans/prd.json]
  -r, --resume              Resume an interrupted session
  -f, --force               Force overwrite existing files
  -d, --description <TEXT>  Initial description of what to build
```

**Example:**
```bash
ralph plan --output plans/prd.json --description "A CLI tool for managing bookmarks"
```

## PRD File Format

```json
{
  "name": "Project Name",
  "quality_gates": [
    "cargo check",
    "cargo test",
    "cargo clippy -- -D warnings"
  ],
  "tasks": [
    {
      "category": "functional",
      "description": "Brief description of the feature",
      "steps": [
        "Step 1",
        "Step 2",
        "Step 3"
      ],
      "passes": false
    }
  ]
}
```

**Fields:**
- `name` — Project identifier
- `quality_gates` — Commands Claude runs to verify changes (tests, lints, etc.)
- `tasks[]` — Array of tasks to complete
  - `category` — Task type (e.g., "functional", "refactor", "bugfix")
  - `description` — What needs to be done
  - `steps` — Verification steps or acceptance criteria
  - `passes` — Whether the task is complete (`true`/`false`)

## File Conventions

When running `ralph build`:

| File | Purpose |
|------|---------|
| `plans/prd.json` | PRD with tasks (default path) |
| `plans/completed.json` | Auto-generated log of completed tasks |
| `progress.txt` | Running progress notes (same dir as PRD) |

## Development

```bash
cargo check           # Type check
cargo test            # Run tests
cargo clippy          # Lint
cargo fmt             # Format
```
