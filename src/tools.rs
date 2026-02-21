use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Tool definition
// ---------------------------------------------------------------------------

/// Represents a tool invocation parsed from an API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "input")]
pub enum Tool {
    #[serde(rename = "read_file")]
    ReadFile { path: String },

    #[serde(rename = "write_file")]
    WriteFile { path: String, content: String },

    #[serde(rename = "list_files")]
    ListFiles {
        path: String,
        pattern: Option<String>,
    },

    #[serde(rename = "search_files")]
    SearchFiles {
        pattern: String,
        path: Option<String>,
    },

    #[serde(rename = "execute")]
    Execute { command: String },

    #[serde(rename = "edit_file")]
    EditFile {
        path: String,
        old_text: String,
        new_text: String,
    },
}

impl Tool {
    /// Human-readable name used for permission checks and display.
    pub fn name(&self) -> &'static str {
        match self {
            Tool::ReadFile { .. } => "read_file",
            Tool::WriteFile { .. } => "write_file",
            Tool::ListFiles { .. } => "list_files",
            Tool::SearchFiles { .. } => "search_files",
            Tool::Execute { .. } => "execute",
            Tool::EditFile { .. } => "edit_file",
        }
    }
}

// ---------------------------------------------------------------------------
// Tool result
// ---------------------------------------------------------------------------

/// The outcome of running a single tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
        }
    }

    pub fn err(output: impl Into<String>) -> Self {
        Self {
            success: false,
            output: output.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool permissions
// ---------------------------------------------------------------------------

/// Controls whether a tool may run without user confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolPermission {
    /// Run immediately without asking.
    AutoAllow,
    /// Prompt the user before executing.
    AskFirst,
    /// Never allow execution.
    Deny,
}

impl Default for ToolPermission {
    fn default() -> Self {
        Self::AskFirst
    }
}

// ---------------------------------------------------------------------------
// Tool executor
// ---------------------------------------------------------------------------

/// Maximum wall-clock time for a shell command before it is killed.
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);

/// Executes tools against the local filesystem and shell.
pub struct ToolExecutor {
    /// Per-tool permission overrides.  Keys are tool names as returned by
    /// [`Tool::name`].  Any tool not present falls back to [`ToolPermission::AskFirst`].
    permissions: HashMap<String, ToolPermission>,

