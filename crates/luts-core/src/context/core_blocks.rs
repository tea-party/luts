//! Core context blocks system for persistent context window management
//!
//! This module provides essential context blocks that should always be present
//! in the AI context window, similar to Letta's core memory architecture.

use crate::memory::{MemoryBlock, MemoryContent, BlockType, BlockId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of core context blocks that persist across conversations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoreBlockType {
    /// System prompt and instructions
    SystemPrompt,
    
    /// User persona and key information about the user
    UserPersona,
    
    /// Current task or project context
    TaskContext,
    
    /// Important facts and knowledge to remember
    KeyFacts,
    
    /// User preferences and settings
    UserPreferences,
    
    /// Current conversation summary for long sessions
    ConversationSummary,
    
    /// Active goals and objectives
    ActiveGoals,
    
    /// Working memory for current session
    WorkingMemory,
}

impl CoreBlockType {
    /// Get all core block types in priority order
    pub fn all_types() -> Vec<CoreBlockType> {
        vec![
            CoreBlockType::SystemPrompt,
            CoreBlockType::UserPersona,
            CoreBlockType::TaskContext,
            CoreBlockType::KeyFacts,
            CoreBlockType::UserPreferences,
            CoreBlockType::ConversationSummary,
            CoreBlockType::ActiveGoals,
            CoreBlockType::WorkingMemory,
        ]
    }
    
    /// Get the priority of this core block type (lower = higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            CoreBlockType::SystemPrompt => 0,
            CoreBlockType::UserPersona => 1,
            CoreBlockType::TaskContext => 2,
            CoreBlockType::KeyFacts => 3,
            CoreBlockType::UserPreferences => 4,
            CoreBlockType::ConversationSummary => 5,
            CoreBlockType::ActiveGoals => 6,
            CoreBlockType::WorkingMemory => 7,
        }
    }
    
    /// Get the default content template for this core block type
    pub fn default_template(&self) -> &'static str {
        match self {
            CoreBlockType::SystemPrompt => {
                "You are a helpful AI assistant. Follow the user's instructions carefully and provide accurate, helpful responses."
            },
            CoreBlockType::UserPersona => {
                "User information:\n- Name: [Not provided]\n- Preferences: [Not specified]\n- Context: [New user]"
            },
            CoreBlockType::TaskContext => {
                "Current task context:\n- Project: [Not specified]\n- Goal: [Not defined]\n- Status: [New session]"
            },
            CoreBlockType::KeyFacts => {
                "Important facts to remember:\n[No key facts recorded yet]"
            },
            CoreBlockType::UserPreferences => {
                "User preferences:\n- Communication style: [Not specified]\n- Level of detail: [Not specified]\n- Special requirements: [None]"
            },
            CoreBlockType::ConversationSummary => {
                "Conversation summary:\n[New conversation - no summary yet]"
            },
            CoreBlockType::ActiveGoals => {
                "Active goals:\n[No active goals defined]"
            },
            CoreBlockType::WorkingMemory => {
                "Working memory:\n[Session just started]"
            },
        }
    }
    
    /// Check if this core block type should be automatically created
    pub fn auto_create(&self) -> bool {
        match self {
            CoreBlockType::SystemPrompt => true,
            CoreBlockType::UserPersona => true,
            CoreBlockType::TaskContext => true,
            CoreBlockType::KeyFacts => true,
            CoreBlockType::UserPreferences => true,
            CoreBlockType::ConversationSummary => false, // Created when needed
            CoreBlockType::ActiveGoals => false, // Created when user sets goals
            CoreBlockType::WorkingMemory => true,
        }
    }
}

/// A core context block with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreBlock {
    /// The underlying memory block
    pub memory_block: MemoryBlock,
    
    /// The type of core block
    pub core_type: CoreBlockType,
    
    /// Whether this block is currently active in context
    pub is_active: bool,
    
    /// Maximum token budget for this block
    pub max_tokens: Option<u32>,
    
    /// Whether this block can be automatically updated
    pub auto_update: bool,
    
    /// Last time this block was accessed
    pub last_accessed: u64,
}

