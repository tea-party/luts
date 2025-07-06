//! LUTS API server with OpenAI compatibility
//!
//! This module provides an HTTP API server that is compatible with the OpenAI API,
//! allowing you to use LUTS as a drop-in replacement for OpenAI services.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use clap::Parser;
use luts_core::block_utils::BlockUtils;
use luts_core::llm::LLMService;
use luts_core::tools::calc::MathTool;
use luts_core::tools::search::DDGSearchTool;
use luts_core::tools::website::WebsiteTool;
use tokio::sync::Mutex;
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod api;

/// Command-line arguments for the LUTS API server
#[derive(Parser, Debug)]
#[clap(name = "luts-api", about = "OpenAI-compatible API server for LUTS")]
struct Args {
    /// Path to the prompt file
    #[clap(short, long)]
    prompt: Option<PathBuf>,

    /// Host to bind to
    #[clap(short, long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[clap(short, long, default_value = "3000")]
    port: u16,

    /// Path to the data directory
    #[clap(short, long, default_value = "./data")]
    data_dir: PathBuf,

    /// LLM provider to use
    #[clap(short, long, default_value = "DeepSeek-R1-0528")]
    provider: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Setup tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting LUTS API server...");
    info!("Data directory: {:?}", args.data_dir);
    info!("Provider: {}", args.provider);

    // Ensure data directory exists
    std::fs::create_dir_all(&args.data_dir)?;

    // Get prompt
    let default_prompt = "You are a helpful AI assistant.".to_string();
    let prompt_string = if let Some(prompt_path) = &args.prompt {
        std::fs::read_to_string(prompt_path)?
    } else {
        // Try to load from default location
        std::fs::read_to_string("prompt_api.txt").unwrap_or(default_prompt)
    };

    info!("Using system prompt: {}", prompt_string);

    // Initialize LLM service
    let llm_service = LLMService::new(
        Some(&prompt_string),
        vec![
            Box::new(MathTool),
            Box::new(DDGSearchTool),
            Box::new(WebsiteTool),
        ],
        &args.provider,
    )?;

    // Initialize conversation store (you may want to use a real store)
    let conversation_store = Mutex::new(HashMap::new());

    // Initialize block utils and memory manager (replace with your actual MemoryManager initialization)
    let memory_manager = Arc::new(luts_core::memory::MemoryManager::new(
        luts_core::memory::FjallMemoryStore::new(&args.data_dir).unwrap(),
    ));
    let block_utils = Arc::new(BlockUtils::new(memory_manager));

    // Build shared state for OpenAI endpoints
    let openai_state = api::openai::OpenAIState {
        llm_service,
        _conversation_store: Arc::new(conversation_store),
    };

    // Build shared state for block endpoints
    let block_api_state = api::blocks::ApiState {
        block_utils: block_utils.clone(),
    };

    // Build Axum app with routes from api modules
    let app = Router::new()
        .merge(api::openai::openai_routes(Arc::new(openai_state)))
        .merge(api::blocks::block_routes(block_api_state));

    // Start the server
    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
