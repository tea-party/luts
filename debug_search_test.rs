#!/usr/bin/env rust-script

//! Simple script to test search tool functionality and debug issues
//! Run with: `rust-script debug_search_test.rs`

use serde_json::json;
use tokio;

#[tokio::main]
async fn main() {
    // Simple test to validate DDGSearchTool functionality
    println!("Testing DDGSearchTool directly...");
    
    // Test parameters
    let test_params = json!({
        "query": "rust programming language latest features",
        "num_results": 3
    });
    
    println!("Test params: {}", test_params);
    println!("This would test the search tool directly to see if it works");
    println!("Issue: AI says 'I will search' but doesn't actually call the search tool");
    println!("");
    println!("Potential causes:");
    println!("1. Tool name mismatch between LLM and agent registry");
    println!("2. LLM provider not configured to use tools");
    println!("3. Tool not properly registered in personality agent");
    println!("4. LLM decides not to use tools despite saying it will");
    println!("");
    println!("Debug logging added to:");
    println!("- personality.rs: Enhanced tool execution debug");
    println!("- llm.rs: Enhanced tool call debug"); 
    println!("- Fixed test naming mismatch in search.rs");
    println!("");
    println!("Next steps:");
    println!("1. Test with actual CLI to see debug output");
    println!("2. Check if specific LLM provider has tool calling issues");
    println!("3. Verify tool registration in agent creation");
}