//! Personality-based agents for LUTS CLI

use crate::agents::{Agent, AgentConfig, AgentMessage, MessageResponse};
use crate::llm::{AiService, InternalChatMessage, LLMService};
use crate::memory::{FjallMemoryStore, MemoryManager};
use crate::tools::{
    AiTool, block::BlockTool, calc::MathTool, delete_block::DeleteBlockTool,
    retrieve_context::RetrieveContextTool, search::DDGSearchTool, update_block::UpdateBlockTool,
    website::WebsiteTool,
};
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

/// Create personality-based agents with different reasoning styles and tools
pub struct PersonalityAgentBuilder;

impl PersonalityAgentBuilder {
    /// Create a "Researcher" agent - thorough, analytical, uses web tools
    pub fn create_researcher(data_dir: &str, provider: &str) -> Result<Box<dyn Agent>, Error> {
        let config = AgentConfig {
            agent_id: "researcher".to_string(),
            name: "Dr. Research".to_string(),
            role: "researcher".to_string(),
            system_prompt: Some(
                "You are Dr. Research, a thorough and analytical researcher. You excel at:\
                \n- Finding accurate information through web searches\
                \n- Analyzing websites and extracting key insights\
                \n- Storing important facts and information in memory blocks\
                \n- Retrieving and referencing previously stored knowledge\
                \n- Synthesizing information from multiple sources\
                \n- Providing well-sourced, factual responses\
                \n- Being methodical and comprehensive in your investigations\
                \n\nYou prefer to verify facts before stating them and always cite your sources.\
                \nYou actively use memory blocks to store important discoveries, facts, and research findings for future reference.\
                \n\nIMPORTANT: When you use any tools: Always give a clear final answer or response after using tools".to_string()
            ),
            provider: provider.to_string(),
            tool_names: vec!["search".to_string(), "website".to_string(), "block".to_string(), "retrieve_context".to_string(), "update_block".to_string()],
            data_dir: data_dir.to_string(),
        };

        let memory_manager = {
            let agent_data_dir = format!("{}/agents/{}", data_dir, config.agent_id);
            std::fs::create_dir_all(&agent_data_dir)
                .map_err(|e| anyhow!("Failed to create agent data directory: {}", e))?;
            let memory_store = FjallMemoryStore::new(&agent_data_dir)?;
            std::sync::Arc::new(MemoryManager::new(memory_store))
        };

        let mut tools = HashMap::new();
        tools.insert(
            "search".to_string(),
            Box::new(DDGSearchTool) as Box<dyn AiTool>,
        );
        tools.insert(
            "website".to_string(),
            Box::new(WebsiteTool) as Box<dyn AiTool>,
        );
        tools.insert(
            "block".to_string(),
            Box::new(BlockTool {
                memory_manager: memory_manager.clone(),
            }) as Box<dyn AiTool>,
        );
        tools.insert(
            "retrieve_context".to_string(),
            Box::new(RetrieveContextTool {
                memory_manager: memory_manager.clone(),
            }) as Box<dyn AiTool>,
        );
        tools.insert(
            "update_block".to_string(),
            Box::new(UpdateBlockTool {
                memory_manager: memory_manager.clone(),
            }) as Box<dyn AiTool>,
        );

        Ok(Box::new(PersonalityAgent::new(config, tools)?))
    }

    /// Create a "Calculator" agent - logical, precise, math-focused
    pub fn create_calculator(data_dir: &str, provider: &str) -> Result<Box<dyn Agent>, Error> {
        let config = AgentConfig {
            agent_id: "calculator".to_string(),
            name: "Logic".to_string(),
            role: "calculator".to_string(),
            system_prompt: Some(
                "You are Logic, a precise and methodical mathematical mind. You excel at:\
                \n- Solving complex mathematical problems step-by-step\
                \n- Breaking down complex calculations into manageable parts\
                \n- Explaining mathematical concepts clearly\
                \n- Verifying calculations and catching errors\
                \n- Finding patterns and relationships in data\
                \n\nYou think systematically, show your work, and double-check important calculations.\
                \n\nIMPORTANT: When you use any tools: Always provide a clear final answer with proper units or formatting".to_string()
            ),
            provider: provider.to_string(),
            tool_names: vec!["calc".to_string()],
            data_dir: data_dir.to_string(),
        };

        let mut tools = HashMap::new();
        tools.insert("calc".to_string(), Box::new(MathTool) as Box<dyn AiTool>);

        Ok(Box::new(PersonalityAgent::new(config, tools)?))
    }

