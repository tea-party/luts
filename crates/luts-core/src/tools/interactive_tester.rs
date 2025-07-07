//! Interactive tool testing framework
//!
//! This module provides a direct interface to test agent tools without LLM intervention.
//! Perfect for debugging SurrealDB operations and tool functionality.

use crate::memory::{MemoryManager, MemoryBlockBuilder, BlockType, MemoryContent, BlockId};
use crate::tools::{
    AiTool, 
    retrieve_context::RetrieveContextTool, 
    modify_core_block::ModifyCoreBlockTool,
    calc::MathTool,
    search::DDGSearchTool,
    website::WebsiteTool,
    semantic_search::SemanticSearchTool,
    delete_block::DeleteBlockTool,
    update_block::UpdateBlockTool,
    block::BlockTool,
};
use anyhow::Result;
use serde_json::json;
use std::io::{self, Write};
use std::sync::Arc;

/// Interactive tool tester for direct tool invocation
pub struct InteractiveToolTester {
    memory_manager: Arc<MemoryManager>,
    // Memory tools
    retrieve_tool: RetrieveContextTool,
    modify_tool: ModifyCoreBlockTool,
    block_tool: BlockTool,
    delete_tool: DeleteBlockTool,
    update_tool: UpdateBlockTool,
    semantic_search_tool: Option<SemanticSearchTool>,
    // Utility tools  
    calc_tool: MathTool,
    search_tool: DDGSearchTool,
    website_tool: WebsiteTool,
}

impl InteractiveToolTester {
    /// Create a new interactive tool tester
    pub async fn new(memory_manager: Arc<MemoryManager>) -> Result<Self> {
        let retrieve_tool = RetrieveContextTool {
            memory_manager: memory_manager.clone(),
        };
        
        let modify_tool = ModifyCoreBlockTool::new("test_user", None);
        
        let block_tool = BlockTool {
            memory_manager: memory_manager.clone(),
        };
        
        let delete_tool = DeleteBlockTool {
            memory_manager: memory_manager.clone(),
        };
        
        let update_tool = UpdateBlockTool {
            memory_manager: memory_manager.clone(),
        };
        
        // Semantic search tool might fail to create, so make it optional
        let semantic_search_tool = SemanticSearchTool::new(memory_manager.clone()).ok();

        Ok(Self {
            memory_manager,
            retrieve_tool,
            modify_tool,
            block_tool,
            delete_tool,
            update_tool,
            semantic_search_tool,
            calc_tool: MathTool,
            search_tool: DDGSearchTool,
            website_tool: WebsiteTool,
        })
    }

