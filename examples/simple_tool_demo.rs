use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 SurrealDB Tool Tester Example");
    println!("================================");
    println!();
    println!("This tool would let you interactively test SurrealDB operations:");
    println!("• Add and remove memory blocks");
    println!("• Search through blocks");
    println!("• Test relationships");
    println!("• Use agent tools directly");
    println!();
    println!("To run the actual interactive tester, run:");
    println!("  cargo test -p luts-core --lib -- interactive_tester --nocapture");
    println!();
    println!("Or use it programmatically in your code like this:");
    println!();
    println!("```rust");
    println!("use luts_core::memory::{{MemoryManager, SurrealMemoryStore, SurrealConfig}};");
    println!("use luts_core::tools::interactive_tester::InteractiveToolTester;");
    println!();
    println!("let store = SurrealMemoryStore::new(config).await?;");
    println!("store.initialize_schema().await?;");
    println!("let memory_manager = Arc::new(MemoryManager::new(store));");
    println!("let tester = InteractiveToolTester::new(memory_manager).await?;");
    println!("tester.run_interactive_session().await?;");
    println!("```");

    Ok(())
}