//! LUTS TUI - Terminal User Interface for LUTS
//!
//! A modern terminal interface for the LUTS multiagent system using ratatui and ratskin.

use anyhow::Result;
use clap::Parser;
use luts_framework::agents::PersonalityAgentBuilder;
use std::path::PathBuf;
use tracing::info;

mod agent_selector;
mod app;
mod block_mode;
mod components;
mod config;
mod config_manager;
mod context_viewer;
mod conversation;
mod events;
mod log_viewer;
mod markdown;
mod streaming_test;
mod tool_activity;

use app::App;

/// Command-line arguments for the LUTS TUI
#[derive(Parser)]
#[command(
    name = "luts-tui",
    about = "LUTS TUI - Terminal User Interface for AI Agents with Markdown Support"
)]
pub struct Args {
    /// Enable debug mode
    #[clap(short, long)]
    debug: bool,

    /// Path to the data directory
    #[clap(long, default_value = "./data", short_alias = 'f')]
    data_dir: PathBuf,

    /// LLM provider to use
    #[clap(long, default_value = "gemini-2.5-pro", short_alias = 'r')]
    provider: String,

    /// Agent personality to use (skip selection screen)
    #[clap(long, short_alias = 'a')]
    agent: Option<String>,

    /// List available agent personalities
    #[clap(long)]
    list_agents: bool,

    /// Run streaming test mode (for testing streaming, tool calls, etc.)
    #[clap(long)]
    test_streaming: bool,

    /// Test scenario to run (use with --test-streaming)
    #[clap(long, requires = "test_streaming")]
    test_scenario: Option<String>,

    /// List available test scenarios
    #[clap(long)]
    list_test_scenarios: bool,
}

/// Initialize the terminal for TUI mode
pub fn init_terminal()
-> Result<ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let terminal = ratatui::Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to normal mode
pub fn restore_terminal<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut ratatui::Terminal<B>,
) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Run the TUI application
pub async fn run_tui(data_dir: &str, provider: &str, agent: Option<String>) -> Result<()> {
    let mut terminal = init_terminal()?;
    let app_result = App::new(data_dir, provider, agent).run(&mut terminal).await;
    restore_terminal(&mut terminal)?;
    app_result
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    dotenvy::dotenv().ok();

    // Handle list test scenarios command
    if args.list_test_scenarios {
        streaming_test::list_test_scenarios();
        return Ok(());
    }

    // Handle streaming test mode
    if args.test_streaming {
        let data_dir = args.data_dir.to_string_lossy().to_string();
        return streaming_test::run_streaming_test(&data_dir, &args.provider, args.test_scenario).await;
    }

    // Handle list agents command
    if args.list_agents {
        let personalities = PersonalityAgentBuilder::list_personalities();
        println!("🤖 Available LUTS Personality Agents:");
        println!();
        for (id, name, description) in personalities {
            println!("• {} ({}) - {}", name, id, description);
        }
        return Ok(());
    }

    // Ensure data directory exists
    std::fs::create_dir_all(&args.data_dir)?;
    let data_dir = args.data_dir.to_string_lossy().to_string();

    info!("Starting LUTS TUI");
    info!("Data directory: {}", data_dir);
    info!("Provider: {}", args.provider);

    run_tui(&data_dir, &args.provider, args.agent).await
}
