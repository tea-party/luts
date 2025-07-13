use luts_llm::tools::AiTool;
use luts_memory::{MemoryManager, BlockId};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// Tool for deleting a memory block
pub struct DeleteBlockTool {
    pub memory_manager: Arc<MemoryManager>,
}

#[async_trait]
impl AiTool for DeleteBlockTool {
    fn name(&self) -> &str {
        "delete_block"
    }

    fn description(&self) -> &str {
        "Deletes a memory block by its ID. Use with caution - this permanently removes the block from storage."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "block_id": {
                    "type": "string",
                    "description": "The ID of the memory block to delete"
                }
            },
            "required": ["block_id"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, Error> {
        let block_id = params
            .get("block_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing block_id"))?;

        let block_id = BlockId::new(block_id);
        
        // Check if the block exists before deleting
        let block_exists = self.memory_manager.get(&block_id).await?.is_some();
        
        if !block_exists {
            return Err(anyhow!("Block not found: {}", block_id));
        }
        
        // Delete the block
        self.memory_manager.delete(&block_id).await?;

        Ok(json!({ 
            "success": true,
            "message": format!("Deleted block {}", block_id),
            "block_id": block_id.as_str()
        }))
    }
}