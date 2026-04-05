use std::path::Path;
use std::process::{Command, Stdio};

/// Dangerous command patterns that should be blocked
const BLOCKED_PATTERNS: &[&str] = &[
    // Recursive delete root/home
    "rm -rf /",
    "rm -rf /*",
    "rm -rf ~",
    "rm -rf ~/*",
    "rm -r /",
    "rm -r /*",
    "rm -r ~",
    "rm -f /",
    "rm -f /*",
    // Disk destruction
    "dd if=/dev/zero of=/dev/",
    "dd if=/dev/random of=/dev/",
    "dd if=/dev/urandom of=/dev/",
    "mkfs.",
    "shred /dev/",
    "cat /dev/urandom > /dev/",
    "cat /dev/zero > /dev/",
    "> /dev/sd",
    // Fork bomb
    ":(){ :|:& };:",
    "fork bomb",
    // Permission destruction
    "chmod -R 777 /",
    "chmod 777 /",
    "chown -R",
    // Reverse shell
    "bash -i >& /dev/tcp/",
    "nc -e /bin/",
    "nc -e /usr/bin/",
    "/dev/tcp/",
    // Remote code execution (pipe to shell)
    "| bash",
    "| sh",
    "| zsh",
    "| fish",
    "| sudo bash",
    "| sudo sh",
    // Firewall manipulation
    "iptables -F",
    "iptables --flush",
    // System file destruction
    "> /etc/passwd",
    "> /etc/shadow",
    "> /etc/sudoers",
    "mv / /dev/null",
    // Kernel parameter manipulation
    "sysctl -w",
];

/// Check if a command contains dangerous patterns
fn is_command_blocked(command: &str) -> Option<&'static str> {
    let normalized = command.to_lowercase();
    for pattern in BLOCKED_PATTERNS {
        if normalized.contains(&pattern.to_lowercase()) {
            return Some(*pattern);
        }
    }
    None
}

pub fn tool_exec(command: &str) -> String {
    if let Some(pattern) = is_command_blocked(command) {
        return format!("Error: Command blocked (matched dangerous pattern: \"{}\"). This operation is not allowed for safety reasons.", pattern);
    }
    run_command(command, None)
}

pub fn tool_exec_in_dir(command: &str, dir: &Path) -> String {
    if let Some(pattern) = is_command_blocked(command) {
        return format!("Error: Command blocked (matched dangerous pattern: \"{}\"). This operation is not allowed for safety reasons.", pattern);
    }
    run_command(command, Some(dir))
}

fn run_command(command: &str, working_dir: Option<&Path>) -> String {
    let (shell, flag) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut cmd = Command::new(shell);
    cmd.arg(flag).arg(command);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let output = match cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(o) => o,
        Err(e) => return format!("Failed to execute: {}", e),
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push_str("\n[stderr]\n");
        }
        result.push_str(&stderr);
    }
    if result.is_empty() {
        result = "(no output)".to_string();
    }

    if result.len() > 10000 {
        result.truncate(10000);
        result.push_str("\n... (truncated)");
    }

    result
}
