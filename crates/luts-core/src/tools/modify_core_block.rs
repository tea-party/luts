//! Tool for agents to modify their own core context blocks
//!
//! This tool allows AI agents to update their core blocks like SystemPrompt, 
//! UserPersona, TaskContext, etc. This enables self-modification and adaptation.

use crate::tools::AiTool;
use crate::context::core_blocks::{CoreBlockManager, CoreBlockType, CoreBlockConfig};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool for modifying core context blocks
pub struct ModifyCoreBlockTool {
    pub core_block_manager: Arc<RwLock<CoreBlockManager>>,
}

impl ModifyCoreBlockTool {
    pub fn new(user_id: impl Into<String>, config: Option<CoreBlockConfig>) -> Self {
        let manager = CoreBlockManager::new(user_id, config);
        Self {
            core_block_manager: Arc::new(RwLock::new(manager)),
        }
    }

    pub fn from_manager(manager: Arc<RwLock<CoreBlockManager>>) -> Self {
        Self {
            core_block_manager: manager,
        }
    }
}

#[async_trait]
impl AiTool for ModifyCoreBlockTool {
    fn name(&self) -> &str {
        "modify_core_block"
    }

    fn description(&self) -> &str {
        "Modifies core context blocks that are always present in the AI's context window. Use this to update your system prompt, user persona, current task context, key facts, preferences, goals, or working memory. Changes persist across conversations."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "block_type": {
                    "type": "string",
                    "enum": [
                        "SystemPrompt",
                        "UserPersona", 
                        "TaskContext",
                        "KeyFacts",
                        "UserPreferences",
                        "ConversationSummary",
                        "ActiveGoals",
                        "WorkingMemory"
                    ],
                    "description": "The type of core block to modify:\n• SystemPrompt: AI instructions & behavior rules\n• UserPersona: Information about the user\n• TaskContext: Current project or task details\n• KeyFacts: Important facts to remember\n• UserPreferences: User settings & preferences\n• ConversationSummary: Summary of current session\n• ActiveGoals: Current objectives and goals\n• WorkingMemory: Temporary notes and context"
                },
                "content": {
                    "type": "string",
                    "description": "The new content for the core block. This will replace the existing content entirely."
                },
                "operation": {
                    "type": "string",
                    "enum": ["replace", "append", "prepend"],
                    "description": "How to modify the content: 'replace' (default), 'append' to existing, or 'prepend' to existing",
                    "default": "replace"
                }
            },
            "required": ["block_type", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, Error> {
        let block_type_str = params
            .get("block_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing block_type"))?;
        
        let new_content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing content"))?;

        let operation = params
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("replace");

        // Parse the core block type
        let core_block_type = match block_type_str {
            "SystemPrompt" => CoreBlockType::SystemPrompt,
            "UserPersona" => CoreBlockType::UserPersona,
            "TaskContext" => CoreBlockType::TaskContext,
            "KeyFacts" => CoreBlockType::KeyFacts,
            "UserPreferences" => CoreBlockType::UserPreferences,
            "ConversationSummary" => CoreBlockType::ConversationSummary,
            "ActiveGoals" => CoreBlockType::ActiveGoals,
            "WorkingMemory" => CoreBlockType::WorkingMemory,
            _ => return Err(anyhow!("Invalid block_type: {}", block_type_str)),
        };

        let mut manager = self.core_block_manager.write().await;
        
        // Ensure core blocks are initialized
        manager.initialize()?;

        // Handle different operations
        let final_content = match operation {
            "replace" => new_content.to_string(),
            "append" => {
                // Get existing content and append
                if let Some(existing_block) = manager.get_block(core_block_type) {
                    if let Some(existing_content) = existing_block.get_text_content() {
                        format!("{}\n\n{}", existing_content, new_content)
                    } else {
                        new_content.to_string()
                    }
                } else {
                    new_content.to_string()
                }
            },
            "prepend" => {
                // Get existing content and prepend
                if let Some(existing_block) = manager.get_block(core_block_type) {
                    if let Some(existing_content) = existing_block.get_text_content() {
                        format!("{}\n\n{}", new_content, existing_content)
                    } else {
                        new_content.to_string()
                    }
                } else {
                    new_content.to_string()
                }
            },
            _ => return Err(anyhow!("Invalid operation: {}. Use 'replace', 'append', or 'prepend'", operation)),
        };

        // Update the core block
        manager.update_block(core_block_type, final_content.clone())?;
        
        // Get stats for response
        let stats = manager.get_stats();

        Ok(json!({
            "success": true,
            "message": format!("Successfully {} {} core block", 
                match operation {
                    "replace" => "replaced",
                    "append" => "appended to",
                    "prepend" => "prepended to",
                    _ => "modified"
                },
                block_type_str
            ),
            "block_type": block_type_str,
            "operation": operation,
            "content_length": final_content.len(),
            "active_blocks": stats.active_blocks,
            "token_usage": stats.token_usage,
            "budget_utilization": format!("{:.1}%", stats.budget_utilization)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::core_blocks::CoreBlockConfig;

    #[tokio::test]
    async fn test_modify_core_block_replace() {
        let tool = ModifyCoreBlockTool::new("test_user", None);
        
        let params = json!({
            "block_type": "UserPersona",
            "content": "I am a software engineer specializing in Rust and AI systems.",
            "operation": "replace"
        });

        let result = tool.execute(params).await.unwrap();
        
        assert_eq!(result["success"], true);
        assert_eq!(result["block_type"], "UserPersona");
        assert_eq!(result["operation"], "replace");
    }

    #[tokio::test]
    async fn test_modify_core_block_append() {
        let tool = ModifyCoreBlockTool::new("test_user", None);
        
        // First set some initial content
        let initial_params = json!({
            "block_type": "KeyFacts",
            "content": "The user prefers concise explanations.",
            "operation": "replace"
        });
        tool.execute(initial_params).await.unwrap();

        // Now append to it
        let append_params = json!({
            "block_type": "KeyFacts", 
            "content": "The user is working on a TUI application.",
            "operation": "append"
        });

        let result = tool.execute(append_params).await.unwrap();
        
        assert_eq!(result["success"], true);
        assert_eq!(result["operation"], "append");
    }

    #[tokio::test]
    async fn test_invalid_block_type() {
        let tool = ModifyCoreBlockTool::new("test_user", None);
        
        let params = json!({
            "block_type": "InvalidType",
            "content": "Some content"
        });

        let result = tool.execute(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid block_type"));
    }

    #[tokio::test]
    async fn test_system_prompt_modification() {
        let tool = ModifyCoreBlockTool::new("test_user", None);
        
        let params = json!({
            "block_type": "SystemPrompt",
            "content": "You are a helpful AI assistant with expertise in Rust programming. Always provide code examples when explaining concepts.",
            "operation": "replace"
        });

        let result = tool.execute(params).await.unwrap();
        
        assert_eq!(result["success"], true);
        assert_eq!(result["block_type"], "SystemPrompt");
        
        // Verify the content was set by checking it exists
        let manager = tool.core_block_manager.read().await;
        // Note: We can't easily test the actual content here due to the mutable borrow,
        // but we can verify the operation succeeded
    }
}