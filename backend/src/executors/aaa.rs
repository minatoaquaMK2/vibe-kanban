use std::path::Path;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    command_runner::{CommandProcess, CommandRunner},
    executor::{
        ActionType, Executor, ExecutorError, NormalizedConversation, NormalizedEntry,
        NormalizedEntryType,
    },
    models::task::Task,
    utils::shell::get_shell_command,
};

/// An executor that uses AAA (Assistant Agent) CLI to process tasks
pub struct AaaExecutor {
    executor_type: String,
    command: String,
}

impl Default for AaaExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl AaaExecutor {
    /// Create a new AaaExecutor with default settings
    pub fn new() -> Self {
        Self {
            executor_type: "AAA".to_string(),
            command: "aaa".to_string(),
        }
    }

    /// Create a new AaaExecutor with custom settings
    pub fn with_command(executor_type: String, command: String) -> Self {
        Self {
            executor_type,
            command,
        }
    }
}

#[async_trait]
impl Executor for AaaExecutor {
    async fn spawn(
        &self,
        pool: &sqlx::SqlitePool,
        task_id: Uuid,
        worktree_path: &str,
    ) -> Result<CommandProcess, ExecutorError> {
        // Get the task to fetch its description
        let task = Task::find_by_id(pool, task_id)
            .await?
            .ok_or(ExecutorError::TaskNotFound)?;

        let problem_statement = if let Some(task_description) = task.description {
            format!(
                "Task: {} - Description: {} - Please help me implement this task in the codebase. Analyze the current code structure and make the necessary changes to fulfill the requirements.",
                task.title, task_description
            )
        } else {
            format!(
                "Task: {} - Please help me implement this task in the codebase. Analyze the current code structure and make the necessary changes to fulfill the requirements.",
                task.title
            )
        };

        // Build AAA command arguments for headless mode
        let mut command = CommandRunner::new();
        command
            .command(&self.command)
            .arg("--workspace")
            .arg(worktree_path)
            .arg("--problem-statement")
            .arg(&problem_statement)
            .arg("--minimize-stdout-logs")
            .working_dir(worktree_path)
            .env("NODE_NO_WARNINGS", "1");

        let proc = command.start().await.map_err(|e| {
            crate::executor::SpawnContext::from_command(&command, &self.executor_type)
                .with_task(task_id, Some(task.title.clone()))
                .with_context(format!("{} CLI execution for new task", self.executor_type))
                .spawn_error(e)
        })?;
        Ok(proc)
    }

    async fn spawn_followup(
        &self,
        _pool: &sqlx::SqlitePool,
        _task_id: Uuid,
        _session_id: &str,
        prompt: &str,
        worktree_path: &str,
    ) -> Result<CommandProcess, ExecutorError> {
        // For follow-up, use interactive mode with the prompt
        let mut command = CommandRunner::new();
        command
            .command(&self.command)
            .arg("--workspace")
            .arg(worktree_path)
            .arg("--minimize-stdout-logs")
            .stdin(prompt)
            .working_dir(worktree_path)
            .env("NODE_NO_WARNINGS", "1");

        let proc = command.start().await.map_err(|e| {
            crate::executor::SpawnContext::from_command(&command, &self.executor_type)
                .with_context(format!(
                    "{} CLI followup execution",
                    self.executor_type
                ))
                .spawn_error(e)
        })?;

        Ok(proc)
    }

    fn normalize_logs(
        &self,
        logs: &str,
        worktree_path: &str,
    ) -> Result<NormalizedConversation, String> {
        let mut entries = Vec::new();
        let session_id = None; // AAA doesn't use session IDs like Claude

        for line in logs.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // AAA outputs are typically plain text, so we'll categorize them based on content
            let entry_type = if trimmed.starts_with("Error:") || trimmed.starts_with("❌") {
                NormalizedEntryType::SystemMessage
            } else if trimmed.starts_with("✅") || trimmed.starts_with("🚀") || trimmed.starts_with("📦") {
                NormalizedEntryType::SystemMessage
            } else if trimmed.starts_with("User input:") || trimmed.starts_with("Enter your message:") {
                NormalizedEntryType::UserMessage
            } else if self.is_tool_usage(trimmed) {
                // Detect tool usage patterns
                let (tool_name, action_type) = self.extract_tool_info(trimmed, worktree_path);
                NormalizedEntryType::ToolUse { tool_name, action_type }
            } else {
                // Default to assistant message for most content
                NormalizedEntryType::AssistantMessage
            };

            entries.push(NormalizedEntry {
                timestamp: None,
                entry_type,
                content: trimmed.to_string(),
                metadata: None,
            });
        }

