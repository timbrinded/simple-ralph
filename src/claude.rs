use std::process::{Command, Stdio};

pub fn launch_claude(prompt: &str) -> std::process::Child {
    let args = vec!["--permission-mode", "bypassPermissions", "-p"];

    Command::new("claude")
        .args(args)
        .arg(prompt)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Error spawning claude code!")
}
