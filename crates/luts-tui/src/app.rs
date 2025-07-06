//! Main TUI application state and event loop

use crate::{
    agent_selector::AgentSelector,
    block_mode::BlockMode,
    config_manager::ConfigManager,
    conversation::Conversation,
    events::{AppEvent, EventHandler, handle_key_event},
    log_viewer::{LogViewer, LogBuffer, LogBufferLayer},
    tool_activity::ToolActivityPanel,
};
use anyhow::Result;
use luts_core::agents::PersonalityAgentBuilder;
use luts_core::llm::LLMService;
use ratatui::{Terminal, backend::Backend};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppState {
    AgentSelection,
    Conversation,
    BlockMode,
    ToolActivity,
    LogViewer,
    Config,
    Quitting,
}

pub struct App {
    state: AppState,
    agent_selector: AgentSelector,
    conversation: Conversation,
    block_mode: BlockMode,
    tool_activity: ToolActivityPanel,
    log_viewer: LogViewer,
    config_manager: Option<ConfigManager>,
    event_handler: EventHandler,
    _llm_service: Arc<LLMService>,
    data_dir: String,
    provider: String,
    initial_agent: Option<String>,
    needs_redraw: bool, // Track if we need to redraw
    _log_buffer: LogBuffer, // Keep reference to log buffer
}

