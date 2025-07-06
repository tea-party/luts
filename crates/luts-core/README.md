# LUTS Core

The core library for Layered Universal Tiered Storage for AI, providing a flexible and extensible foundation for AI context management with memory blocking capabilities.

## Features

- **Tiered Context Management**: Store and retrieve context data across multiple storage backends
- **Memory Blocks**: Organize AI context into structured blocks for better retrieval and management
- **LLM Service**: Interface with large language models with streaming and tool support
- **Tools API**: Extensible framework for implementing AI tools
- **Fjäll Integration**: Built-in support for Fjäll/LSM-tree as a storage backend
- **Asynchronous API**: Built with Tokio for efficient async operations

## Getting Started

Add luts-core to your `Cargo.toml`:

```toml
[dependencies]
luts-core = "0.1.0"
```

### Context Management

```rust
use luts_core::context::{ContextManager, FjallContextProvider};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a context manager
    let mut manager = ContextManager::new();

    // Add a Fjäll storage provider
    manager.add_provider("fjall", FjallContextProvider::new("./data")?);

    // Store data
    let context_id = "user123";
    let data = serde_json::json!({
        "history": ["message1", "message2"],
        "preferences": {"theme": "dark"}
    });

    manager.store(context_id, &data, None).await?;

    // Retrieve data
    let retrieved = manager.retrieve(context_id, None).await?;
    println!("Retrieved: {:?}", retrieved);

    Ok(())
}
```

### Memory Blocks

```rust
use luts_core::memory::{
    BlockType, FjallMemoryStore, MemoryBlockBuilder, MemoryContent,
    MemoryManager, MemoryQuery, QuerySort,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a memory store and manager
    let store = FjallMemoryStore::new("./memory_data")?;
    let memory_manager = MemoryManager::new(store);

    // Create a memory block
    let block = MemoryBlockBuilder::new()
        .with_type(BlockType::Message)
        .with_user_id("user123")
        .with_session_id("session456")
        .with_content(MemoryContent::Text("Hello, world!".to_string()))
        .build()?;

    // Store the block
    let block_id = memory_manager.store(block).await?;

    // Query for blocks
    let query = MemoryQuery {
        user_id: Some("user123".to_string()),
        ..Default::default()
    };

    let blocks = memory_manager.search(&query).await?;
    
    Ok(())
}
```

### LLM Service with Tools

```rust
use luts_core::llm::{AiService, ChatMessage, LLMService};
use luts_core::tools::calc::MathTool;
use luts_core::tools::search::DDGSearchTool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize LLM service with tools
    let llm_service = LLMService::new(
        Some("You are a helpful assistant."),
        vec![
            Box::new(MathTool),
            Box::new(DDGSearchTool),
        ],
        "DeepSeek-R1-0528",
    )?;

    // Create a conversation
    let messages = vec![
        ChatMessage::user("What is 2 + 2?"),
    ];

    // Generate a response
    let response = llm_service.generate_response(&messages, None).await?;
    println!("Response: {}", response);

    Ok(())
}
```

## Architecture

### Context Management

The Context Manager provides a simple key-value store interface with pluggable backends:

```
User/Application → ContextManager → ContextProvider (Fjäll, Redis, etc.)
```

### Memory System

The Memory System organizes data into blocks with rich metadata:

```
User/Application → MemoryManager → MemoryStore → Blocks (Message, Fact, Summary, etc.)
```

### LLM Service

The LLM Service provides a uniform interface for interacting with language models:

```
User/Application → LLMService → AI Model → Response/Stream
```

## Extending LUTS Core

### Adding a New Context Provider

Implement the `ContextProvider` trait:

```rust
#[async_trait]
impl ContextProvider for MyProvider {
    async fn store(&self, id: &str, data: &Value) -> Result<(), Error> {
        // Implementation
    }

    async fn retrieve(&self, id: &str) -> Result<Option<Value>, Error> {
        // Implementation
    }

    async fn delete(&self, id: &str) -> Result<(), Error> {
        // Implementation
    }

    async fn exists(&self, id: &str) -> Result<bool, Error> {
        // Implementation
    }

    fn name(&self) -> &str {
        "my_provider"
    }
}
```

### Adding a New Tool

Implement the `AiTool` trait:

```rust
#[async_trait]
impl AiTool for MyTool {
    fn name(&self) -> &str {
        "my_tool"
    }

    fn description(&self) -> &str {
        "Description of what my tool does"
    }

    fn schema(&self) -> &str {
        r#"{
            "type": "object",
            "properties": {
                "param1": {
                    "type": "string",
                    "description": "Parameter description"
                }
            },
            "required": ["param1"]
        }"#
    }

    async fn execute(&self, params: Value) -> Result<Value, Error> {
        // Tool implementation
    }
}
```

## License

MIT License