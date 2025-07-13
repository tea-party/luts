use luts_llm::tools::AiTool;
use luts_memory::{MemoryManager, MemoryContent, BlockId};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// Tool for updating an existing memory block's content
pub struct UpdateBlockTool {
    pub memory_manager: Arc<MemoryManager>,
}

#[async_trait]
impl AiTool for UpdateBlockTool {
    fn name(&self) -> &str {
        "update_block"
    }

    fn description(&self) -> &str {
        "Updates the content of an existing memory block by its ID. Useful for correcting or expanding stored information."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "block_id": {
                    "type": "string",
                    "description": "The ID of the memory block to update"
                },
                "content": {
                    "type": "string", 
                    "description": "The new content to replace the existing content"
                }
            },
            "required": ["block_id", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, Error> {
        let block_id = params
            .get("block_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing block_id"))?;
        
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing content"))?;

        let block_id = BlockId::new(block_id);
        
        // Get the existing block
        let mut block = self.memory_manager.get(&block_id).await?
            .ok_or_else(|| anyhow!("Block not found: {}", block_id))?;
        
        // Update the content
        block.set_content(MemoryContent::Text(content.to_string()));
        
        // Store the updated block
        self.memory_manager.store(block).await?;

        Ok(json!({ 
            "success": true,
            "message": format!("Updated block {}", block_id),
            "block_id": block_id.as_str()
        }))
    }
}