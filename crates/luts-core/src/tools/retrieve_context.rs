use crate::memory::{BlockType, MemoryManager, MemoryQuery};
use crate::tools::AiTool;
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// Tool for retrieving relevant memory blocks from the MemoryManager.
pub struct RetrieveContextTool {
    pub memory_manager: Arc<MemoryManager>,
}

#[async_trait]
impl AiTool for RetrieveContextTool {
    fn name(&self) -> &str {
        "retrieve_context"
    }

    fn description(&self) -> &str {
        "Fetches relevant memory blocks for the conversation, given user/session/content parameters."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "user_id": { "type": "string" },
                "session_id": { "type": "string" },
                "content_query": { "type": "string" },
                "block_types": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["Fact", "Message", "Summary", "Preference", "PersonalInfo", "Goal", "Task"] }
                },
                "limit": { "type": "integer" }
            },
            "required": ["user_id"]
        })
    }

    async fn execute(&self, params: Value) -> Result<Value, Error> {
        let user_id = params
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing user_id"))?;
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let content_query = params
            .get("content_query")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        let block_types = params
            .get("block_types")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .filter_map(|s| match s {
                        "Fact" => Some(BlockType::Fact),
                        "Message" => Some(BlockType::Message),
                        "Summary" => Some(BlockType::Summary),
                        "Preference" => Some(BlockType::Preference),
                        "PersonalInfo" => Some(BlockType::PersonalInfo),
                        "Goal" => Some(BlockType::Goal),
                        "Task" => Some(BlockType::Task),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            });

        let query = MemoryQuery {
            user_id: Some(user_id.to_string()),
            session_id,
            content_contains: content_query,
            block_types: block_types.unwrap_or_default(),
            limit,
            ..Default::default()
        };

        let blocks = self.memory_manager.search(&query).await?;
        let blocks_json: Vec<Value> = blocks
            .iter()
            .map(|b| {
                json!({
                    "id": b.id(),
                    "type": b.block_type(),
                    "content": b.content(),
                    "created_at": b.created_at(),
                })
            })
            .collect();

        Ok(json!({ "blocks": blocks_json }))
    }
}