    /// Create a "Creative" agent - imaginative, artistic, big-picture thinking
    pub fn create_creative(data_dir: &str, provider: &str) -> Result<Box<dyn Agent>, Error> {
        let config = AgentConfig {
            agent_id: "creative".to_string(),
            name: "Spark".to_string(),
            role: "creative".to_string(),
            system_prompt: Some(
                "You are Spark, a creative and imaginative thinker. You excel at:\
                \n- Generating novel ideas and creative solutions\
                \n- Thinking outside the box and making unexpected connections\
                \n- Storytelling and creative writing\
                \n- Brainstorming and ideation\
                \n- Finding artistic and aesthetic approaches to problems\
                \n\nYou approach challenges with curiosity and wonder, often proposing multiple creative alternatives.".to_string()
            ),
            provider: provider.to_string(),
            tool_names: vec![],
            data_dir: data_dir.to_string(),
        };

        let tools = HashMap::new(); // Creative agent relies on pure reasoning

        Ok(Box::new(PersonalityAgent::new(config, tools)?))
    }

    /// Create a "Coordinator" agent - organized, strategic, good at delegation
    pub fn create_coordinator(data_dir: &str, provider: &str) -> Result<Box<dyn Agent>, Error> {
        let config = AgentConfig {
            agent_id: "coordinator".to_string(),
            name: "Maestro".to_string(),
            role: "coordinator".to_string(),
            system_prompt: Some(
                "You are Maestro, a strategic coordinator and organizer. You excel at:\
                \n- Breaking complex tasks into manageable steps\
                \n- Coordinating multiple resources and team members\
                \n- Strategic planning and project management\
                \n- Storing important project information and decisions in memory blocks\
                \n- Tracking progress, goals, and preferences across projects\
                \n- Identifying which specialists to involve for different tasks\
                \n- Synthesizing results from multiple sources\
                \n\nYou think systematically about workflows and can delegate tasks to the right agents.\
                \nYou actively use memory blocks to track project goals, user preferences, and important decisions.\
                \n\nIMPORTANT: When you use any tools: Always provide clear recommendations or next actions based on the tool results".to_string()
            ),
            provider: provider.to_string(),
            tool_names: vec!["calc".to_string(), "search".to_string(), "website".to_string(), "block".to_string(), "retrieve_context".to_string(), "update_block".to_string()],
            data_dir: data_dir.to_string(),
        };

        let memory_manager = {
            let agent_data_dir = format!("{}/agents/{}", data_dir, config.agent_id);
            std::fs::create_dir_all(&agent_data_dir)
                .map_err(|e| anyhow!("Failed to create agent data directory: {}", e))?;
            let memory_store = FjallMemoryStore::new(&agent_data_dir)?;
            std::sync::Arc::new(MemoryManager::new(memory_store))
        };

        let mut tools = HashMap::new();
        tools.insert("calc".to_string(), Box::new(MathTool) as Box<dyn AiTool>);
        tools.insert(
            "search".to_string(),
            Box::new(DDGSearchTool) as Box<dyn AiTool>,
        );
        tools.insert(
            "website".to_string(),
            Box::new(WebsiteTool) as Box<dyn AiTool>,
        );
        tools.insert(
            "block".to_string(),
            Box::new(BlockTool {
                memory_manager: memory_manager.clone(),
            }) as Box<dyn AiTool>,
        );
        tools.insert(
            "retrieve_context".to_string(),
            Box::new(RetrieveContextTool {
                memory_manager: memory_manager.clone(),
            }) as Box<dyn AiTool>,
        );
        tools.insert(
            "update_block".to_string(),
            Box::new(UpdateBlockTool {
                memory_manager: memory_manager.clone(),
            }) as Box<dyn AiTool>,
        );

        Ok(Box::new(PersonalityAgent::new(config, tools)?))
    }

