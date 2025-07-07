#!/usr/bin/env cargo

//! Interactive SurrealDB Tool Tester
//! 
//! A standalone binary for testing SurrealDB operations through agent tools
//! without needing an LLM in the loop.

use luts_core::memory::{MemoryManager, SurrealMemoryStore, SurrealConfig};
use luts_core::tools::interactive_tester::InteractiveToolTester;
use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize basic tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Get data directory from args or use default
    let data_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./test_data"));

    println!("🚀 Starting SurrealDB Tool Tester");
    println!("📁 Data directory: {}", data_dir.display());

    // Create data directory if it doesn't exist
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
        println!("📁 Created data directory");
    }

    // Configure SurrealDB
    let db_path = data_dir.join("tool_test.db");
    let config = SurrealConfig::File {
        path: db_path.clone(),
        namespace: "tool_test".to_string(),
        database: "memory".to_string(),
    };

    println!("🗄️  SurrealDB path: {}", db_path.display());

    // Initialize SurrealDB store
    println!("🔧 Initializing SurrealDB...");
    let store = SurrealMemoryStore::new(config).await?;
    store.initialize_schema().await?;
    println!("✅ SurrealDB initialized");

    // Create memory manager
    let memory_manager = Arc::new(
        MemoryManager::new(store)
    );

    // Create interactive tester
    let tester = InteractiveToolTester::new(memory_manager).await?;

    // Start interactive session
    tester.run_interactive_session().await?;

    Ok(())
}