//! Letta-style context blocks for structured agent interaction
//!
//! This module implements a block-based system similar to Letta's context blocks,
//! allowing for modular, coordinated agent interactions with dependencies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use anyhow::Result;
use luts_core::llm::LLMService;
use std::sync::Arc;

/// Type of context block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlockType {
    /// System prompt defining agent personality and behavior
    System,
    /// User input or request
    User,
    /// Memory block with retrieved facts or prior interactions
    Memory,
    /// Example for few-shot learning
    Example,
    /// Tool capability or API call
    Tool,
    /// Dynamic context like task metadata or retrieved documents
    Dynamic,
    /// Response block for agent outputs with coordination
    Response,
}

/// Status of a block in the coordination workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlockStatus {
    /// Block is waiting to be processed
    Pending,
    /// Block is currently being processed
    Processing,
    /// Block has been successfully processed
    Completed,
    /// Block failed to process
    Failed,
    /// Block was skipped due to conditions
    Skipped,
}

/// Context block for modular agent interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlock {
    /// Unique identifier for this block
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Type of block
    pub block_type: BlockType,
    /// The content of the block
    pub content: String,
    /// Current processing status
    pub status: BlockStatus,
    /// Priority level (1-10, higher = more important)
    pub priority: u8,
    /// List of block IDs this block depends on
    pub depends_on: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// When this block was created
    pub created_at: DateTime<Utc>,
    /// When this block was last updated
    pub updated_at: DateTime<Utc>,
    /// Optional tags for organization
    pub tags: Vec<String>,
}

impl ContextBlock {
    /// Create a new context block
    pub fn new(
        id: String,
        name: String,
        block_type: BlockType,
        content: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            block_type,
            content,
            status: BlockStatus::Pending,
            priority: 5,
            depends_on: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
        }
    }

    /// Create a system block
    pub fn system(id: String, name: String, content: String) -> Self {
        Self::new(id, name, BlockType::System, content)
    }

    /// Create a user block
    pub fn user(id: String, name: String, content: String) -> Self {
        Self::new(id, name, BlockType::User, content)
    }

    /// Create a memory block
    pub fn memory(id: String, name: String, content: String) -> Self {
        Self::new(id, name, BlockType::Memory, content)
    }

    /// Create a tool block
    pub fn tool(id: String, name: String, content: String) -> Self {
        Self::new(id, name, BlockType::Tool, content)
    }

    /// Create a response block
    pub fn response(id: String, name: String, content: String) -> Self {
        Self::new(id, name, BlockType::Response, content)
    }

    /// Add a dependency to this block
    pub fn with_dependency(mut self, dependency_id: String) -> Self {
        if !self.depends_on.contains(&dependency_id) {
            self.depends_on.push(dependency_id);
        }
        self
    }

    /// Add multiple dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        for dep in dependencies {
            if !self.depends_on.contains(&dep) {
                self.depends_on.push(dep);
            }
        }
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(10);
        self
    }

    /// Add metadata
    #[allow(dead_code)]
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add tags
    #[allow(dead_code)]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Update status
    pub fn update_status(&mut self, status: BlockStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Check if all dependencies are completed
    pub fn dependencies_satisfied(&self, blocks: &[ContextBlock]) -> bool {
        if self.depends_on.is_empty() {
            return true;
        }

        for dep_id in &self.depends_on {
            if let Some(dep_block) = blocks.iter().find(|b| b.id == *dep_id) {
                if dep_block.status != BlockStatus::Completed {
                    return false;
                }
            } else {
                // Dependency not found
                return false;
            }
        }
        true
    }

    /// Get display color based on block type
    pub fn get_display_color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self.block_type {
            BlockType::System => Color::Blue,
            BlockType::User => Color::Green,
            BlockType::Memory => Color::Yellow,
            BlockType::Example => Color::Magenta,
            BlockType::Tool => Color::Cyan,
            BlockType::Dynamic => Color::Gray,
            BlockType::Response => Color::Red,
        }
    }

    /// Add execution result to the block
    pub fn set_result(&mut self, result: String) {
        self.metadata.insert("result".to_string(), result);
        self.updated_at = Utc::now();
    }

    /// Get execution result from the block
    pub fn get_result(&self) -> Option<&String> {
        self.metadata.get("result")
    }

    /// Set error message for failed blocks
    pub fn set_error(&mut self, error: String) {
        self.metadata.insert("error".to_string(), error);
        self.updated_at = Utc::now();
    }

    /// Get error message from failed blocks
    pub fn get_error(&self) -> Option<&String> {
        self.metadata.get("error")
    }

    /// Get status icon
    pub fn get_status_icon(&self) -> &'static str {
        match self.status {
            BlockStatus::Pending => "⏳",
            BlockStatus::Processing => "🔄",
            BlockStatus::Completed => "✅",
            BlockStatus::Failed => "❌",
            BlockStatus::Skipped => "⏭️",
        }
    }
}