    /// Create a "Pragmatic" agent - practical, efficient, solution-focused
    pub fn create_pragmatic(data_dir: &str, provider: &str) -> Result<Box<dyn Agent>, Error> {
        let config = AgentConfig {
            agent_id: "pragmatic".to_string(),
            name: "Practical".to_string(),
            role: "pragmatic".to_string(),
            system_prompt: Some(
                "You are Practical, a pragmatic and efficient problem-solver. You excel at:\
                \n- Finding the most efficient solution to problems\
                \n- Cutting through complexity to focus on what matters\
                \n- Providing actionable, concrete advice\
                \n- Balancing trade-offs and making practical decisions\
                \n- Getting things done with minimal fuss\
                \n\nYou prefer simple, working solutions over complex theoretical approaches.\
                \n\nIMPORTANT: When you use any tools: Always provide a clear, practical final answer or next steps"
                    .to_string(),
            ),
            provider: provider.to_string(),
            tool_names: vec!["calc".to_string(), "search".to_string()],
            data_dir: data_dir.to_string(),
        };

        let mut tools = HashMap::new();
        tools.insert("calc".to_string(), Box::new(MathTool) as Box<dyn AiTool>);
        tools.insert(
            "search".to_string(),
            Box::new(DDGSearchTool) as Box<dyn AiTool>,
        );

        Ok(Box::new(PersonalityAgent::new(config, tools)?))
    }

    /// List all available personality types
    pub fn list_personalities() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            (
                "researcher",
                "Dr. Research",
                "Thorough analyst and fact-finder",
            ),
            ("calculator", "Logic", "Precise mathematical problem-solver"),
            ("creative", "Spark", "Imaginative and artistic thinker"),
            (
                "coordinator",
                "Maestro",
                "Strategic organizer and delegator",
            ),
            ("pragmatic", "Practical", "Efficient and solution-focused"),
        ]
    }

    /// Create an agent by personality type
    pub fn create_by_type(
        personality: &str,
        data_dir: &str,
        provider: &str,
    ) -> Result<Box<dyn Agent>, Error> {
        match personality.to_lowercase().as_str() {
            "researcher" => Self::create_researcher(data_dir, provider),
            "calculator" => Self::create_calculator(data_dir, provider),
            "creative" => Self::create_creative(data_dir, provider),
            "coordinator" => Self::create_coordinator(data_dir, provider),
            "pragmatic" => Self::create_pragmatic(data_dir, provider),
            _ => Err(anyhow!(
                "Unknown personality type: {}. Available: researcher, calculator, creative, coordinator, pragmatic",
                personality
            )),
        }
    }
}

/// A personality-based agent implementation
pub struct PersonalityAgent {
    config: AgentConfig,
    llm_service: LLMService,
    _memory_manager: MemoryManager,
    tools: HashMap<String, Box<dyn AiTool>>,
    /// Conversation history for this agent
    conversation_history: Vec<InternalChatMessage>,
}