    /// Start the interactive testing session
    pub async fn run_interactive_session(&self) -> Result<()> {
        println!("🧪 Interactive Agent Tool Tester");
        println!("=================================");
        println!("Available commands:");
        println!();
        println!("📦 Memory Tools:");
        println!("  1. add-block     - Add a memory block (using block tool)");
        println!("  2. get-block     - Retrieve a memory block by ID");
        println!("  3. search        - Search memory blocks");
        println!("  4. delete-block  - Delete a memory block (using delete tool)");
        println!("  5. update-block  - Update a memory block (using update tool)");
        println!("  6. list-blocks   - List all blocks for user");
        println!("  7. modify-core   - Modify core block");
        println!("  8. semantic      - Semantic search (if available)");
        println!();
        println!("🛠️  Utility Tools:");
        println!("  c. calc          - Calculator tool");
        println!("  w. web-search    - DuckDuckGo search");  
        println!("  s. website       - Fetch website content");
        println!();
        println!("📊 System:");
        println!("  9. stats         - Show memory stats");
        println!("  0. clear-user    - Clear all data for user");
        println!("  h. help          - Show this help");
        println!("  q. quit          - Exit tester");
        println!();

        let mut user_id = "test_user".to_string();
        
        loop {
            print!("🔧 [{}] > ", user_id);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            match input {
                "1" | "add-block" => {
                    if let Err(e) = self.interactive_add_block_tool(&user_id).await {
                        println!("❌ Error adding block: {}", e);
                    }
                }
                "2" | "get-block" => {
                    if let Err(e) = self.interactive_get_block().await {
                        println!("❌ Error getting block: {}", e);
                    }
                }
                "3" | "search" => {
                    if let Err(e) = self.interactive_search(&user_id).await {
                        println!("❌ Error searching: {}", e);
                    }
                }
                "4" | "delete-block" => {
                    if let Err(e) = self.interactive_delete_block_tool().await {
                        println!("❌ Error deleting block: {}", e);
                    }
                }
                "5" | "update-block" => {
                    if let Err(e) = self.interactive_update_block_tool().await {
                        println!("❌ Error updating block: {}", e);
                    }
                }
                "6" | "list-blocks" => {
                    if let Err(e) = self.interactive_list_blocks(&user_id).await {
                        println!("❌ Error listing blocks: {}", e);
                    }
                }
                "7" | "modify-core" => {
                    if let Err(e) = self.interactive_modify_core(&user_id).await {
                        println!("❌ Error modifying core block: {}", e);
                    }
                }
                "8" | "semantic" => {
                    if let Err(e) = self.interactive_semantic_search(&user_id).await {
                        println!("❌ Error with semantic search: {}", e);
                    }
                }
                "c" | "calc" => {
                    if let Err(e) = self.interactive_calc().await {
                        println!("❌ Error with calculator: {}", e);
                    }
                }
                "w" | "web-search" => {
                    if let Err(e) = self.interactive_web_search().await {
                        println!("❌ Error with web search: {}", e);
                    }
                }
                "s" | "website" => {
                    if let Err(e) = self.interactive_website().await {
                        println!("❌ Error fetching website: {}", e);
                    }
                }
                "9" | "stats" => {
                    if let Err(e) = self.interactive_stats(&user_id).await {
                        println!("❌ Error getting stats: {}", e);
                    }
                }
                "0" | "clear-user" => {
                    if let Err(e) = self.interactive_clear_user(&user_id).await {
                        println!("❌ Error clearing user data: {}", e);
                    }
                }
                "user" => {
                    print!("Enter new user ID: ");
                    io::stdout().flush().unwrap();
                    let mut new_user = String::new();
                    io::stdin().read_line(&mut new_user)?;
                    user_id = new_user.trim().to_string();
                    println!("✅ User ID changed to: {}", user_id);
                }
                "h" | "help" => {
                    self.show_help();
                }
                "q" | "quit" | "exit" => {
                    println!("👋 Goodbye!");
                    break;
                }
                "" => continue,
                _ => {
                    println!("❓ Unknown command. Type 'help' for available commands.");
                }
            }
            println!();
        }

        Ok(())
    }

    fn show_help(&self) {
        println!("📦 Memory Tools:");
        println!("  1. add-block     - Add a memory block (using block tool)");
        println!("  2. get-block     - Retrieve a memory block by ID");
        println!("  3. search        - Search memory blocks");
        println!("  4. delete-block  - Delete a memory block (using delete tool)");
        println!("  5. update-block  - Update a memory block (using update tool)");
        println!("  6. list-blocks   - List all blocks for user");
        println!("  7. modify-core   - Modify core block");
        println!("  8. semantic      - Semantic search (if available)");
        println!();
        println!("🛠️  Utility Tools:");
        println!("  c. calc          - Calculator tool");
        println!("  w. web-search    - DuckDuckGo search");  
        println!("  s. website       - Fetch website content");
        println!();
        println!("📊 System:");
        println!("  9. stats         - Show memory stats");
        println!("  0. clear-user    - Clear all data for user");
        println!("  user             - Change user ID");
        println!("  h. help          - Show this help");
        println!("  q. quit          - Exit tester");
    }

