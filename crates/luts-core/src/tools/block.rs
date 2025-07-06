use crate::tools::AiTool;
use crate::memory::{MemoryManager, MemoryBlockBuilder, MemoryContent, BlockType};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// Tool for creating and storing a new memory block (fact, message, summary, etc.)
pub struct BlockTool {
    pub memory_manager: Arc<MemoryManager>,
}

#[async_trait]
impl AiTool for BlockTool {
    fn name(&self) -> &str {
        "block"
    }

    fn description(&self) -> &str {
        "Creates and stores a new memory block (e.g., fact, message, summary) for the user/session."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "user_id": { "type": "string" },
                "session_id": { "type": "string" },
                "block_type": { 
                    "type": "string", 
                    "enum": ["Fact", "Message", "Summary", "Preference", "PersonalInfo", "Goal", "Task"],
                    "description": "Type of memory block: Fact (long-term knowledge), Message (conversation), Summary (condensed info), Preference (user settings), PersonalInfo (about user), Goal (objectives), Task (actions)"
                },
                "content": { "type": "string", "description": "The content to store in the memory block" },
                "tags": { 
                    "type": "array", 
                    "items": { "type": "string" },
                    "description": "Optional tags to categorize the block"
                }
            },
            "required": ["user_id", "block_type", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, Error> {
        let user_id = params.get("user_id").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing user_id"))?;
        let session_id = params.get("session_id").and_then(|v| v.as_str());
        let block_type = params.get("block_type").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing block_type"))?;
        let content = params.get("content").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing content"))?;
        let tags = params.get("tags").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
            .unwrap_or_default();

        let block_type = match block_type {
            "Fact" => BlockType::Fact,
            "Message" => BlockType::Message,
            "Summary" => BlockType::Summary,
            "Preference" => BlockType::Preference,
            "PersonalInfo" => BlockType::PersonalInfo,
            "Goal" => BlockType::Goal,
            "Task" => BlockType::Task,
            _ => return Err(anyhow!("Invalid block_type: {}", block_type)),
        };

        let mut builder = MemoryBlockBuilder::default()
            .with_user_id(user_id)
            .with_type(block_type)
            .with_content(MemoryContent::Text(content.to_string()));

        if let Some(session_id) = session_id {
            builder = builder.with_session_id(session_id);
        }

        // Add tags if provided
        for tag in tags {
            builder = builder.with_tag(tag);
        }

        let block = builder.build()?;
        let block_id = self.memory_manager.store(block).await?;

        Ok(json!({ 
            "success": true,
            "block_id": block_id.as_str(),
            "message": format!("Created {} block with ID {}", block_type, block_id)
        }))
    }
}