impl PersonalityAgent {
    pub fn new(
        config: AgentConfig,
        tools: HashMap<String, Box<dyn AiTool>>,
    ) -> Result<Self, Error> {
        // Create LLM service with agent's tools
        let tool_vec: Vec<Box<dyn AiTool>> = tools
            .values()
            .map(|tool| {
                // Create a simple clone of the tool for the LLM service
                // In a real implementation, you'd want better tool sharing
                match tool.name() {
                    "calc" => Box::new(MathTool) as Box<dyn AiTool>,
                    "search" => Box::new(DDGSearchTool) as Box<dyn AiTool>,
                    "website" => Box::new(WebsiteTool) as Box<dyn AiTool>,
                    "block" => {
                        // Create memory manager for this tool instance
                        let agent_data_dir =
                            format!("{}/agents/{}", config.data_dir, config.agent_id);
                        std::fs::create_dir_all(&agent_data_dir).unwrap();
                        let memory_store = FjallMemoryStore::new(&agent_data_dir).unwrap();
                        let memory_manager = std::sync::Arc::new(MemoryManager::new(memory_store));
                        Box::new(BlockTool { memory_manager }) as Box<dyn AiTool>
                    }
                    "retrieve_context" => {
                        let agent_data_dir =
                            format!("{}/agents/{}", config.data_dir, config.agent_id);
                        std::fs::create_dir_all(&agent_data_dir).unwrap();
                        let memory_store = FjallMemoryStore::new(&agent_data_dir).unwrap();
                        let memory_manager = std::sync::Arc::new(MemoryManager::new(memory_store));
                        Box::new(RetrieveContextTool { memory_manager }) as Box<dyn AiTool>
                    }
                    "update_block" => {
                        let agent_data_dir =
                            format!("{}/agents/{}", config.data_dir, config.agent_id);
                        std::fs::create_dir_all(&agent_data_dir).unwrap();
                        let memory_store = FjallMemoryStore::new(&agent_data_dir).unwrap();
                        let memory_manager = std::sync::Arc::new(MemoryManager::new(memory_store));
                        Box::new(UpdateBlockTool { memory_manager }) as Box<dyn AiTool>
                    }
                    "delete_block" => {
                        let agent_data_dir =
                            format!("{}/agents/{}", config.data_dir, config.agent_id);
                        std::fs::create_dir_all(&agent_data_dir).unwrap();
                        let memory_store = FjallMemoryStore::new(&agent_data_dir).unwrap();
                        let memory_manager = std::sync::Arc::new(MemoryManager::new(memory_store));
                        Box::new(DeleteBlockTool { memory_manager }) as Box<dyn AiTool>
                    }
                    _ => Box::new(DummyTool {
                        name: tool.name().to_string(),
                    }) as Box<dyn AiTool>,
                }
            })
            .collect();

        let llm_service =
            LLMService::new(config.system_prompt.as_deref(), tool_vec, &config.provider)?;

        // Create memory manager with agent-specific data directory
        let agent_data_dir = format!("{}/agents/{}", config.data_dir, config.agent_id);
        std::fs::create_dir_all(&agent_data_dir)?;
        let memory_store = FjallMemoryStore::new(&agent_data_dir)?;
        let memory_manager = MemoryManager::new(memory_store);

        Ok(PersonalityAgent {
            config,
            llm_service,
            _memory_manager: memory_manager,
            tools,
            conversation_history: Vec::new(),
        })
    }
}

#[async_trait]
impl Agent for PersonalityAgent {
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
        debug!(
            "Agent {} ({}) processing message from {}",
            self.name(),
            self.agent_id(),
            message.from_agent_id
        );
        debug!(
            "Agent {} has {} tools available: {:?}",
            self.name(),
            self.tools.len(),
            self.tools.keys().collect::<Vec<_>>()
        );

        // Add the user message to conversation history
        self.conversation_history.push(InternalChatMessage::User {
            content: message.content.clone(),
        });

        // Start with the full conversation history
        let mut conversation_messages = self.conversation_history.clone();

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

            debug!(
                "Agent {} tool loop iteration {}, conversation has {} messages",
                self.name(),
                iteration_count,
                conversation_messages.len()
            );