        Ok(NormalizedConversation {
            entries,
            session_id,
            executor_type: self.executor_type.clone(),
            prompt: None,
            summary: None,
        })
    }
}

impl AaaExecutor {
    /// Check if a line indicates tool usage
    fn is_tool_usage(&self, line: &str) -> bool {
        line.contains("Reading file:") ||
        line.contains("Writing file:") ||
        line.contains("Running command:") ||
        line.contains("Searching for:") ||
        line.contains("Creating task:") ||
        line.contains("Fetching URL:")
    }

    /// Extract tool information from a line
    fn extract_tool_info(&self, line: &str, worktree_path: &str) -> (String, ActionType) {
        if line.contains("Reading file:") {
            let path = self.extract_path_from_line(line, worktree_path);
            ("file_read".to_string(), ActionType::FileRead { path })
        } else if line.contains("Writing file:") {
            let path = self.extract_path_from_line(line, worktree_path);
            ("file_write".to_string(), ActionType::FileWrite { path })
        } else if line.contains("Running command:") {
            let command = self.extract_command_from_line(line);
            ("command_run".to_string(), ActionType::CommandRun { command })
        } else if line.contains("Searching for:") {
            let query = self.extract_query_from_line(line);
            ("search".to_string(), ActionType::Search { query })
        } else if line.contains("Creating task:") {
            let description = self.extract_description_from_line(line);
            ("task_create".to_string(), ActionType::TaskCreate { description })
        } else if line.contains("Fetching URL:") {
            let url = self.extract_url_from_line(line);
            ("web_fetch".to_string(), ActionType::WebFetch { url })
        } else {
            ("unknown".to_string(), ActionType::Other { description: line.to_string() })
        }
    }

    /// Extract file path from a log line
    fn extract_path_from_line(&self, line: &str, worktree_path: &str) -> String {
        // Simple extraction - look for path after colon
        if let Some(colon_pos) = line.find(':') {
            let path = line[colon_pos + 1..].trim();
            self.make_path_relative(path, worktree_path)
        } else {
            line.to_string()
        }
    }

    /// Extract command from a log line
    fn extract_command_from_line(&self, line: &str) -> String {
        if let Some(colon_pos) = line.find(':') {
            line[colon_pos + 1..].trim().to_string()
        } else {
            line.to_string()
        }
    }

    /// Extract search query from a log line
    fn extract_query_from_line(&self, line: &str) -> String {
        if let Some(colon_pos) = line.find(':') {
            line[colon_pos + 1..].trim().to_string()
        } else {
            line.to_string()
        }
    }

    /// Extract task description from a log line
    fn extract_description_from_line(&self, line: &str) -> String {
        if let Some(colon_pos) = line.find(':') {
            line[colon_pos + 1..].trim().to_string()
        } else {
            line.to_string()
        }
    }

    /// Extract URL from a log line
    fn extract_url_from_line(&self, line: &str) -> String {
        if let Some(colon_pos) = line.find(':') {
            line[colon_pos + 1..].trim().to_string()
        } else {
            line.to_string()
        }
    }

    /// Convert absolute paths to relative paths based on worktree path
    fn make_path_relative(&self, path: &str, worktree_path: &str) -> String {
        let path_obj = Path::new(path);
        let worktree_path_obj = Path::new(worktree_path);

        // If path is already relative, return as is
        if path_obj.is_relative() {
            return path.to_string();
        }

        // Try to make path relative to the worktree path
        match path_obj.strip_prefix(worktree_path_obj) {
            Ok(relative_path) => relative_path.to_string_lossy().to_string(),
            Err(_) => path.to_string(), // Return original if can't make relative
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_path_relative() {
        let executor = AaaExecutor::new();

        // Test with relative path (should remain unchanged)
        assert_eq!(
            executor.make_path_relative("src/main.rs", "/tmp/test-worktree"),
            "src/main.rs"
        );

        // Test with absolute path (should become relative if possible)
        let test_worktree = "/tmp/test-worktree";
        let absolute_path = format!("{}/src/main.rs", test_worktree);
        let result = executor.make_path_relative(&absolute_path, test_worktree);
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn test_is_tool_usage() {
        let executor = AaaExecutor::new();

        assert!(executor.is_tool_usage("Reading file: src/main.rs"));
        assert!(executor.is_tool_usage("Writing file: output.txt"));
        assert!(executor.is_tool_usage("Running command: npm install"));
        assert!(!executor.is_tool_usage("This is just a regular message"));
    }
}
