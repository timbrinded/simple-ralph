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

    /// Output format (--output-format flag): "text", "json", or "stream-json"
    pub output_format: Option<&'a str>,
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

    // Output format
    if let Some(format) = opts.output_format {
        args.push("--output-format");
        args.push(format);
    }

    // Prompt
    args.push("-p");
    args.push(opts.prompt);

    Command::new("claude")
        .args(args)
        .stdin(Stdio::null())
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

/// Error returned when Haiku normalization fails
#[derive(Debug)]
pub struct NormalizationError {
    pub message: String,
    pub raw_output: String,
}

impl std::fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n\nRaw output:\n{}", self.message, self.raw_output)
    }
}

impl std::error::Error for NormalizationError {}

/// Use Haiku to normalize malformed JSON output into valid JSON matching a schema.
///
/// This is a fallback mechanism when strict JSON parsing fails. Haiku is fast and cheap,
/// making it ideal for this "JSON repair" task.
pub fn normalize_json_with_haiku(
    raw_output: &str,
    target_schema: &str,
) -> Result<String, NormalizationError> {
    let normalization_prompt = format!(
        r#"Given this raw output from Claude:
---
{raw_output}
---

Extract the structured data and return it as valid JSON matching this schema:
{target_schema}

Rules:
1. Return ONLY valid JSON, no markdown or explanation
2. If fields are missing, use sensible defaults (empty string, false, empty array)
3. The "phase" field MUST be one of: "exploring", "asking", "working", "complete"
4. Preserve all question/answer data as accurately as possible"#
    );

    let child = Command::new("claude")
        .args(["--model", "haiku", "-p", &normalization_prompt])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return Err(NormalizationError {
                message: format!("Failed to spawn Haiku process: {}", e),
                raw_output: raw_output.to_string(),
            });
        }
    };

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            return Err(NormalizationError {
                message: format!("Failed to get Haiku output: {}", e),
                raw_output: raw_output.to_string(),
            });
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();

    // Haiku might wrap the JSON in markdown code blocks - strip them
    let json_str = if trimmed.starts_with("```") {
        // Find the actual JSON content between code blocks
        let without_prefix = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        without_prefix
            .strip_suffix("```")
            .unwrap_or(without_prefix)
            .trim()
    } else {
        trimmed
    };

    // Validate it's actual JSON before returning
    if !json_str.starts_with('{') {
        return Err(NormalizationError {
            message: format!("Haiku did not return valid JSON. Got: {}", json_str),
            raw_output: raw_output.to_string(),
        });
    }

    Ok(json_str.to_string())
}
