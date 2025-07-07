//! Tools module for AI assistants
//!
//! This module provides a collection of tools that can be used by AI assistants
//! to perform various tasks. The tools are implemented as traits that can be
//! registered with an LLM service.

pub mod block;
pub mod calc;
pub mod delete_block;
pub mod interactive_tester;
pub mod modify_core_block;
pub mod retrieve_context;
pub mod search;
pub mod semantic_search;
pub mod update_block;
pub mod website;

use anyhow::Error;
use async_trait::async_trait;
use serde_json::Value;

/// A tool that can be used by an AI assistant
#[async_trait]
pub trait AiTool: Send + Sync {
    /// The name of the tool
    fn name(&self) -> &str;

    /// A description of what the tool does
    fn description(&self) -> &str;

    /// The JSON schema for the tool's parameters
    fn schema(&self) -> Value;

    /// Execute the tool with the given parameters
    async fn execute(&self, params: Value) -> Result<Value, Error>;

    /// Validate the parameters against the schema
    fn validate_params(&self, _params: &Value) -> Result<(), Error> {
        // Default implementation that just passes validation
        // In a real implementation, this would validate against the schema
        Ok(())
    }
    /// Convert to a genai Tool
    fn to_genai_tool(&self) -> genai::chat::Tool {
        genai::chat::Tool::new(self.name())
            .with_description(self.description())
            .with_schema(self.schema())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct EchoTool;

    #[async_trait]
    impl AiTool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> &str {
            "Echoes back the input text"
        }

        fn schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to echo back"
                    }
                },
                "required": ["text"]
            })
        }

        async fn execute(&self, params: Value) -> Result<Value, Error> {
            if let Some(text) = params.get("text").and_then(|t| t.as_str()) {
                Ok(json!(text))
            } else {
                Err(anyhow::anyhow!("Missing 'text' parameter"))
            }
        }
    }

    #[tokio::test]
    async fn test_echo_tool() {
        let tool = EchoTool;
        let params = json!({"text": "Hello, world!"});
        let result = tool.execute(params).await.unwrap();
        assert_eq!(result.as_str().unwrap(), "Hello, world!");
    }
}