/// Block coordinator for managing workflow dependencies
#[derive(Clone)]
pub struct BlockCoordinator {
    /// All blocks in the system
    pub _blocks: Vec<ContextBlock>,
    /// Current execution context
    #[allow(dead_code)]
    pub context: HashMap<String, String>,
    /// Optional LLM service for executing AI blocks
    pub _llm_service: Option<Arc<LLMService>>,
}

impl BlockCoordinator {
    /// Create a new block coordinator
    pub fn new() -> Self {
        Self {
            _blocks: Vec::new(),
            context: HashMap::new(),
            _llm_service: None,
        }
    }

    /// Set LLM service for AI block execution
    pub fn set_llm_service(&mut self, llm_service: Arc<LLMService>) {
        self._llm_service = Some(llm_service);
    }

    /// Add a block to the coordinator
    pub fn add_block(&mut self, block: ContextBlock) {
        self._blocks.push(block);
    }

    /// Get the next block ready for processing
    pub fn get_next_ready_block(&self) -> Option<&ContextBlock> {
        self._blocks
            .iter()
            .filter(|b| b.status == BlockStatus::Pending)
            .filter(|b| b.dependencies_satisfied(&self._blocks))
            .max_by_key(|b| b.priority)
    }

    /// Update block status
    pub fn update_block_status(&mut self, block_id: &str, status: BlockStatus) {
        if let Some(block) = self._blocks.iter_mut().find(|b| b.id == block_id) {
            block.update_status(status);
        }
    }

    /// Get blocks by type
    pub fn get_blocks_by_type(&self, block_type: BlockType) -> Vec<&ContextBlock> {
        self._blocks
            .iter()
            .filter(|b| b.block_type == block_type)
            .collect()
    }

    /// Get blocks by status
    pub fn get_blocks_by_status(&self, status: BlockStatus) -> Vec<&ContextBlock> {
        self._blocks
            .iter()
            .filter(|b| b.status == status)
            .collect()
    }

    /// Generate context for LLM from completed blocks
    pub fn generate_llm_context(&self) -> String {
        let mut context = String::new();
        
        // Add system blocks first
        for block in self.get_blocks_by_type(BlockType::System) {
            if block.status == BlockStatus::Completed {
                context.push_str(&format!("[SYSTEM: {}]\n{}\n\n", block.name, block.content));
            }
        }

        // Add memory blocks
        for block in self.get_blocks_by_type(BlockType::Memory) {
            if block.status == BlockStatus::Completed {
                context.push_str(&format!("[MEMORY: {}]\n{}\n\n", block.name, block.content));
            }
        }

        // Add examples
        for block in self.get_blocks_by_type(BlockType::Example) {
            if block.status == BlockStatus::Completed {
                context.push_str(&format!("[EXAMPLE: {}]\n{}\n\n", block.name, block.content));
            }
        }

        // Add tools
        for block in self.get_blocks_by_type(BlockType::Tool) {
            if block.status == BlockStatus::Completed {
                context.push_str(&format!("[TOOL: {}]\n{}\n\n", block.name, block.content));
            }
        }

        // Add dynamic context
        for block in self.get_blocks_by_type(BlockType::Dynamic) {
            if block.status == BlockStatus::Completed {
                context.push_str(&format!("[CONTEXT: {}]\n{}\n\n", block.name, block.content));
            }
        }

        // Add user input
        for block in self.get_blocks_by_type(BlockType::User) {
            if block.status == BlockStatus::Completed {
                context.push_str(&format!("[USER: {}]\n{}\n\n", block.name, block.content));
            }
        }

        context
    }

