//! Streaming Test Mode for LUTS TUI
//!
//! A comprehensive test suite for testing streaming responses, tool calling,
//! and other advanced features in a controlled CLI environment.

use anyhow::Result;
use colored::*;
use futures_util::StreamExt;
use luts_core::{
    llm::{InternalChatMessage, LLMService},
    streaming::{ChunkType, ResponseStreamManager},
    tools::{calc::MathTool, search::DDGSearchTool, website::WebsiteTool},
};
use std::{
    io::{self, Write},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;

/// Available test scenarios
#[derive(Debug, Clone)]
pub struct TestScenario {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Test context containing shared resources
pub struct TestContext {
    #[allow(dead_code)]
    pub data_dir: String,
    #[allow(dead_code)]
    pub provider: String,
    pub llm_service: Arc<LLMService>,
    pub stream_manager: Arc<ResponseStreamManager>,
}

impl TestContext {
    pub async fn new(data_dir: &str, provider: &str) -> Result<Self> {
        // Create LLM service with all tools
        let llm_service = LLMService::new(
            Some(
                "You are a helpful AI assistant with access to various tools. Use tools when appropriate to help answer questions.",
            ),
            vec![
                Box::new(MathTool),
                Box::new(DDGSearchTool),
                Box::new(WebsiteTool),
            ],
            provider,
        )?;

        let stream_manager = Arc::new(ResponseStreamManager::new());

        Ok(Self {
            data_dir: data_dir.to_string(),
            provider: provider.to_string(),
            llm_service: Arc::new(llm_service),
            stream_manager,
        })
    }
}

/// List all available test scenarios
pub fn list_test_scenarios() {
    let scenarios = get_test_scenarios();

    println!(
        "{}",
        "🧪 Available LUTS Streaming Test Scenarios:".bold().cyan()
    );
    println!();

    for scenario in scenarios {
        println!(
            "• {} ({}) - {}",
            scenario.name.green().bold(),
            scenario.id.yellow(),
            scenario.description
        );
    }

    println!();
    println!("{}", "Usage:".bold());
    println!("  luts-tui --test-streaming                    # Interactive mode");
    println!("  luts-tui --test-streaming --test-scenario basic   # Run specific test");
    println!("  luts-tui --list-test-scenarios               # List scenarios");
}

/// Get all available test scenarios
fn get_test_scenarios() -> Vec<TestScenario> {
    vec![
        TestScenario {
            id: "basic".to_string(),
            name: "Basic Streaming".to_string(),
            description: "Test basic streaming response without tools".to_string(),
        },
        TestScenario {
            id: "calculator".to_string(),
            name: "Calculator Tool".to_string(),
            description: "Test streaming with calculator tool calls".to_string(),
        },
        TestScenario {
            id: "web-search".to_string(),
            name: "Web Search Tool".to_string(),
            description: "Test streaming with web search tool calls".to_string(),
        },
        TestScenario {
            id: "multiple-tools".to_string(),
            name: "Multiple Tools".to_string(),
            description: "Test streaming with multiple tool calls in sequence".to_string(),
        },
        TestScenario {
            id: "error-handling".to_string(),
            name: "Error Handling".to_string(),
            description: "Test streaming with tool errors and recovery".to_string(),
        },
        TestScenario {
            id: "stress".to_string(),
            name: "Stress Test".to_string(),
            description: "High-volume streaming and tool calling stress test".to_string(),
        },
    ]
}

/// Main entry point for streaming test mode
pub async fn run_streaming_test(
    data_dir: &str,
    provider: &str,
    scenario: Option<String>,
) -> Result<()> {
    println!("{}", "🧪 LUTS Streaming Test Mode".bold().cyan());
    println!("{}", "=".repeat(50).cyan());
    println!();

    // Initialize test context
    print!("{}", "Initializing test environment... ".yellow());
    io::stdout().flush()?;

    let ctx = TestContext::new(data_dir, provider).await?;

    println!("{}", "✓".green().bold());
    println!("Provider: {}", provider.cyan());
    println!("Data Dir: {}", data_dir.cyan());
    println!("Tools: {}", ctx.llm_service.list_tools().join(", ").cyan());
    println!();

    if let Some(scenario_id) = scenario {
        // Run specific scenario
        run_specific_scenario(&ctx, &scenario_id).await
    } else {
        // Interactive mode
        run_interactive_mode(&ctx).await
    }
}

/// Run a specific test scenario
async fn run_specific_scenario(ctx: &TestContext, scenario_id: &str) -> Result<()> {
    let scenarios = get_test_scenarios();

    if let Some(scenario) = scenarios.iter().find(|s| s.id == scenario_id) {
        println!("{} {}", "Running:".bold(), scenario.name.green().bold());
        println!("{}", scenario.description.italic());
        println!();

        let start_time = Instant::now();

        // Call the appropriate test function based on scenario ID
        let result = match scenario_id {
            "basic" => test_basic_streaming(ctx).await,
            "calculator" => test_calculator_tool(ctx).await,
            "web-search" => test_web_search_tool(ctx).await,
            "multiple-tools" => test_multiple_tools(ctx).await,
            "error-handling" => test_error_handling(ctx).await,
            "stress" => test_stress_scenario(ctx).await,
            _ => Err(anyhow::anyhow!("Unknown test scenario: {}", scenario_id)),
        };

        let duration = start_time.elapsed();

        match result {
            Ok(()) => {
                println!();
                println!(
                    "{} Test completed successfully in {:.2}s",
                    "✓".green().bold(),
                    duration.as_secs_f64()
                );
            }
            Err(e) => {
                println!();
                println!("{} Test failed: {}", "✗".red().bold(), e.to_string().red());
                return Err(e);
            }
        }
    } else {
        println!(
            "{} Unknown scenario: {}",
            "✗".red().bold(),
            scenario_id.red()
        );
        println!();
        list_test_scenarios();
        return Err(anyhow::anyhow!("Unknown test scenario"));
    }

    Ok(())
}

/// Run interactive test mode
async fn run_interactive_mode(ctx: &TestContext) -> Result<()> {
    let scenarios = get_test_scenarios();

    loop {
        println!("{}", "Select a test scenario:".bold());
        println!();

        for (i, scenario) in scenarios.iter().enumerate() {
            println!(
                "{}. {} - {}",
                (i + 1).to_string().cyan().bold(),
                scenario.name.green(),
                scenario.description.italic()
            );
        }

        println!();
        println!("{}. {}", "a".cyan().bold(), "Run all scenarios".green());
        println!("{}. {}", "q".cyan().bold(), "Quit".red());
        println!();

        print!("{} ", "Enter your choice:".bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "q" | "quit" | "exit" => {
                println!("{}", "Goodbye! 👋".yellow());
                break;
            }
            "a" | "all" => {
                for scenario in &scenarios {
                    println!();
                    println!("{}", "─".repeat(60).cyan());
                    run_specific_scenario(ctx, &scenario.id).await?;
                }
                println!();
                println!("{}", "All tests completed! 🎉".green().bold());
                break;
            }
            choice => {
                if let Ok(index) = choice.parse::<usize>() {
                    if index > 0 && index <= scenarios.len() {
                        let scenario = &scenarios[index - 1];
                        println!();
                        println!("{}", "─".repeat(60).cyan());
                        run_specific_scenario(ctx, &scenario.id).await?;
                        println!();
                        println!("{}", "Press Enter to continue...".italic());
                        let mut _dummy = String::new();
                        io::stdin().read_line(&mut _dummy)?;
                    } else {
                        println!("{} Invalid choice. Please try again.", "✗".red().bold());
                    }
                } else {
                    println!("{} Invalid choice. Please try again.", "✗".red().bold());
                }
            }
        }
        println!();
    }

    Ok(())
}

// Test Scenarios Implementation

/// Test basic streaming without tools
async fn test_basic_streaming(ctx: &TestContext) -> Result<()> {
    println!("{}", "Testing basic streaming response...".yellow());

    let messages = vec![InternalChatMessage::User {
        content: "Write a short poem about programming. Make it exactly 4 lines.".to_string(),
    }];

    print_streaming_response(ctx, messages, "Basic Streaming").await
}

/// Test calculator tool functionality
async fn test_calculator_tool(ctx: &TestContext) -> Result<()> {
    println!("{}", "Testing calculator tool...".yellow());

    let messages = vec![InternalChatMessage::User {
        content: "What is 15 * 23 + 47? Please calculate this for me.".to_string(),
    }];

    print_streaming_response(ctx, messages, "Calculator Tool").await
}

/// Test web search tool functionality
async fn test_web_search_tool(ctx: &TestContext) -> Result<()> {
    println!("{}", "Testing web search tool...".yellow());

    let messages = vec![InternalChatMessage::User {
        content: "Search for recent news about Rust programming language updates.".to_string(),
    }];

    print_streaming_response(ctx, messages, "Web Search Tool").await
}

/// Test multiple tool calls in sequence
async fn test_multiple_tools(ctx: &TestContext) -> Result<()> {
    println!("{}", "Testing multiple tool calls...".yellow());

    let messages = vec![InternalChatMessage::User {
        content:
            "First calculate 25 * 4, then search for information about that number in mathematics."
                .to_string(),
    }];

    print_streaming_response(ctx, messages, "Multiple Tools").await
}

/// Test error handling with invalid tool calls
async fn test_error_handling(ctx: &TestContext) -> Result<()> {
    println!("{}", "Testing error handling...".yellow());

    let messages = vec![InternalChatMessage::User {
        content:
            "Calculate the square root of -1 using real numbers only, using the 'calculator' tool."
                .to_string(),
    }];

    print_streaming_response(ctx, messages, "Error Handling").await
}

/// Stress test with rapid tool calls
async fn test_stress_scenario(ctx: &TestContext) -> Result<()> {
    println!("{}", "Running stress test...".yellow());

    let messages = vec![
        InternalChatMessage::User {
            content: "Perform these calculations rapidly: 1+1, 2*3, 4/2, 5-1, 6+4, 7*2, 8/4, 9-3, then search for 'stress testing' and give me a summary.".to_string(),
        }
    ];

    print_streaming_response(ctx, messages, "Stress Test").await
}

/// Helper function to print streaming response with nice formatting
async fn print_streaming_response(
    ctx: &TestContext,
    messages: Vec<InternalChatMessage>,
    _test_name: &str,
) -> Result<()> {
    let session_id = format!("test_session_{}", chrono::Utc::now().timestamp_millis());

    println!("{}:", "Request".bold().blue());
    if let Some(InternalChatMessage::User { content }) = messages.first() {
        println!("  {}", content.italic());
    }
    println!();

    println!("{}:", "Streaming Response".bold().green());

    // Start streaming
    let mut stream = ctx
        .stream_manager
        .stream_genai_response(session_id, ctx.llm_service.clone(), messages)
        .await?;

    let mut total_chunks = 0;
    let mut tool_calls = 0;
    let mut text_content = String::new();
    let start_time = Instant::now();

    // Process stream chunks
    while let Some(chunk) = stream.next().await {
        match chunk.chunk_type {
            ChunkType::Text => {
                if !chunk.content.is_empty() {
                    print!("{}", chunk.content);
                    io::stdout().flush()?;
                    text_content.push_str(&chunk.content);
                }
            }
            ChunkType::ToolCall => {
                tool_calls += 1;
                println!();
                println!("  {}", chunk.content.cyan().bold());
            }
            ChunkType::ToolResponse => {
                println!("  {}", chunk.content.green());
            }
            ChunkType::Reasoning => {
                if !chunk.content.is_empty() {
                    println!();
                    println!("  💭 {}", chunk.content.yellow().italic());
                }
            }
            ChunkType::Error => {
                println!();
                println!("  {}", chunk.content.red().bold());
            }
            ChunkType::Complete => {
                break;
            }
            _ => {}
        }

        total_chunks += 1;

        // Small delay to make streaming visible
        sleep(Duration::from_millis(10)).await;
    }

    let duration = start_time.elapsed();

    println!();
    println!();
    println!("{}:", "Test Results".bold().magenta());
    println!("  Duration: {:.2}s", duration.as_secs_f64());
    println!("  Total chunks: {}", total_chunks);
    println!("  Tool calls: {}", tool_calls);
    println!("  Response length: {} chars", text_content.len());

    if tool_calls > 0 {
        println!("  {}", "✓ Tool calling tested successfully".green());
    }
    if !text_content.is_empty() {
        println!("  {}", "✓ Text streaming tested successfully".green());
    }

    Ok(())
}
