//! Base agent implementation

use crate::agents::{Agent, AgentConfig, AgentMessage, MessageResponse, ToolCallInfo};
use luts_llm::{AiService, InternalChatMessage, LLMService};
use luts_memory::{MemoryManager, SurrealMemoryStore, SurrealConfig};
use luts_llm::tools::AiTool;
use crate::tools::modify_core_block::ModifyCoreBlockTool;
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// A base implementation of an Agent
pub struct BaseAgent {
    /// Agent configuration
    config: AgentConfig,
    
    /// LLM service for this agent
    llm_service: LLMService,
    
    /// Memory manager for this agent's personal memory
    memory_manager: MemoryManager,
    
    /// Available tools for this agent
    tools: HashMap<String, Box<dyn AiTool>>,
    
    /// Message sender (injected by registry)
    message_sender: Option<Arc<RwLock<dyn MessageSender>>>,
    
    /// Conversation history for this agent
    conversation_history: Vec<InternalChatMessage>,
}

/// Trait for sending messages (implemented by registry)
#[async_trait]
pub trait MessageSender: Send + Sync {
    async fn send_message(&self, message: AgentMessage) -> Result<(), Error>;
    async fn send_message_and_wait(&self, message: AgentMessage) -> Result<MessageResponse, Error>;
}

impl BaseAgent {
    /// Create a new base agent
    pub fn new(
        config: AgentConfig,
        tools: HashMap<String, Box<dyn AiTool>>,
    ) -> Result<Self, Error> {
        // Clone tools for LLM service - we need to implement a proper clone method
        // For now, let's pass the tools directly to LLM service without cloning
        let tool_vec: Vec<Box<dyn AiTool>> = tools.iter()
            .map(|(name, tool)| {
                // Create a new instance of each tool type based on its name
                // This is a temporary workaround until we implement proper tool cloning
                match name.as_str() {
                    "calculator" | "calc" => Box::new(luts_tools::calc::MathTool) as Box<dyn AiTool>,
                    "search" => Box::new(luts_tools::search::DDGSearchTool) as Box<dyn AiTool>,
                    "website" => Box::new(luts_tools::website::WebsiteTool) as Box<dyn AiTool>,
                    "retrieve_context" => {
                        let agent_data_dir = format!("{}/agents/{}", config.data_dir, config.agent_id);
                        std::fs::create_dir_all(&agent_data_dir).unwrap_or_default();
                        let surreal_config = SurrealConfig::File {
                            path: std::path::PathBuf::from(agent_data_dir).join("memory.db"),
                            namespace: "luts".to_string(),
                            database: "memory".to_string(),
                        };
                        let memory_store = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                SurrealMemoryStore::new(surreal_config).await.unwrap()
                            })
                        });
                        let memory_manager = std::sync::Arc::new(luts_memory::MemoryManager::new(memory_store));
                        Box::new(crate::tools::retrieve_context::RetrieveContextTool { memory_manager }) as Box<dyn AiTool>
                    },
                    "block" => {
                        let agent_data_dir = format!("{}/agents/{}", config.data_dir, config.agent_id);
                        std::fs::create_dir_all(&agent_data_dir).unwrap_or_default();
                        let memory_store = {
                            let surreal_config = SurrealConfig::File {
                                path: std::path::PathBuf::from(&agent_data_dir).join("memory.db"),
                                namespace: "luts".to_string(),
                                database: "memory".to_string(),
                            };
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    SurrealMemoryStore::new(surreal_config).await.unwrap()
                                })
                            })
                        };
                        let memory_manager = std::sync::Arc::new(luts_memory::MemoryManager::new(memory_store));
                        Box::new(crate::tools::block::BlockTool { memory_manager }) as Box<dyn AiTool>
                    },
                    "update_block" => {
                        let agent_data_dir = format!("{}/agents/{}", config.data_dir, config.agent_id);
                        std::fs::create_dir_all(&agent_data_dir).unwrap_or_default();
                        let memory_store = {
                            let surreal_config = SurrealConfig::File {
                                path: std::path::PathBuf::from(&agent_data_dir).join("memory.db"),
                                namespace: "luts".to_string(),
                                database: "memory".to_string(),
                            };
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    SurrealMemoryStore::new(surreal_config).await.unwrap()
                                })
                            })
                        };
                        let memory_manager = std::sync::Arc::new(luts_memory::MemoryManager::new(memory_store));
                        Box::new(crate::tools::update_block::UpdateBlockTool { memory_manager }) as Box<dyn AiTool>
                    },
                    "delete_block" => {
                        let agent_data_dir = format!("{}/agents/{}", config.data_dir, config.agent_id);
                        std::fs::create_dir_all(&agent_data_dir).unwrap_or_default();
                        let memory_store = {
                            let surreal_config = SurrealConfig::File {
                                path: std::path::PathBuf::from(&agent_data_dir).join("memory.db"),
                                namespace: "luts".to_string(),
                                database: "memory".to_string(),
                            };
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    SurrealMemoryStore::new(surreal_config).await.unwrap()
                                })
                            })
                        };
                        let memory_manager = std::sync::Arc::new(luts_memory::MemoryManager::new(memory_store));
                        Box::new(crate::tools::delete_block::DeleteBlockTool { memory_manager }) as Box<dyn AiTool>
                    },
                    "modify_core_block" => {
                        Box::new(ModifyCoreBlockTool::new(config.agent_id.clone(), None)) as Box<dyn AiTool>
                    },
                    _ => {
                        tracing::warn!("Unknown tool type: {}, using dummy tool", name);
                        Box::new(DummyTool { name: tool.name().to_string() }) as Box<dyn AiTool>
                    }
                }
            })
            .collect();
        
        let llm_service = LLMService::new(
            config.system_prompt.as_deref(),
            tool_vec,
            &config.provider,
        )?;
        
        // Create memory manager with agent-specific data directory
        let agent_data_dir = format!("{}/agents/{}", config.data_dir, config.agent_id);
        std::fs::create_dir_all(&agent_data_dir)
            .map_err(|e| anyhow!("Failed to create agent data directory: {}", e))?;
        let surreal_config = SurrealConfig::File {
            path: std::path::PathBuf::from(agent_data_dir).join("memory.db"),
            namespace: "luts".to_string(),
            database: "memory".to_string(),
        };
        let memory_store = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                SurrealMemoryStore::new(surreal_config).await
            })
        })?;
        let memory_manager = MemoryManager::new(memory_store);
        
        Ok(BaseAgent {
            config,
            llm_service,
            memory_manager,
            tools,
            message_sender: None,
            conversation_history: Vec::new(),
        })
    }
    
    /// Set the message sender (called by registry)
    pub fn set_message_sender(&mut self, sender: Arc<RwLock<dyn MessageSender>>) {
        self.message_sender = Some(sender);
    }
    
    /// Get the memory manager for this agent
    pub fn memory_manager(&self) -> &MemoryManager {
        &self.memory_manager
    }
}

