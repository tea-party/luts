use anyhow::Result;
use clap::Parser;
use colored::*;
use luts_core::agents::{Agent, AgentMessage, PersonalityAgentBuilder};
use regex::Regex;
use std::io::{self, Write};
use std::path::PathBuf;
use termimad::MadSkin;
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

/// Command-line arguments for the LUTS CLI
#[derive(Parser)]
#[command(
    name = "luts",
    about = "LUTS - Layered Universal Tiered Storage for AI with Personality Agents"
)]
pub struct Args {
    /// Path to the prompt file
    #[clap(short, long)]
    prompt: Option<PathBuf>,

    /// Enable debug mode
    #[clap(short, long)]
    debug: bool,

    /// Path to the data directory
    #[clap(long, default_value = "./data", short_alias = 'f')]
    data_dir: PathBuf,

    /// LLM provider to use
    #[clap(long, default_value = "gemini-2.5-pro", short_alias = 'r')]
    provider: String,

    /// Agent personality to use
    #[clap(long, short_alias = 'a')]
    agent: Option<String>,

    /// List available agent personalities
    #[clap(long)]
    list_agents: bool,
}

/// Replace Markdown links with OSC 8 hyperlinks for supported terminals.
fn add_osc8_hyperlinks(input: &str) -> String {
    let re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let text = &caps[1];
        let url = &caps[2];
        format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text)
    })
    .to_string()
}

/// Show agent selection menu and let user choose
fn select_agent_interactively() -> Result<String> {
    let personalities = PersonalityAgentBuilder::list_personalities();

    println!(
        "{}",
        "🤖 Available LUTS Personality Agents:".bright_cyan().bold()
    );
    println!();

    for (i, (id, name, description)) in personalities.iter().enumerate() {
        println!(
            "{}. {} ({}) - {}",
            (i + 1).to_string().bright_yellow(),
            name.bright_green().bold(),
            id.bright_blue(),
            description.white()
        );
    }
    println!();

    loop {
        print!(
            "{}",
            "Choose an agent (1-5) or type personality name: ".bright_cyan()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Try parsing as number first
        if let Ok(choice) = input.parse::<usize>() {
            if choice >= 1 && choice <= personalities.len() {
                return Ok(personalities[choice - 1].0.to_string());
            }
        }

        // Try matching by name
        let input_lower = input.to_lowercase();
        for (id, name, _) in &personalities {
            if id.to_lowercase() == input_lower || name.to_lowercase() == input_lower {
                return Ok(id.to_string());
            }
        }

        println!("{}", "❌ Invalid choice. Please try again.".red());
    }
}

/// Display agent information
fn display_agent_info(agent: &dyn Agent) {
    println!();
    println!("{}", "🤖 Agent Information".bright_cyan().bold());
    println!(
        "{} {}",
        "Name:".bright_yellow(),
        agent.name().bright_green().bold()
    );
    println!(
        "{} {}",
        "ID:".bright_yellow(),
        agent.agent_id().bright_blue()
    );
    println!(
        "{} {}",
        "Role:".bright_yellow(),
        agent.role().bright_magenta()
    );

    let tools = agent.get_available_tools();
    if !tools.is_empty() {
        println!(
            "{} {}",
            "Tools:".bright_yellow(),
            tools.join(", ").bright_cyan()
        );
    } else {
        println!(
            "{} {}",
            "Tools:".bright_yellow(),
            "Pure reasoning (no tools)".bright_cyan()
        );
    }
    println!();
}

/// Main conversation loop with the selected agent
async fn conversation_loop(mut agent: Box<dyn Agent>) -> Result<()> {
    display_agent_info(agent.as_ref());

    println!(
        "{}",
        "💬 Starting conversation. Type 'quit' or 'exit' to stop.".bright_green()
    );
    println!("{}", "Type '/switch' to change agents.".bright_yellow());
    println!();

    let skin = MadSkin::default();

    loop {
        // Get user input
        print!("{}", "You: ".bright_cyan().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // Handle special commands
        match input.to_lowercase().as_str() {
            "quit" | "exit" => {
                println!("{}", "👋 Goodbye!".bright_green());
                break;
            }
            "/switch" => {
                return Ok(()); // Return to agent selection
            }
            _ => {}
        }

        // Create message for agent
        let message = AgentMessage::new_chat(
            "user".to_string(),
            agent.agent_id().to_string(),
            input.to_string(),
        );

        // Process message with agent
        print!("{}", format!("{}: ", agent.name()).bright_green().bold());
        io::stdout().flush()?;

        match agent.process_message(message).await {
            Ok(response) => {
                if response.success {
                    // Format and display the response with markdown rendering
                    let formatted_content = add_osc8_hyperlinks(&response.content);
                    let rendered = skin.term_text(&formatted_content);
                    println!("{}", rendered);
                } else {
                    println!(
                        "{}",
                        format!(
                            "❌ Error: {}",
                            response
                                .error
                                .unwrap_or_else(|| "Unknown error".to_string())
                        )
                        .red()
                    );
                }
            }
            Err(e) => {
                println!("{}", format!("❌ Agent error: {}", e).red());
            }
        }

        println!();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(if args.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Handle list agents command
    if args.list_agents {
        let personalities = PersonalityAgentBuilder::list_personalities();
        println!(
            "{}",
            "🤖 Available LUTS Personality Agents:".bright_cyan().bold()
        );
        println!();
        for (id, name, description) in personalities {
            println!(
                "• {} ({}) - {}",
                name.bright_green().bold(),
                id.bright_blue(),
                description
            );
        }
        return Ok(());
    }

    // Ensure data directory exists
    std::fs::create_dir_all(&args.data_dir)?;
    let data_dir = args.data_dir.to_string_lossy().to_string();

    info!("Starting LUTS CLI with multiagent support");
    info!("Data directory: {}", data_dir);
    info!("Provider: {}", args.provider);

    // Main application loop
    loop {
        // Determine which agent to use
        let agent_type = if let Some(agent) = &args.agent {
            agent.clone()
        } else {
            select_agent_interactively()?
        };

        // Create the selected agent
        println!(
            "{}",
            format!("🚀 Loading {} agent...", agent_type).bright_yellow()
        );

        let agent =
            match PersonalityAgentBuilder::create_by_type(&agent_type, &data_dir, &args.provider) {
                Ok(agent) => agent,
                Err(e) => {
                    error!("Failed to create agent: {}", e);
                    println!("{}", format!("❌ Failed to create agent: {}", e).red());
                    continue;
                }
            };

        // Start conversation with the agent
        match conversation_loop(agent).await {
            Ok(()) => {
                // User chose to switch agents, continue loop
                continue;
            }
            Err(e) => {
                error!("Conversation error: {}", e);
                println!("{}", format!("❌ Conversation error: {}", e).red());
                break;
            }
        }
    }

    Ok(())
}