    async fn interactive_add_block_tool(&self, user_id: &str) -> Result<()> {
        println!("📝 Add Memory Block (using Block Tool)");
        
        print!("Block type (1=Message, 2=Fact, 3=Summary, 4=Preference, 5=PersonalInfo, 6=Goal, 7=Task): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let block_type_str = match input.trim() {
            "1" => "Message",
            "2" => "Fact",
            "3" => "Summary",
            "4" => "Preference",
            "5" => "PersonalInfo",
            "6" => "Goal",
            "7" => "Task",
            _ => {
                println!("❌ Invalid block type");
                return Ok(());
            }
        };

        print!("Content: ");
        io::stdout().flush().unwrap();
        let mut content = String::new();
        io::stdin().read_line(&mut content)?;
        let content = content.trim().to_string();

        print!("Session ID (optional, press enter to skip): ");
        io::stdout().flush().unwrap();
        let mut session_input = String::new();
        io::stdin().read_line(&mut session_input)?;
        let session_id = if session_input.trim().is_empty() {
            None
        } else {
            Some(session_input.trim().to_string())
        };

        // Use the block tool
        let mut params = json!({
            "user_id": user_id,
            "block_type": block_type_str,
            "content": content
        });

        if let Some(session_id) = session_id {
            params["session_id"] = json!(session_id);
        }

        println!("🔧 Creating block using BlockTool...");
        match self.block_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Block created successfully:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Err(e) => {
                println!("❌ Block creation failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_delete_block_tool(&self) -> Result<()> {
        print!("Block ID to delete: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let block_id = input.trim();

        let params = json!({
            "block_id": block_id
        });

        println!("🗑️  Deleting block using DeleteBlockTool...");
        match self.delete_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Delete result:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Err(e) => {
                println!("❌ Delete failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_update_block_tool(&self) -> Result<()> {
        print!("Block ID to update: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let block_id = input.trim();

        print!("New content: ");
        io::stdout().flush().unwrap();
        let mut content = String::new();
        io::stdin().read_line(&mut content)?;
        let content = content.trim();

        let params = json!({
            "block_id": block_id,
            "content": content
        });

        println!("✏️  Updating block using UpdateBlockTool...");
        match self.update_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Update result:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Err(e) => {
                println!("❌ Update failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_semantic_search(&self, user_id: &str) -> Result<()> {
        if let Some(semantic_tool) = &self.semantic_search_tool {
            print!("Search query: ");
            io::stdout().flush().unwrap();
            let mut query = String::new();
            io::stdin().read_line(&mut query)?;
            let query = query.trim();

            print!("Limit (default 5): ");
            io::stdout().flush().unwrap();
            let mut limit_input = String::new();
            io::stdin().read_line(&mut limit_input)?;
            let limit = if limit_input.trim().is_empty() {
                5
            } else {
                limit_input.trim().parse().unwrap_or(5)
            };

            let params = json!({
                "user_id": user_id,
                "query": query,
                "limit": limit
            });

            println!("🔍 Searching using SemanticSearchTool...");
            match semantic_tool.execute(params).await {
                Ok(result) => {
                    println!("✅ Semantic search results:");
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                Err(e) => {
                    println!("❌ Semantic search failed: {}", e);
                }
            }
        } else {
            println!("❌ Semantic search tool not available (embedding service required)");
        }

        Ok(())
    }

    async fn interactive_calc(&self) -> Result<()> {
        print!("Mathematical expression: ");
        io::stdout().flush().unwrap();
        let mut expression = String::new();
        io::stdin().read_line(&mut expression)?;
        let expression = expression.trim();

        let params = json!({
            "expression": expression
        });

        println!("🧮 Calculating using MathTool...");
        match self.calc_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Result: {}", result);
            }
            Err(e) => {
                println!("❌ Calculation failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_web_search(&self) -> Result<()> {
        print!("Search query: ");
        io::stdout().flush().unwrap();
        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();

        print!("Number of results (default 3, max 10): ");
        io::stdout().flush().unwrap();
        let mut num_input = String::new();
        io::stdin().read_line(&mut num_input)?;
        let num_results = if num_input.trim().is_empty() {
            3
        } else {
            num_input.trim().parse().unwrap_or(3).min(10)
        };

        let params = json!({
            "query": query,
            "num_results": num_results
        });

        println!("🔍 Searching web using DDGSearchTool...");
        match self.search_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Search results:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Err(e) => {
                println!("❌ Web search failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_website(&self) -> Result<()> {
        print!("Website URL: ");
        io::stdout().flush().unwrap();
        let mut url = String::new();
        io::stdin().read_line(&mut url)?;
        let url = url.trim();

        print!("Render format (html/md, default md): ");
        io::stdout().flush().unwrap();
        let mut format_input = String::new();
        io::stdin().read_line(&mut format_input)?;
        let render_format = if format_input.trim().is_empty() {
            "md"
        } else {
            format_input.trim()
        };

        let params = json!({
            "website": url,
            "render": render_format
        });

        println!("🌐 Fetching website using WebsiteTool...");
        match self.website_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Website content:");
                if let Some(content) = result.get("content") {
                    let content_str = content.as_str().unwrap_or("(unable to display)");
                    // Limit output to first 1000 chars to avoid overwhelming the terminal
                    if content_str.len() > 1000 {
                        println!("{}...\n[Content truncated - {} total characters]", 
                               &content_str[..1000], content_str.len());
                    } else {
                        println!("{}", content_str);
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            Err(e) => {
                println!("❌ Website fetch failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_get_block(&self) -> Result<()> {
        print!("Block ID: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let block_id = input.trim();

        // Use the retrieve context tool
        let params = json!({
            "block_id": block_id
        });

        println!("🔍 Retrieving block using RetrieveContextTool...");
        match self.retrieve_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Tool result:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Err(e) => {
                println!("❌ Tool execution failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_search(&self, user_id: &str) -> Result<()> {
        print!("Search query: ");
        io::stdout().flush().unwrap();
        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();

        print!("Block types (comma-separated, e.g., Message,Fact): ");
        io::stdout().flush().unwrap();
        let mut types_input = String::new();
        io::stdin().read_line(&mut types_input)?;
        let types: Vec<&str> = if types_input.trim().is_empty() {
            vec![]
        } else {
            types_input.trim().split(',').map(|s| s.trim()).collect()
        };

        print!("Limit (default 10): ");
        io::stdout().flush().unwrap();
        let mut limit_input = String::new();
        io::stdin().read_line(&mut limit_input)?;
        let limit = if limit_input.trim().is_empty() {
            10
        } else {
            limit_input.trim().parse().unwrap_or(10)
        };

        // Use the retrieve context tool for search
        let mut params = json!({
            "user_id": user_id,
            "query": query,
            "limit": limit
        });

        if !types.is_empty() {
            params["block_types"] = json!(types);
        }

        println!("🔍 Searching using RetrieveContextTool...");
        match self.retrieve_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Search results:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Err(e) => {
                println!("❌ Search failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_list_blocks(&self, user_id: &str) -> Result<()> {
        let params = json!({
            "user_id": user_id,
            "limit": 50
        });

        println!("📋 Listing blocks for user: {}", user_id);
        match self.retrieve_tool.execute(params).await {
            Ok(result) => {
                println!("✅ User blocks:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Err(e) => {
                println!("❌ Failed to list blocks: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_modify_core(&self, user_id: &str) -> Result<()> {
        println!("🔧 Modify Core Block");
        
        print!("Core block type (SystemPrompt/UserPersona/TaskContext/KeyFacts/WorkingMemory/RecentContext/LearningLog/ResponseGuidelines): ");
        io::stdout().flush().unwrap();
        let mut block_type = String::new();
        io::stdin().read_line(&mut block_type)?;
        let block_type = block_type.trim();

        print!("New content: ");
        io::stdout().flush().unwrap();
        let mut content = String::new();
        io::stdin().read_line(&mut content)?;
        let content = content.trim();

        let params = json!({
            "user_id": user_id,
            "block_type": block_type,
            "content": content
        });

        println!("🔧 Modifying core block using ModifyCoreBlockTool...");
        match self.modify_tool.execute(params).await {
            Ok(result) => {
                println!("✅ Core block modified:");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            Err(e) => {
                println!("❌ Core block modification failed: {}", e);
            }
        }

        Ok(())
    }

    async fn interactive_stats(&self, user_id: &str) -> Result<()> {
        let stats = self.memory_manager.get_stats(user_id).await?;
        
        println!("📊 Memory Statistics for {}:", user_id);
        println!("  Total blocks: {}", stats.total_blocks);
        println!("  Total size: {} bytes", stats.total_size_bytes);
        println!("  Last updated: {}", stats.last_updated);
        println!("  Blocks by type:");
        for (block_type, count) in &stats.blocks_by_type {
            println!("    {}: {}", block_type, count);
        }

        Ok(())
    }

    async fn interactive_clear_user(&self, user_id: &str) -> Result<()> {
        print!("⚠️  Are you sure you want to delete ALL data for user '{}'? (yes/no): ", user_id);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if input.trim().to_lowercase() == "yes" {
            let deleted_count = self.memory_manager.clear_user_data(user_id).await?;
            println!("✅ Deleted {} blocks for user {}", deleted_count, user_id);
        } else {
            println!("❌ Operation cancelled");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{SurrealMemoryStore, SurrealConfig};
    use tempfile::TempDir;

    async fn create_test_tester() -> (InteractiveToolTester, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SurrealConfig::File {
            path: db_path,
            namespace: "test".to_string(),
            database: "memory".to_string(),
        };

        let store = SurrealMemoryStore::new(config).await.unwrap();
        store.initialize_schema().await.unwrap();

        let memory_manager = Arc::new(
            MemoryManager::new(store)
        );

        let tester = InteractiveToolTester::new(memory_manager).await.unwrap();
        (tester, temp_dir)
    }

    #[tokio::test]
    async fn test_tool_creation() {
        let (tester, _temp_dir) = create_test_tester().await;
        
        // Test that we can create the tester without errors
        assert_eq!(tester.retrieve_tool.name(), "retrieve_context");
        assert_eq!(tester.modify_tool.name(), "modify_core_block");
    }

    #[tokio::test]
    async fn test_add_and_retrieve_block() {
        let (tester, _temp_dir) = create_test_tester().await;
        
        // Add a block directly through memory manager
        let block = MemoryBlockBuilder::new()
            .with_user_id("test_user")
            .with_type(BlockType::Fact)
            .with_content(MemoryContent::Text("Test fact for tool testing".to_string()))
            .build()
            .unwrap();

        let block_id = tester.memory_manager.store(block).await.unwrap();

        // Test retrieve using the tool
        let params = json!({
            "user_id": "test_user",
            "block_id": block_id.as_str()
        });

        let result = tester.retrieve_tool.execute(params).await.unwrap();
        
        // Verify the result contains our block
        assert!(result.get("blocks").is_some());
        let blocks = result["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["content"]["Text"], "Test fact for tool testing");
    }

    #[tokio::test]
    async fn test_search_functionality() {
        let (tester, _temp_dir) = create_test_tester().await;
        
        // Add multiple blocks        
        for i in 0..3 {
            let block = MemoryBlockBuilder::new()
                .with_user_id("test_user")
                .with_type(BlockType::Fact)
                .with_content(MemoryContent::Text(format!("Searchable fact {}", i)))
                .build()
                .unwrap();
            
            tester.memory_manager.store(block).await.unwrap();
        }

        // Test search using the tool
        let params = json!({
            "user_id": "test_user",
            "query": "Searchable",
            "limit": 10
        });

        let result = tester.retrieve_tool.execute(params).await.unwrap();
        
        // Verify we found the blocks
        let blocks = result["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 3);
    }

    #[tokio::test]
    async fn test_comprehensive_add_remove_workflow() {
        let (tester, _temp_dir) = create_test_tester().await;
        let user_id = "workflow_test_user";

        println!("🧪 Testing comprehensive add/remove workflow");

        // 1. Add multiple blocks of different types
        let mut block_ids = Vec::new();
        
        let block_types = [
            (BlockType::Fact, "Important fact about the user"),
            (BlockType::Message, "User said hello"),
            (BlockType::Summary, "Conversation summary"),
            (BlockType::Preference, "User prefers dark mode"),
        ];

        for (i, (block_type, content)) in block_types.iter().enumerate() {
            let block = MemoryBlockBuilder::new()
                .with_user_id(user_id)
                .with_type(*block_type)
                .with_content(MemoryContent::Text(format!("{} - {}", content, i)))
                .with_session_id(format!("session_{}", i % 2)) // Alternate sessions
                .build()
                .unwrap();

            let block_id = tester.memory_manager.store(block).await.unwrap();
            block_ids.push(block_id);
            println!("✅ Added {:?} block with ID: {}", block_type, block_ids[i].as_str());
        }

        // 2. Verify all blocks are stored by listing them
        let params = serde_json::json!({
            "user_id": user_id,
            "limit": 10
        });

        let result = tester.retrieve_tool.execute(params).await.unwrap();
        let blocks = result["blocks"].as_array().unwrap();
        assert_eq!(blocks.len(), 4, "Should have 4 blocks stored");
        println!("✅ Verified {} blocks are stored", blocks.len());

        // 3. Test searching for specific block types
        let params = serde_json::json!({
            "user_id": user_id,
            "block_types": ["Fact", "Preference"],
            "limit": 10
        });

        let result = tester.retrieve_tool.execute(params).await.unwrap();
        let filtered_blocks = result["blocks"].as_array().unwrap();
        assert_eq!(filtered_blocks.len(), 2, "Should find 2 blocks (Fact + Preference)");
        println!("✅ Found {} blocks when filtering by type", filtered_blocks.len());

        // 4. Delete every other block
        for (i, block_id) in block_ids.iter().enumerate() {
            if i % 2 == 0 {
                let deleted = tester.memory_manager.delete(block_id).await.unwrap();
                assert!(deleted, "Block should be deleted successfully");
                println!("✅ Deleted block: {}", block_id.as_str());
            }
        }

        // 5. Verify remaining blocks
        let params = serde_json::json!({
            "user_id": user_id,
            "limit": 10
        });

        let result = tester.retrieve_tool.execute(params).await.unwrap();
        let remaining_blocks = result["blocks"].as_array().unwrap();
        assert_eq!(remaining_blocks.len(), 2, "Should have 2 blocks remaining");
        println!("✅ Verified {} blocks remain after deletion", remaining_blocks.len());

        // 6. Clear all user data
        let deleted_count = tester.memory_manager.clear_user_data(user_id).await.unwrap();
        assert_eq!(deleted_count, 2, "Should delete remaining 2 blocks");
        println!("✅ Cleared all user data - deleted {} blocks", deleted_count);

        // 7. Final verification - should be empty
        let params = serde_json::json!({
            "user_id": user_id,
            "limit": 10
        });

        let result = tester.retrieve_tool.execute(params).await.unwrap();
        let final_blocks = result["blocks"].as_array().unwrap();
        assert_eq!(final_blocks.len(), 0, "Should have no blocks after clear");
        println!("✅ Verified no blocks remain after clear");

        println!("🎉 Comprehensive workflow test completed successfully!");
    }

    #[tokio::test]
    async fn test_all_tools_available() {
        let (tester, _temp_dir) = create_test_tester().await;
        
        // Test that all tools are properly initialized
        assert_eq!(tester.calc_tool.name(), "calculator");
        assert_eq!(tester.search_tool.name(), "search");
        assert_eq!(tester.website_tool.name(), "website");
        assert_eq!(tester.block_tool.name(), "block");
        assert_eq!(tester.delete_tool.name(), "delete_block");
        assert_eq!(tester.update_tool.name(), "update_block");
        assert_eq!(tester.retrieve_tool.name(), "retrieve_context");
        assert_eq!(tester.modify_tool.name(), "modify_core_block");
        
        // Check if semantic search is available (it might not be due to mock embeddings)
        println!("Semantic search available: {}", tester.semantic_search_tool.is_some());
        
        println!("✅ All tools are properly initialized");
    }

    #[tokio::test]
    async fn test_calculator_tool() {
        let (tester, _temp_dir) = create_test_tester().await;
        
        let params = json!({
            "expression": "2 + 2"
        });

        let result = tester.calc_tool.execute(params).await.unwrap();
        assert_eq!(result, json!(4.0));
        
        println!("✅ Calculator tool works: 2 + 2 = {}", result);
    }

    #[tokio::test]
    async fn test_block_tool_integration() {
        let (tester, _temp_dir) = create_test_tester().await;
        
        let params = json!({
            "user_id": "test_user",
            "block_type": "Fact", 
            "content": "Test fact created via BlockTool",
            "session_id": "test_session"
        });

        let result = tester.block_tool.execute(params).await.unwrap();
        
        // Should return success and block_id
        assert!(result.get("block_id").is_some());
        assert_eq!(result["success"], true);
        assert!(result.get("message").is_some());
        
        println!("✅ Block tool created block: {}", result["block_id"]);
        
        // Verify the block was actually created by retrieving it directly
        let block_id = result["block_id"].as_str().unwrap();
        let block_id_obj = crate::memory::BlockId::new(block_id);
        let retrieved_block = tester.memory_manager.get(&block_id_obj).await.unwrap().unwrap();
        
        assert_eq!(retrieved_block.user_id(), "test_user");
        assert_eq!(
            retrieved_block.content(),
            &crate::memory::MemoryContent::Text("Test fact created via BlockTool".to_string())
        );
        
        // Test the delete tool with the created block
        let block_id = result["block_id"].as_str().unwrap();
        let delete_params = json!({
            "block_id": block_id
        });
        
        let delete_result = tester.delete_tool.execute(delete_params).await.unwrap();
        assert_eq!(delete_result["success"], true);
        
        println!("✅ Delete tool successfully removed block");
    }
}