    /// Maximum duration for shell commands.
    command_timeout: Duration,
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            permissions: HashMap::new(),
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
        }
    }

    // -- configuration helpers ------------------------------------------------

    pub fn set_permission(&mut self, tool_name: &str, perm: ToolPermission) {
        self.permissions.insert(tool_name.to_string(), perm);
    }

    pub fn permission(&self, tool_name: &str) -> ToolPermission {
        self.permissions
            .get(tool_name)
            .copied()
            .unwrap_or_default()
    }

    pub fn set_command_timeout(&mut self, timeout: Duration) {
        self.command_timeout = timeout;
    }

    // -- execution ------------------------------------------------------------

    /// Execute a tool, returning the result.
    ///
    /// The caller is responsible for checking [`ToolPermission`] *before*
    /// calling this method.
    pub fn execute(&self, tool: &Tool) -> ToolResult {
        match tool {
            Tool::ReadFile { path } => self.read_file(path),
            Tool::WriteFile { path, content } => self.write_file(path, content),
            Tool::ListFiles { path, pattern } => self.list_files(path, pattern.as_deref()),
            Tool::SearchFiles { pattern, path } => self.search_files(pattern, path.as_deref()),
            Tool::Execute { command } => self.execute_command(command),
            Tool::EditFile {
                path,
                old_text,
                new_text,
            } => self.edit_file(path, old_text, new_text),
        }
    }

    // -- individual tool implementations --------------------------------------

    fn read_file(&self, path: &str) -> ToolResult {
        let path = Path::new(path);
        if !path.exists() {
            return ToolResult::err(format!("File not found: {}", path.display()));
        }
        match fs::read_to_string(path) {
            Ok(contents) => {
                let numbered: String = contents
                    .lines()
                    .enumerate()
                    .map(|(i, line)| format!("{:>6}\t{}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n");
                ToolResult::ok(numbered)
            }
            Err(e) => ToolResult::err(format!("Failed to read {}: {e}", path.display())),
        }
    }

    fn write_file(&self, path: &str, content: &str) -> ToolResult {
        let path = Path::new(path);

        // Create parent directories if they don't exist.
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return ToolResult::err(format!(
                        "Failed to create directory {}: {e}",
                        parent.display()
                    ));
                }
            }
        }

        let mut file = match fs::File::create(path) {
            Ok(f) => f,
            Err(e) => {
                return ToolResult::err(format!("Failed to create {}: {e}", path.display()));
            }
        };

        match file.write_all(content.as_bytes()) {
            Ok(()) => ToolResult::ok(format!(
                "Wrote {} bytes to {}",
                content.len(),
                path.display()
            )),
            Err(e) => ToolResult::err(format!("Failed to write {}: {e}", path.display())),
        }
    }

    fn list_files(&self, path: &str, pattern: Option<&str>) -> ToolResult {
        let base = PathBuf::from(path);
        if !base.exists() {
            return ToolResult::err(format!("Directory not found: {}", base.display()));
        }

        let glob_pattern = match pattern {
            Some(p) => format!("{}/{p}", base.display()),
            None => format!("{}/**/*", base.display()),
        };

        match glob::glob(&glob_pattern) {
            Ok(entries) => {
                let mut files: Vec<String> = Vec::new();
                for entry in entries {
                    match entry {
                        Ok(p) => files.push(p.display().to_string()),
                        Err(e) => files.push(format!("(error: {e})")),
                    }
                }
                if files.is_empty() {
                    ToolResult::ok("No files matched the pattern.")
                } else {
                    ToolResult::ok(files.join("\n"))
                }
            }
            Err(e) => ToolResult::err(format!("Invalid glob pattern: {e}")),
        }
    }

    fn search_files(&self, pattern: &str, path: Option<&str>) -> ToolResult {
        let search_path = path.unwrap_or(".");

        // Try ripgrep first, fall back to grep.
        let (program, args) = if command_exists("rg") {
            (
                "rg",
                vec![
                    "--line-number".to_string(),
                    "--no-heading".to_string(),
                    "--color=never".to_string(),
                    pattern.to_string(),
                    search_path.to_string(),
                ],
            )
        } else {
            (
                "grep",
                vec![
                    "-rn".to_string(),
                    "--color=never".to_string(),
                    pattern.to_string(),
                    search_path.to_string(),
                ],
            )
        };

        match Command::new(program).args(&args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                if output.status.success() || !stdout.is_empty() {
                    ToolResult::ok(if stdout.is_empty() {
                        "No matches found.".to_string()
                    } else {
                        stdout
                    })
                } else {
                    // grep/rg exit 1 when no matches; treat that as success with
                    // an informational message.
                    if output.status.code() == Some(1) && stderr.is_empty() {
                        ToolResult::ok("No matches found.")
                    } else {
                        ToolResult::err(format!("{program} error: {stderr}"))
                    }
                }
            }
            Err(e) => ToolResult::err(format!("Failed to run {program}: {e}")),
        }
    }

    fn execute_command(&self, command: &str) -> ToolResult {
        use std::process::Stdio;

        let mut child = match Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return ToolResult::err(format!("Failed to spawn command: {e}")),
        };

        // Wait with timeout.
        let result = wait_with_timeout(&mut child, self.command_timeout);

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);

                let mut combined = String::new();
                if !stdout.is_empty() {
                    combined.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !combined.is_empty() {
                        combined.push('\n');
                    }
                    combined.push_str("[stderr]\n");
                    combined.push_str(&stderr);
                }
                if combined.is_empty() {
                    combined = "(no output)".to_string();
                }

                if output.status.success() {
                    ToolResult::ok(combined)
                } else {
                    ToolResult::err(format!("Exit code {code}\n{combined}"))
                }
            }
            Err(msg) => ToolResult::err(msg),
        }
    }

    fn edit_file(&self, path: &str, old_text: &str, new_text: &str) -> ToolResult {
        let file_path = Path::new(path);
        if !file_path.exists() {
            return ToolResult::err(format!("File not found: {}", file_path.display()));
        }

        let contents = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult::err(format!("Failed to read {}: {e}", file_path.display()));
            }
        };

        let count = contents.matches(old_text).count();
        if count == 0 {
            return ToolResult::err(format!(
                "old_text not found in {}",
                file_path.display()
            ));
        }
        if count > 1 {
            return ToolResult::err(format!(
                "old_text matches {count} locations in {} -- provide more context to make it unique",
                file_path.display()
            ));
        }

        let new_contents = contents.replacen(old_text, new_text, 1);
        match fs::write(file_path, &new_contents) {
            Ok(()) => ToolResult::ok(format!(
                "Applied edit to {} (replaced 1 occurrence)",
                file_path.display()
            )),
            Err(e) => ToolResult::err(format!("Failed to write {}: {e}", file_path.display())),
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing tool calls from the Anthropic API response
// ---------------------------------------------------------------------------

/// A parsed tool-use block from an Anthropic Messages API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The unique id assigned by the API (used when sending tool results back).
    pub id: String,
    /// The resolved tool variant.
    pub tool: Tool,
}