#[async_trait]
impl Agent for BaseAgent {
    fn agent_id(&self) -> &str {
        &self.config.agent_id
    }
    
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn role(&self) -> &str {
        &self.config.role
    }
    
    async fn process_message(&mut self, message: AgentMessage) -> Result<MessageResponse, Error> {
        debug!("Agent {} processing message from {}", self.agent_id(), message.from_agent_id);
        
        // Add the user message to conversation history
        self.conversation_history.push(InternalChatMessage::User {
            content: message.content.clone(),
        });

        // Start with the full conversation history
        let mut conversation_messages = self.conversation_history.clone();
        
        // Track all tool calls for this message
        let mut all_tool_calls = Vec::new();

        // Tool execution loop - continue until we get a text response
        let max_tool_iterations = 10; // Prevent infinite loops
        let mut iteration_count = 0;

        loop {
            iteration_count += 1;
            if iteration_count > max_tool_iterations {
                return Ok(MessageResponse::error(
                    message.message_id,
                    "Maximum tool execution iterations reached".to_string(),
                ));
            }

            debug!("Agent {} tool loop iteration {}, conversation has {} messages", 
                   self.agent_id(), iteration_count, conversation_messages.len());

            // Generate response using LLM service
            match self.llm_service.generate_response(&conversation_messages).await {
                Ok(response_content) => {
                    debug!("Agent {} received response content type: {:?}", 
                           self.agent_id(), std::mem::discriminant(&response_content));
                    
                    // Pattern match to handle different content types
                    match response_content {
                        genai::chat::MessageContent::ToolCalls(tool_calls) => {
                            debug!("Agent {} received {} tool calls", self.agent_id(), tool_calls.len());
                            
                            // Add assistant message with tool calls to conversation
                            let assistant_message = InternalChatMessage::Assistant {
                                content: "Tool calls requested".to_string(),
                                tool_responses: None,
                            };
                            conversation_messages.push(assistant_message.clone());
                            // IMPORTANT: Save to persistent history
                            self.conversation_history.push(assistant_message);

                            // Execute each tool call
                            for tool_call in tool_calls {
                                let tool_name = &tool_call.fn_name;
                                let tool_args = &tool_call.fn_arguments;
                                let call_id = &tool_call.call_id;
                                
                                debug!("Executing tool: {} with args: {:?}", tool_name, tool_args);
                                
                                // Find and execute the tool
                                let (tool_result, tool_success) = if let Some(tool) = self.tools.get(tool_name) {
                                    match tool.execute(tool_args.clone()).await {
                                        Ok(result) => {
                                            info!("Tool {} completed successfully: {:?}", tool_name, result);
                                            (result.to_string(), true)
                                        }
                                        Err(e) => {
                                            info!("Tool {} failed: {}", tool_name, e);
                                            (format!("Error executing tool {}: {}", tool_name, e), false)
                                        }
                                    }
                                } else {
                                    (format!("Tool '{}' not found. Available tools: {:?}", tool_name, self.tools.keys().collect::<Vec<_>>()), false)
                                };
                                
                                debug!("Tool {} result: {}", tool_name, tool_result);
                                
                                // Record tool call info for API response
                                let tool_call_info = ToolCallInfo {
                                    tool_name: tool_name.clone(),
                                    tool_args: tool_args.clone(),
                                    tool_result: tool_result.clone(),
                                    success: tool_success,
                                    call_id: Some(call_id.clone()),
                                };
                                all_tool_calls.push(tool_call_info);
                                debug!("Agent {} recorded tool call: {} (success: {})", self.agent_id(), tool_name, tool_success);
                                
                                // Add tool response to conversation
                                let tool_message = InternalChatMessage::Tool {
                                    tool_name: tool_name.clone(),
                                    content: tool_result,
                                    call_id: Some(call_id.clone()),
                                };
                                conversation_messages.push(tool_message.clone());
                                // IMPORTANT: Save to persistent history
                                self.conversation_history.push(tool_message);
                            }
                            
                            debug!("Agent {} continuing loop after tool execution, conversation now has {} messages", 
                                   self.agent_id(), conversation_messages.len());
                            
                            // Continue the loop to get the next LLM response
                            continue;
                        }
                        genai::chat::MessageContent::Text(response_text) => {
                            info!("Agent {} generated final response: {}", self.agent_id(), response_text);
                            
                            // Add assistant response to conversation history
                            let assistant_message = InternalChatMessage::Assistant {
                                content: response_text.clone(),
                                tool_responses: None,
                            };
                            self.conversation_history.push(assistant_message);
                            
                            debug!("Agent {} returning response with {} tool calls", self.agent_id(), all_tool_calls.len());
                            
                            return Ok(MessageResponse::success_with_tools(
                                message.message_id,
                                response_text,
                                None,
                                all_tool_calls,
                            ));
                        }
                        genai::chat::MessageContent::Parts(parts) => {
                            // Extract text from parts and treat as final response
                            let combined_text = parts.into_iter()
                                .filter_map(|part| match part {
                                    genai::chat::ContentPart::Text(text) => Some(text),
                                    _ => None, // Skip images or other non-text parts
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            
                            if !combined_text.is_empty() {
                                info!("Agent {} generated final response from parts: {}", self.agent_id(), combined_text);
                                
                                // Add assistant response to conversation history
                                let assistant_message = InternalChatMessage::Assistant {
                                    content: combined_text.clone(),
                                    tool_responses: None,
                                };
                                self.conversation_history.push(assistant_message);
                                
                                debug!("Agent {} returning response with {} tool calls (from parts)", self.agent_id(), all_tool_calls.len());
                                
                                return Ok(MessageResponse::success_with_tools(
                                    message.message_id,
                                    combined_text,
                                    None,
                                    all_tool_calls,
                                ));
                            } else {
                                return Ok(MessageResponse::error(
                                    message.message_id,
                                    "LLM response contained only non-text parts (images, etc.)".to_string(),
                                ));
                            }
                        }
                        genai::chat::MessageContent::ToolResponses(_) => {
                            // This shouldn't happen from LLM, but handle gracefully
                            return Ok(MessageResponse::error(
                                message.message_id,
                                "LLM unexpectedly returned tool responses".to_string(),
                            ));
                        }
                    }
                }
                Err(e) => {
                    error!("Agent {} failed to generate response: {}", self.agent_id(), e);
                    return Ok(MessageResponse::error(
                        message.message_id,
                        format!("Failed to generate response: {}", e),
                    ));
                }
            }
        }
    }
    
    async fn send_message(&self, message: AgentMessage) -> Result<(), Error> {
        if let Some(sender) = &self.message_sender {
            sender.read().await.send_message(message).await
        } else {
            Err(anyhow!("No message sender configured for agent {}", self.agent_id()))
        }
    }
    
    fn get_available_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Temporary dummy tool for compilation - we'll improve tool sharing later
struct DummyTool {
    name: String,
}

#[async_trait]
impl AiTool for DummyTool {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "Dummy tool"
    }
    
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }
    
    async fn execute(&self, _params: serde_json::Value) -> Result<serde_json::Value, Error> {
        Ok(serde_json::json!({"result": "dummy"}))
    }
}