impl CoreBlock {
    /// Create a new core block
    pub fn new(
        core_type: CoreBlockType,
        user_id: impl Into<String>,
        content: Option<String>,
    ) -> Self {
        let content_text = content.unwrap_or_else(|| core_type.default_template().to_string());
        let memory_content = MemoryContent::Text(content_text);
        
        let memory_block = MemoryBlock::new(
            BlockType::Custom(core_type as u8),
            user_id,
            memory_content,
        );
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        CoreBlock {
            memory_block,
            core_type,
            is_active: core_type.auto_create(),
            max_tokens: Some(match core_type {
                CoreBlockType::SystemPrompt => 500,
                CoreBlockType::UserPersona => 300,
                CoreBlockType::TaskContext => 400,
                CoreBlockType::KeyFacts => 600,
                CoreBlockType::UserPreferences => 200,
                CoreBlockType::ConversationSummary => 800,
                CoreBlockType::ActiveGoals => 300,
                CoreBlockType::WorkingMemory => 400,
            }),
            auto_update: match core_type {
                CoreBlockType::SystemPrompt => false,
                CoreBlockType::UserPersona => true,
                CoreBlockType::TaskContext => true,
                CoreBlockType::KeyFacts => true,
                CoreBlockType::UserPreferences => true,
                CoreBlockType::ConversationSummary => true,
                CoreBlockType::ActiveGoals => true,
                CoreBlockType::WorkingMemory => true,
            },
            last_accessed: now,
        }
    }
    
    /// Update the content of this core block
    pub fn update_content(&mut self, new_content: String) -> Result<()> {
        self.memory_block.content = MemoryContent::Text(new_content);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
            
        self.memory_block.metadata.updated_at = now;
        self.last_accessed = now;
        
        Ok(())
    }
    
    /// Get the text content of this core block
    pub fn get_text_content(&self) -> Option<&str> {
        self.memory_block.content.as_text()
    }
    
    /// Mark this block as accessed
    pub fn mark_accessed(&mut self) {
        self.last_accessed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }
    
    /// Get the ID of this core block
    pub fn id(&self) -> &BlockId {
        self.memory_block.id()
    }
    
    /// Check if this block is essential (should never be removed)
    pub fn is_essential(&self) -> bool {
        matches!(
            self.core_type,
            CoreBlockType::SystemPrompt | CoreBlockType::UserPersona | CoreBlockType::WorkingMemory
        )
    }
}

/// Configuration for core block management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreBlockConfig {
    /// Total token budget for all core blocks
    pub total_token_budget: u32,
    
    /// Whether to automatically create missing core blocks
    pub auto_create_missing: bool,
    
    /// Whether to automatically update core blocks based on conversation
    pub auto_update_enabled: bool,
    
    /// Minimum number of core blocks to keep active
    pub min_active_blocks: usize,
    
    /// Maximum number of core blocks to keep active
    pub max_active_blocks: usize,
}

impl Default for CoreBlockConfig {
    fn default() -> Self {
        CoreBlockConfig {
            total_token_budget: 3000, // Reserve ~3k tokens for core blocks
            auto_create_missing: true,
            auto_update_enabled: true,
            min_active_blocks: 3, // SystemPrompt, UserPersona, WorkingMemory
            max_active_blocks: 8, // All core block types
        }
    }
}

/// Manager for core context blocks
pub struct CoreBlockManager {
    /// Collection of core blocks indexed by type
    core_blocks: HashMap<CoreBlockType, CoreBlock>,
    
    /// Configuration for core block management
    config: CoreBlockConfig,
    
    /// User ID this manager belongs to
    user_id: String,
}

impl CoreBlockManager {
    /// Create a new core block manager
    pub fn new(user_id: impl Into<String>, config: Option<CoreBlockConfig>) -> Self {
        CoreBlockManager {
            core_blocks: HashMap::new(),
            config: config.unwrap_or_default(),
            user_id: user_id.into(),
        }
    }
    
    /// Initialize core blocks with default templates
    pub fn initialize(&mut self) -> Result<()> {
        if self.config.auto_create_missing {
            for core_type in CoreBlockType::all_types() {
                if core_type.auto_create() && !self.core_blocks.contains_key(&core_type) {
                    let core_block = CoreBlock::new(core_type, &self.user_id, None);
                    self.core_blocks.insert(core_type, core_block);
                }
            }
        }
        Ok(())
    }
    
    /// Get a core block by type
    pub fn get_block(&mut self, core_type: CoreBlockType) -> Option<&mut CoreBlock> {
        if let Some(block) = self.core_blocks.get_mut(&core_type) {
            block.mark_accessed();
            Some(block)
        } else {
            None
        }
    }
    
    /// Update or create a core block
    pub fn update_block(&mut self, core_type: CoreBlockType, content: String) -> Result<()> {
        if let Some(block) = self.core_blocks.get_mut(&core_type) {
            block.update_content(content)?;
        } else {
            let core_block = CoreBlock::new(core_type, &self.user_id, Some(content));
            self.core_blocks.insert(core_type, core_block);
        }
        Ok(())
    }
    
    /// Get all active core blocks sorted by priority
    pub fn get_active_blocks(&mut self) -> Vec<&mut CoreBlock> {
        let mut blocks: Vec<_> = self.core_blocks
            .values_mut()
            .filter(|block| block.is_active)
            .collect();
        
        blocks.sort_by_key(|block| block.core_type.priority());
        blocks
    }
    
