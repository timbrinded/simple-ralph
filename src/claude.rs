use std::process::{Command, Stdio};

/// Options for launching Claude Code
#[derive(Debug, Default)]
pub struct ClaudeOptions<'a> {
    /// The prompt to send
    pub prompt: &'a str,

    /// Session ID for --session-id flag (starts new named session)
    pub session_id: Option<&'a str>,

    /// Whether to continue the previous session (-c flag)
    pub continue_session: bool,

    /// JSON schema for structured output (--json-schema flag)
    pub json_schema: Option<&'a str>,

    /// Whether to bypass permissions (--permission-mode bypassPermissions)
    pub bypass_permissions: bool,
}

/// Launch Claude Code with the given options
pub fn launch_claude_with_options(opts: &ClaudeOptions) -> std::process::Child {
    let mut args = Vec::new();

    // Permission mode
    if opts.bypass_permissions {
        args.push("--permission-mode");
        args.push("bypassPermissions");
    }

    // Session management
    if let Some(session_id) = opts.session_id {
        args.push("--session-id");
        args.push(session_id);
    }

    if opts.continue_session {
        args.push("-c");
    }

    // JSON schema for structured output
    if let Some(schema) = opts.json_schema {
        args.push("--json-schema");
        args.push(schema);
    }

    // Prompt
    args.push("-p");
    args.push(opts.prompt);

    Command::new("claude")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Error spawning claude code!")
}

/// Launch Claude Code with simple prompt (backward compatible)
pub fn launch_claude(prompt: &str) -> std::process::Child {
    launch_claude_with_options(&ClaudeOptions {
        prompt,
        bypass_permissions: true,
        ..Default::default()
    })
}