impl App {
    pub fn new(data_dir: &str, provider: &str, initial_agent: Option<String>) -> Self {
        // Create log buffer and set up tracing
        let log_buffer = LogBuffer::new(1000); // Keep 1000 log entries
        
        // Set up tracing with our custom layer only (no terminal output)
        let log_layer = LogBufferLayer::new(log_buffer.clone());
        tracing_subscriber::registry()
            .with(log_layer)
            .init();
        
        let event_handler = EventHandler::new(Duration::from_millis(100)); // Reduce from 250ms to 100ms
        let event_sender = event_handler.sender();

        // Create LLM service
        let llm_service = match LLMService::new(None, Vec::new(), provider) {
            Ok(service) => Arc::new(service),
            Err(e) => {
                eprintln!("Failed to create LLM service: {}", e);
                std::process::exit(1);
            }
        };

        let mut conversation = Conversation::new(event_sender.clone());
        conversation.set_llm_service(llm_service.clone());

        Self {
            state: if initial_agent.is_some() {
                AppState::Conversation
            } else {
                AppState::AgentSelection
            },
            agent_selector: AgentSelector::new(event_sender.clone()),
            conversation,
            block_mode: BlockMode::new(event_sender.clone()),
            tool_activity: ToolActivityPanel::new(event_sender.clone()),
            log_viewer: LogViewer::new(log_buffer.clone()),
            config_manager: None,
            event_handler,
            _llm_service: llm_service,
            data_dir: data_dir.to_string(),
            provider: provider.to_string(),
            initial_agent,
            needs_redraw: true, // Initial draw needed
            _log_buffer: log_buffer,
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        info!("Starting LUTS TUI application");

        // If we have an initial agent, load it immediately
        if let Some(agent_id) = &self.initial_agent.clone() {
            match PersonalityAgentBuilder::create_by_type(agent_id, &self.data_dir, &self.provider)
            {
                Ok(agent) => {
                    self.conversation.set_agent(agent);
                    self.state = AppState::Conversation;
                }
                Err(e) => {
                    error!("Failed to create initial agent {}: {}", agent_id, e);
                    self.state = AppState::AgentSelection;
                }
            }
        }

        loop {
            // Handle events
            match self.event_handler.next_event().await? {
                AppEvent::Key(key) => {
                    self.needs_redraw = true; // Key events usually need redraw
                    // Check for global quit commands first
                    if let Some(global_event) = handle_key_event(key) {
                        if let AppEvent::Quit = global_event {
                            self.state = AppState::Quitting;
                            break;
                        }
                    } else {
                        match self.state {
                            AppState::AgentSelection => {
                                // Check for mode switches
                                if matches!(key.code, crossterm::event::KeyCode::Char('b'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::BlockMode;
                                } else if matches!(key.code, crossterm::event::KeyCode::Char('t'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::ToolActivity;
                                } else if matches!(key.code, crossterm::event::KeyCode::F(2))
                                {
                                    // Initialize config manager if needed
                                    if self.config_manager.is_none() {
                                        match ConfigManager::new(self.event_handler.sender()) {
                                            Ok(config_manager) => {
                                                self.config_manager = Some(config_manager);
                                                self.state = AppState::Config;
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to initialize config manager: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        self.state = AppState::Config;
                                    }
                                } else {
                                    self.agent_selector.handle_key_event(key)?;
                                }
                            }
                            AppState::Conversation => {
                                // Check for back to agent selection or mode switches
                                if matches!(key.code, crossterm::event::KeyCode::Char('q'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                    || matches!(key.code, crossterm::event::KeyCode::Esc)
                                {
                                    self.state = AppState::AgentSelection;
                                } else if matches!(key.code, crossterm::event::KeyCode::Char('b'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::BlockMode;
                                } else if matches!(key.code, crossterm::event::KeyCode::Char('t'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::ToolActivity;
                                } else if matches!(key.code, crossterm::event::KeyCode::Char('l'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::LogViewer;
                                } else if matches!(key.code, crossterm::event::KeyCode::F(2))
                                {
                                    if self.config_manager.is_none() {
                                        match ConfigManager::new(self.event_handler.sender()) {
                                            Ok(config_manager) => {
                                                self.config_manager = Some(config_manager);
                                                self.state = AppState::Config;
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to initialize config manager: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        self.state = AppState::Config;
                                    }
                                } else {
                                    self.conversation.handle_key_event(key)?;
                                }
                            }
                            AppState::BlockMode => {
                                // Check for back to agent selection or config
                                if matches!(key.code, crossterm::event::KeyCode::Char('q'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::AgentSelection;
                                } else if matches!(key.code, crossterm::event::KeyCode::Esc) {
                                    self.state = AppState::Conversation;
                                } else if matches!(key.code, crossterm::event::KeyCode::Char('t'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::ToolActivity;
                                } else if matches!(key.code, crossterm::event::KeyCode::F(2))
                                {
                                    if self.config_manager.is_none() {
                                        match ConfigManager::new(self.event_handler.sender()) {
                                            Ok(config_manager) => {
                                                self.config_manager = Some(config_manager);
                                                self.state = AppState::Config;
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to initialize config manager: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        self.state = AppState::Config;
                                    }
                                } else {
                                    self.block_mode.handle_key_event(key)?;
                                }
                            }
                            AppState::ToolActivity => {
                                // Check for back to agent selection
                                if matches!(key.code, crossterm::event::KeyCode::Char('q'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::AgentSelection;
                                } else if matches!(key.code, crossterm::event::KeyCode::Esc) {
                                    self.state = AppState::Conversation;
                                } else if matches!(key.code, crossterm::event::KeyCode::Char('b'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::BlockMode;
                                } else if matches!(key.code, crossterm::event::KeyCode::F(2))
                                {
                                    if self.config_manager.is_none() {
                                        match ConfigManager::new(self.event_handler.sender()) {
                                            Ok(config_manager) => {
                                                self.config_manager = Some(config_manager);
                                                self.state = AppState::Config;
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to initialize config manager: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        self.state = AppState::Config;
                                    }
                                } else {
                                    self.tool_activity.handle_key_event(key)?;
                                }
                            }
                            AppState::LogViewer => {
                                // Check for back to agent selection
                                if matches!(key.code, crossterm::event::KeyCode::Char('q'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::AgentSelection;
                                } else if matches!(key.code, crossterm::event::KeyCode::Esc) {
                                    self.state = AppState::Conversation;
                                } else if matches!(key.code, crossterm::event::KeyCode::Char('b'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::BlockMode;
                                } else if matches!(key.code, crossterm::event::KeyCode::Char('t'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::ToolActivity;
                                } else if matches!(key.code, crossterm::event::KeyCode::F(2))
                                {
                                    if self.config_manager.is_none() {
                                        match ConfigManager::new(self.event_handler.sender()) {
                                            Ok(config_manager) => {
                                                self.config_manager = Some(config_manager);
                                                self.state = AppState::Config;
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to initialize config manager: {}",
                                                    e
                                                );
                                            }
                                        }
                                    } else {
                                        self.state = AppState::Config;
                                    }
                                } else if self.log_viewer.handle_key(key) {
                                    self.needs_redraw = true;
                                }
                            }
                            AppState::Config => {
                                // Check for back to agent selection
                                if matches!(key.code, crossterm::event::KeyCode::Char('q'))
                                    && key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL)
                                {
                                    self.state = AppState::AgentSelection;
                                } else if matches!(key.code, crossterm::event::KeyCode::Esc) {
                                    self.state = AppState::Conversation;
                                } else if let Some(config_manager) = &mut self.config_manager {
                                    config_manager.handle_key_event(key)?;
                                }
                            }
                            AppState::Quitting => break,
                        }
                    }
                }

                AppEvent::AgentSelected(agent_id) => {
                    self.needs_redraw = true;
                    info!("Agent selected: {}", agent_id);
                    match PersonalityAgentBuilder::create_by_type(
                        &agent_id,
                        &self.data_dir,
                        &self.provider,
                    ) {
                        Ok(agent) => {
                            self.conversation.set_agent(agent);
                            self.state = AppState::Conversation;
                        }
                        Err(e) => {
                            error!("Failed to create agent {}: {}", agent_id, e);
                            // Stay in agent selection and maybe show error
                        }
                    }
                }

                AppEvent::MessageSent(message) => {
                    self.needs_redraw = true;
                    debug!("Message sent: {}", message);
                    if let Err(e) = self
                        .conversation
                        .send_message_to_agent(message)
                        .await
                    {
                        error!("Failed to send message to agent: {}", e);
                    }
                }
                
                AppEvent::AgentResponseReceived(response) => {
                    self.needs_redraw = true;
                    debug!("Agent response received");
                    if let Err(e) = self.conversation.handle_agent_response(response).await {
                        error!("Failed to handle agent response: {}", e);
                    }
                }
                
                AppEvent::AgentResponseError(error) => {
                    self.needs_redraw = true;
                    debug!("Agent response error: {}", error);
                    self.conversation.handle_agent_error(error);
                }
                
                AppEvent::AgentProcessingStarted => {
                    self.needs_redraw = true;
                    debug!("Agent processing started");
                    self.conversation.set_processing(true);
                }
                
                AppEvent::AgentProcessingFinished => {
                    self.needs_redraw = true;
                    debug!("Agent processing finished");
                    self.conversation.set_processing(false);
                }

                AppEvent::Quit => {
                    self.state = AppState::Quitting;
                    break;
                }

                AppEvent::Tick => {
                    // Regular tick for animations or periodic updates
                    // Update spinner animation
                    if self.state == AppState::Conversation {
                        self.conversation.update_spinner();
                        // Only redraw if we have an active spinner
                        if self.conversation.is_processing() {
                            self.needs_redraw = true;
                        }
                    }
                }

                AppEvent::Mouse(mouse) => {
                    self.needs_redraw = true;
                    // Handle mouse events based on current state
                    match self.state {
                        AppState::AgentSelection => {
                            self.agent_selector.handle_mouse_event(mouse)?;
                        }
                        AppState::Conversation => {
                            self.conversation.handle_mouse_event(mouse)?;
                        }
                        AppState::BlockMode => {
                            self.block_mode.handle_mouse_event(mouse)?;
                        }
                        AppState::ToolActivity => {
                            self.tool_activity.handle_mouse_event(mouse)?;
                        }
                        AppState::LogViewer => {
                            // Log viewer doesn't need mouse handling for now
                        }
                        AppState::Config => {
                            if let Some(config_manager) = &mut self.config_manager {
                                config_manager.handle_mouse_event(mouse)?;
                            }
                        }
                        AppState::Quitting => {
                            // No mouse handling needed when quitting
                        }
                    }
                }

                AppEvent::Resize(_width, _height) => {
                    self.needs_redraw = true;
                    // Terminal was resized - the next draw call will automatically
                    // re-layout everything with the new dimensions
                    debug!("Terminal resized to {}x{}", _width, _height);
                    // Force a redraw by doing nothing - ratatui will handle the resize
                }
            }

            // Only render if needed to reduce unnecessary redraws
            if self.needs_redraw {
                terminal.draw(|frame| {
                    match self.state {
                        AppState::AgentSelection => {
                            self.agent_selector.render(frame);
                        }
                        AppState::Conversation => {
                            self.conversation.render(frame);
                        }
                        AppState::BlockMode => {
                            self.block_mode.render(frame);
                        }
                        AppState::ToolActivity => {
                            self.tool_activity.render(frame);
                        }
                        AppState::LogViewer => {
                            self.log_viewer.render(frame, frame.area());
                        }
                        AppState::Config => {
                            if let Some(config_manager) = &mut self.config_manager {
                                config_manager.render(frame);
                            }
                        }
                        AppState::Quitting => {
                            // Could show a goodbye message here
                        }
                    }
                })?;
                self.needs_redraw = false; // Reset the flag after rendering
            }

            if self.state == AppState::Quitting {
                break;
            }
        }

        info!("LUTS TUI application exiting");
        Ok(())
    }
}