    /// Get core blocks formatted for AI context
    pub fn format_for_context(&mut self) -> String {
        let active_blocks = self.get_active_blocks();
        let mut context = String::new();
        
        for block in active_blocks {
            if let Some(content) = block.get_text_content() {
                context.push_str(&format!("# {}\n\n{}\n\n", 
                    format!("{:?}", block.core_type).replace('_', " "), 
                    content
                ));
            }
        }
        
        context
    }
    
    /// Calculate total token usage (approximate)
    pub fn estimate_token_usage(&self) -> u32 {
        self.core_blocks
            .values()
            .filter(|block| block.is_active)
            .map(|block| {
                block.get_text_content()
                    .map(|content| content.len() as u32 / 4) // Rough token estimation
                    .unwrap_or(0)
            })
            .sum()
    }
    
    /// Activate a core block
    pub fn activate_block(&mut self, core_type: CoreBlockType) -> Result<()> {
        if let Some(block) = self.core_blocks.get_mut(&core_type) {
            block.is_active = true;
            Ok(())
        } else {
            // Create and activate if auto-create is enabled
            if self.config.auto_create_missing {
                let core_block = CoreBlock::new(core_type, &self.user_id, None);
                self.core_blocks.insert(core_type, core_block);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Core block {:?} not found and auto-create is disabled", core_type))
            }
        }
    }
    
    /// Deactivate a core block (only if not essential)
    pub fn deactivate_block(&mut self, core_type: CoreBlockType) -> Result<()> {
        if let Some(block) = self.core_blocks.get_mut(&core_type) {
            if block.is_essential() {
                return Err(anyhow::anyhow!("Cannot deactivate essential core block {:?}", core_type));
            }
            block.is_active = false;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Core block {:?} not found", core_type))
        }
    }
    
    /// Auto-manage core blocks based on token budget
    pub fn auto_manage_blocks(&mut self) -> Result<()> {
        let current_usage = self.estimate_token_usage();
        
        // If over budget, deactivate non-essential blocks by access time
        if current_usage > self.config.total_token_budget {
            let mut non_essential_blocks: Vec<_> = self.core_blocks
                .iter()
                .filter(|(_, block)| block.is_active && !block.is_essential())
                .map(|(core_type, block)| (*core_type, block.last_accessed))
                .collect();
            
            // Sort by last accessed time (oldest first)
            non_essential_blocks.sort_by_key(|(_, last_accessed)| *last_accessed);
            
            for (core_type, _) in non_essential_blocks {
                if self.estimate_token_usage() <= self.config.total_token_budget {
                    break;
                }
                self.deactivate_block(core_type)?;
            }
        }
        
        Ok(())
    }
    
    /// Get statistics about core blocks
    pub fn get_stats(&self) -> CoreBlockStats {
        let total_blocks = self.core_blocks.len();
        let active_blocks = self.core_blocks.values().filter(|b| b.is_active).count();
        let token_usage = self.estimate_token_usage();
        
        CoreBlockStats {
            total_blocks,
            active_blocks,
            token_usage,
            token_budget: self.config.total_token_budget,
            budget_utilization: (token_usage as f32 / self.config.total_token_budget as f32) * 100.0,
        }
    }
}

/// Statistics about core block usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreBlockStats {
    /// Total number of core blocks
    pub total_blocks: usize,
    
    /// Number of active core blocks
    pub active_blocks: usize,
    
    /// Current token usage
    pub token_usage: u32,
    
    /// Total token budget
    pub token_budget: u32,
    
    /// Budget utilization percentage
    pub budget_utilization: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_block_creation() {
        let block = CoreBlock::new(CoreBlockType::SystemPrompt, "user1", None);
        assert_eq!(block.core_type, CoreBlockType::SystemPrompt);
        assert!(block.is_active);
        assert!(block.get_text_content().is_some());
    }

    #[test]
    fn test_core_block_manager() {
        let mut manager = CoreBlockManager::new("user1", None);
        manager.initialize().unwrap();
        
        // Should have auto-created core blocks
        assert!(!manager.core_blocks.is_empty());
        
        // Test updating a block
        manager.update_block(
            CoreBlockType::UserPersona, 
            "User is a software developer interested in AI".to_string()
        ).unwrap();
        
        let context = manager.format_for_context();
        assert!(context.contains("software developer"));
    }

    #[test]
    fn test_core_block_priorities() {
        assert!(CoreBlockType::SystemPrompt.priority() < CoreBlockType::WorkingMemory.priority());
        assert!(CoreBlockType::UserPersona.priority() < CoreBlockType::ActiveGoals.priority());
    }
}