            // Generate response using LLM service
            match self
                .llm_service
                .generate_response(&conversation_messages)
                .await
            {
                Ok(response_content) => {
                    debug!(
                        "Agent {} received response content type: {:?}",
                        self.name(),
                        std::mem::discriminant(&response_content)
                    );

                    // Pattern match to handle different content types
                    match response_content {
                        genai::chat::MessageContent::ToolCalls(tool_calls) => {
                            debug!(
                                "Agent {} received {} tool calls",
                                self.name(),
                                tool_calls.len()
                            );

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

                                debug!("=== TOOL EXECUTION DEBUG ===");
                                debug!("Tool name requested: '{}'", tool_name);
                                debug!("Tool args: {:?}", tool_args);
                                debug!("Call ID: {}", call_id);
                                debug!("Available tools: {:?}", self.tools.keys().collect::<Vec<_>>());
                                
                                // Check if the tool exists in our registry
                                if !self.tools.contains_key(tool_name) {
                                    debug!("ERROR: Tool '{}' not found in agent's tool registry!", tool_name);
                                }

                                // Find and execute the tool
                                let tool_result = if let Some(tool) = self.tools.get(tool_name) {
                                    debug!("Found tool '{}', executing...", tool_name);
                                    match tool.execute(tool_args.clone()).await {
                                        Ok(result) => {
                                            info!(
                                                "Tool {} completed successfully: {:?}",
                                                tool_name, result
                                            );
                                            result.to_string()
                                        }
                                        Err(e) => {
                                            info!("Tool {} failed: {}", tool_name, e);
                                            format!("Error executing tool {}: {}", tool_name, e)
                                        }
                                    }
                                } else {
                                    let error_msg = format!(
                                        "Tool '{}' not found. Available tools: {:?}",
                                        tool_name,
                                        self.tools.keys().collect::<Vec<_>>()
                                    );
                                    debug!("Tool lookup failed: {}", error_msg);
                                    error_msg
                                };

                                debug!("Tool {} result: {}", tool_name, tool_result);

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

                            // Add explanatory prompt after tool execution to encourage explanation
                            let explanation_prompt = InternalChatMessage::System {
                                content: "Please explain what tools you just used, what results you obtained, and how this information helps answer the user's question. Provide your reasoning and give a clear final response.".to_string(),
                            };
                            conversation_messages.push(explanation_prompt);

                            debug!(
                                "Agent {} continuing loop after tool execution, conversation now has {} messages",
                                self.name(),
                                conversation_messages.len()
                            );

                            // Continue the loop to get the next LLM response
                            continue;
                        }
                        genai::chat::MessageContent::Text(response_text) => {
                            info!(
                                "Agent {} generated final response: {}",
                                self.name(),
                                response_text
                            );

                            // DEBUG: Check if AI mentioned searching but didn't actually call search tool
                            let mentions_search = response_text.to_lowercase().contains("search")
                                || response_text.to_lowercase().contains("look")
                                || response_text.to_lowercase().contains("find");
                            
                            if mentions_search && iteration_count == 1 {
                                debug!("WARNING: AI mentioned search-related action ('{}') but didn't make tool calls!", 
                                       response_text.chars().take(100).collect::<String>());
                                debug!("Available search tools: {:?}", 
                                       self.tools.keys().filter(|k| k.contains("search")).collect::<Vec<_>>());
                            }

                            // Add assistant response to conversation history
                            let assistant_message = InternalChatMessage::Assistant {
                                content: response_text.clone(),
                                tool_responses: None,
                            };
                            self.conversation_history.push(assistant_message);

                            return Ok(MessageResponse::success(
                                message.message_id,
                                response_text,
                                None,
                            ));
                        }
                        genai::chat::MessageContent::Parts(parts) => {
                            // Extract text from parts and treat as final response
                            let combined_text = parts
                                .into_iter()
                                .filter_map(|part| match part {
                                    genai::chat::ContentPart::Text(text) => Some(text),
                                    _ => None, // Skip images or other non-text parts
                                })
                                .collect::<Vec<_>>()
                                .join(" ");

                            if !combined_text.is_empty() {
                                info!(
                                    "Agent {} generated final response from parts: {}",
                                    self.name(),
                                    combined_text
                                );

                                // Add assistant response to conversation history
                                let assistant_message = InternalChatMessage::Assistant {
                                    content: combined_text.clone(),
                                    tool_responses: None,
                                };
                                self.conversation_history.push(assistant_message);

                                return Ok(MessageResponse::success(
                                    message.message_id,
                                    combined_text,
                                    None,
                                ));
                            } else {
                                return Ok(MessageResponse::error(
                                    message.message_id,
                                    "LLM response contained only non-text parts (images, etc.)"
                                        .to_string(),
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
                    return Ok(MessageResponse::error(
                        message.message_id,
                        format!("Failed to generate response: {}", e),
                    ));
                }
            }
        }
    }

    async fn send_message(&self, _message: AgentMessage) -> Result<(), Error> {
        // In CLI mode, agents don't need to send messages to each other
        // This would be implemented if running in a full multiagent environment
        Ok(())
    }

    fn get_available_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Simple dummy tool for unknown tool types
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