/// Parse tool-use content blocks from an Anthropic API response body.
///
/// The Anthropic Messages API returns content as an array of typed blocks.
/// Blocks with `"type": "tool_use"` contain `id`, `name`, and `input` fields.
///
/// ```json
/// {
///   "content": [
///     { "type": "text", "text": "Let me read that file." },
///     { "type": "tool_use", "id": "toolu_01A...", "name": "read_file", "input": { "path": "src/main.rs" } }
///   ]
/// }
/// ```
pub fn parse_tool_calls(response: &Value) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    let content = match response.get("content").and_then(|c| c.as_array()) {
        Some(arr) => arr,
        None => return calls,
    };

    for block in content {
        if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
            continue;
        }

        let id = match block.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        let name = match block.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => continue,
        };

        let input = match block.get("input") {
            Some(v) => v.clone(),
            None => Value::Object(serde_json::Map::new()),
        };

        let tool = match name {
            "read_file" => {
                let path = input
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Tool::ReadFile { path }
            }
            "write_file" => {
                let path = input
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let file_content = input
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Tool::WriteFile {
                    path,
                    content: file_content,
                }
            }
            "list_files" => {
                let path = input
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".")
                    .to_string();
                let pattern = input
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Tool::ListFiles { path, pattern }
            }
            "search_files" => {
                let pattern = input
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let path = input
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Tool::SearchFiles { pattern, path }
            }
            "execute" => {
                let command = input
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Tool::Execute { command }
            }
            "edit_file" => {
                let path = input
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let old_text = input
                    .get("old_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let new_text = input
                    .get("new_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Tool::EditFile {
                    path,
                    old_text,
                    new_text,
                }
            }
            _ => continue, // unknown tool -- skip
        };

        calls.push(ToolCall { id, tool });
    }

    calls
}

// ---------------------------------------------------------------------------
// Formatting tool definitions for the Anthropic API
// ---------------------------------------------------------------------------

