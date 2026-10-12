//! External process / IPC plugin runner.
//! Enables running external formatters, linters, or language tools via standard stdio pipes.
//! Zero third-party dependencies, purely standard library Rust.

use std::io::Write;
use std::process::{Command, Stdio};

pub struct ProcessPluginRunner;

impl ProcessPluginRunner {
    /// Pipes `input` into an external command's stdin and captures its stdout.
    /// Returns Ok(stdout) if exit status is success, otherwise Err(stderr).
    pub fn pipe_through_command(cmd: &str, args: &[&str], input: &str) -> Result<String, String> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn process '{}': {}", cmd, e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .map_err(|e| format!("Failed to write to stdin of '{}': {}", cmd, e))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for process '{}': {}", cmd, e))?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| format!("Command output was not valid UTF-8: {}", e))
        } else {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "Command '{}' failed with code {:?}: {}",
                cmd,
                output.status.code(),
                stderr_str
            ))
        }
    }
}