    /// Check if workflow is complete
    pub fn is_workflow_complete(&self) -> bool {
        self._blocks.iter().all(|b| 
            b.status == BlockStatus::Completed || 
            b.status == BlockStatus::Skipped ||
            b.status == BlockStatus::Failed
        )
    }

    /// Execute a block based on its type
    pub async fn execute_block(&mut self, block_id: &str) -> Result<()> {
        let block_index = self._blocks.iter().position(|b| b.id == block_id)
            .ok_or_else(|| anyhow::anyhow!("Block not found: {}", block_id))?;

        // Get block info for execution
        let (block_type, content, dependencies) = {
            let block = &self._blocks[block_index];
            (block.block_type.clone(), block.content.clone(), block.depends_on.clone())
        };

        // Set block to processing
        self._blocks[block_index].update_status(BlockStatus::Processing);

        let result = match block_type {
            BlockType::System => {
                // System blocks just complete - they set context/personality
                Ok("System block activated".to_string())
            },
            BlockType::User => {
                // User blocks store user input - just complete
                Ok(format!("User input received: {}", content))
            },
            BlockType::Memory => {
                // In a real implementation, this would query the memory system
                // For now, simulate memory retrieval
                Ok(format!("Memory retrieved: {}", content))
            },
            BlockType::Tool => {
                // Tool blocks would execute actual tools
                self.execute_tool_block(&content).await
            },
            BlockType::Response => {
                // Response blocks generate AI responses
                self.execute_response_block(&content, &dependencies).await
            },
            BlockType::Dynamic => {
                // Dynamic blocks process context
                Ok(format!("Dynamic context processed: {}", content))
            },
            BlockType::Example => {
                // Example blocks just provide context
                Ok(format!("Example provided: {}", content))
            }
        };

        // Update block with result
        match result {
            Ok(output) => {
                self._blocks[block_index].set_result(output);
                self._blocks[block_index].update_status(BlockStatus::Completed);
            },
            Err(e) => {
                self._blocks[block_index].set_error(e.to_string());
                self._blocks[block_index].update_status(BlockStatus::Failed);
            }
        }

        Ok(())
    }

    /// Execute a tool block
    async fn execute_tool_block(&self, content: &str) -> Result<String> {
        // Parse tool specification from content
        if content.contains("weather") {
            // Simulate weather API call
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            Ok("Today's weather: 72°F, sunny with light clouds ☀️".to_string())
        } else if content.contains("bluesky") || content.contains("post") {
            // Simulate Bluesky API call
            tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
            Ok("Successfully posted to Bluesky! 🦋 Post ID: at://did:example/app.bsky.feed.post/12345".to_string())
        } else if content.contains("generate") {
            // Generate content using dependencies
            Ok("Beautiful day out there! Perfect weather for a walk ☀️ #weather #sunshine".to_string())
        } else {
            Ok(format!("Tool executed: {}", content))
        }
    }

    /// Execute a response block that generates AI content
    async fn execute_response_block(&self, content: &str, dependencies: &[String]) -> Result<String> {
        if let Some(_llm_service) = &self._llm_service {
            // Gather context from completed dependencies
            let mut context = String::new();
            for dep_id in dependencies {
                if let Some(dep_block) = self._blocks.iter().find(|b| b.id == *dep_id) {
                    if dep_block.status == BlockStatus::Completed {
                        if let Some(result) = dep_block.get_result() {
                            context.push_str(&format!("{}=>{}\n", dep_block.name, result));
                        }
                    }
                }
            }

            let prompt = format!("{}\n\nContext:\n{}", content, context);
            
            // This is a simplified example - in real usage you'd use the full LLM interface
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            Ok(format!("AI Response: Generated content based on prompt: {}", prompt))
        } else {
            Ok(format!("Response generated: {}", content))
        }
    }

    /// Reset all blocks to pending status
    pub fn reset_workflow(&mut self) {
        for block in &mut self._blocks {
            block.status = BlockStatus::Pending;
            block.updated_at = Utc::now();
        }
    }
}

impl Default for BlockCoordinator {
    fn default() -> Self {
        Self::new()
    }
}