/// Return the tool definitions array suitable for inclusion in an Anthropic
/// Messages API request body under the `"tools"` key.
pub fn format_tool_definitions() -> Value {
    json!([
        {
            "name": "read_file",
            "description": "Read the contents of a file at the given path. Returns the file contents with line numbers.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or relative path to the file to read."
                    }
                },
                "required": ["path"]
            }
        },
        {
            "name": "write_file",
            "description": "Write content to a file at the given path. Creates parent directories if they do not exist. Overwrites the file if it already exists.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute or relative path to the file to write."
                    },
                    "content": {
                        "type": "string",
                        "description": "The full content to write to the file."
                    }
                },
                "required": ["path", "content"]
            }
        },
        {
            "name": "list_files",
            "description": "List files in a directory, optionally filtered by a glob pattern.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to list files in."
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Optional glob pattern to filter files (e.g. \"**/*.rs\"). If omitted, all files are listed recursively."
                    }
                },
                "required": ["path"]
            }
        },
        {
            "name": "search_files",
            "description": "Search file contents using a regular expression pattern (via ripgrep or grep). Returns matching lines with file paths and line numbers.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regular expression pattern to search for."
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional directory or file to search in. Defaults to the current directory."
                    }
                },
                "required": ["pattern"]
            }
        },
        {
            "name": "execute",
            "description": "Execute a shell command and return its stdout and stderr. The command runs under `sh -c` with a configurable timeout (default 120 seconds).",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "edit_file",
            "description": "Perform a precise string replacement in a file. The old_text must appear exactly once in the file; it will be replaced with new_text.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit."
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The exact text to find (must be unique in the file)."
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The text to replace old_text with."
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }
        }
    ])
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check whether a command is available on the system PATH.
fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Wait for a child process with a timeout, killing it if it exceeds the limit.
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    use std::thread;
    use std::time::Instant;

    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                // Child has exited; collect output.
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(ref mut out) = child.stdout {
                    std::io::Read::read_to_end(out, &mut stdout)
                        .unwrap_or_default();
                }
                if let Some(ref mut err) = child.stderr {
                    std::io::Read::read_to_end(err, &mut stderr)
                        .unwrap_or_default();
                }
                return Ok(std::process::Output {
                    status: _status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                // Still running.
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait(); // reap
                    return Err(format!(
                        "Command timed out after {} seconds",
                        timeout.as_secs()
                    ));
                }
                thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(format!("Error waiting for process: {e}"));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn test_parse_tool_calls_read_file() {
        let response = json!({
            "content": [
                { "type": "text", "text": "Reading the file now." },
                {
                    "type": "tool_use",
                    "id": "toolu_123",
                    "name": "read_file",
                    "input": { "path": "/tmp/test.txt" }
                }
            ]
        });

        let calls = parse_tool_calls(&response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "toolu_123");
        assert!(matches!(&calls[0].tool, Tool::ReadFile { path } if path == "/tmp/test.txt"));
    }

    #[test]
    fn test_parse_tool_calls_multiple() {
        let response = json!({
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_1",
                    "name": "read_file",
                    "input": { "path": "a.txt" }
                },
                {
                    "type": "tool_use",
                    "id": "toolu_2",
                    "name": "execute",
                    "input": { "command": "ls" }
                }
            ]
        });

        let calls = parse_tool_calls(&response);
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn test_parse_tool_calls_empty() {
        let response = json!({
            "content": [
                { "type": "text", "text": "No tools here." }
            ]
        });
        let calls = parse_tool_calls(&response);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_format_tool_definitions_is_array() {
        let defs = format_tool_definitions();
        assert!(defs.is_array());
        assert_eq!(defs.as_array().unwrap().len(), 6);
    }

    #[test]
    fn test_read_file_not_found() {
        let executor = ToolExecutor::new();
        let result = executor.execute(&Tool::ReadFile {
            path: "/tmp/__nonexistent_pro_chat_test__".into(),
        });
        assert!(!result.success);
        assert!(result.output.contains("not found"));
    }

    #[test]
    fn test_write_and_read_file() {
        let dir = std::env::temp_dir().join("pro_chat_test_write_read");
        let _ = fs::remove_dir_all(&dir);

        let file_path = dir.join("hello.txt");
        let executor = ToolExecutor::new();

        let write_result = executor.execute(&Tool::WriteFile {
            path: file_path.display().to_string(),
            content: "line one\nline two\n".into(),
        });
        assert!(write_result.success);

        let read_result = executor.execute(&Tool::ReadFile {
            path: file_path.display().to_string(),
        });
        assert!(read_result.success);
        assert!(read_result.output.contains("line one"));
        assert!(read_result.output.contains("line two"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_edit_file() {
        let dir = std::env::temp_dir().join("pro_chat_test_edit");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let file_path = dir.join("edit_me.txt");
        fs::write(&file_path, "Hello world\nFoo bar\n").unwrap();

        let executor = ToolExecutor::new();
        let result = executor.execute(&Tool::EditFile {
            path: file_path.display().to_string(),
            old_text: "Foo bar".into(),
            new_text: "Baz qux".into(),
        });
        assert!(result.success);

        let contents = fs::read_to_string(&file_path).unwrap();
        assert!(contents.contains("Baz qux"));
        assert!(!contents.contains("Foo bar"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_edit_file_not_found_text() {
        let dir = std::env::temp_dir().join("pro_chat_test_edit_nf");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let file_path = dir.join("nf.txt");
        fs::write(&file_path, "aaa\n").unwrap();

        let executor = ToolExecutor::new();
        let result = executor.execute(&Tool::EditFile {
            path: file_path.display().to_string(),
            old_text: "zzz".into(),
            new_text: "yyy".into(),
        });
        assert!(!result.success);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_execute_command() {
        let executor = ToolExecutor::new();
        let result = executor.execute(&Tool::Execute {
            command: "echo hello".into(),
        });
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn test_permission_defaults() {
        let executor = ToolExecutor::new();
        assert_eq!(executor.permission("read_file"), ToolPermission::AskFirst);
        assert_eq!(executor.permission("execute"), ToolPermission::AskFirst);
    }

    #[test]
    fn test_permission_override() {
        let mut executor = ToolExecutor::new();
        executor.set_permission("read_file", ToolPermission::AutoAllow);
        assert_eq!(executor.permission("read_file"), ToolPermission::AutoAllow);
        assert_eq!(executor.permission("execute"), ToolPermission::AskFirst);
    }

    #[test]
    fn test_tool_name() {
        assert_eq!(
            Tool::ReadFile {
                path: String::new()
            }
            .name(),
            "read_file"
        );
        assert_eq!(
            Tool::Execute {
                command: String::new()
            }
            .name(),
            "execute"
        );
    }
}
