use std::process::Command;

pub fn launch_claude(prompt: &str) -> std::process::Child {
    let args = vec!["--permission-mode", "bypassPermissions", "-p"];

    Command::new("claude")
        .args(args)
        .arg(prompt)
        .spawn()
        .expect("Error spawning claude code!")
}
