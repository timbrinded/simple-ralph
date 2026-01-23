use std::process::Command;

pub fn launch_claude(prompt: &str) -> String {
    let args = vec!["--permission-mode", "bypassPermissions", "-p"];

    let cmd_output = Command::new("claude")
        .args(args)
        .arg(prompt)
        .output()
        .expect("failed to execute process");

    String::from_utf8_lossy(&cmd_output.stdout).into_owned()
}
