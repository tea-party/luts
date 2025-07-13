//! Conversation TUI component for chatting with agents

use crate::{components::show_popup, events::AppEvent, markdown::SimpleMarkdownRenderer};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use futures_util::StreamExt;
use luts_framework::agents::{Agent, AgentMessage};
use luts_core::llm::{InternalChatMessage, LLMService};
use luts_core::streaming::{ChunkType, ResponseStreamManager};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, info};
use tui_textarea::TextArea;

/// Wrap text to fit within a specified width, breaking at word boundaries when possible
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        // If adding this word would exceed the width, start a new line
        if current_width + word_len + 1 > width && !current_line.is_empty() {
            lines.push(current_line.trim().to_string());
            current_line = String::new();
            current_width = 0;
        }

        // Add the word to the current line
        if !current_line.is_empty() {
            current_line.push(' ');
            current_width += 1;
        }
        current_line.push_str(word);
        current_width += word_len;
    }

    // Add the last line if it's not empty
    if !current_line.is_empty() {
        lines.push(current_line.trim().to_string());
    }

    // If no lines were created, return the original text
    if lines.is_empty() {
        lines.push(text.to_string());
    }

    lines
}

/// Convert spans to plain text for width calculation
fn spans_to_text(spans: &[Span]) -> String {
    spans.iter().map(|span| span.content.as_ref()).collect()
}

#[derive(Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: String,
    pub is_markdown: bool,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub show_reasoning: bool,
    cached_lines: Option<Vec<Line<'static>>>, // Cache rendered lines
    cached_width: Option<usize>, // Track the width used for caching
    // Streaming state
    pub is_streaming: bool,
    pub streaming_complete: bool,
}

#[derive(Clone, Debug)]
pub struct ToolCall {
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub status: ToolStatus,
}

#[derive(Clone, Debug)]
pub enum ToolStatus {
    Running,
    Completed,
    Failed(#[allow(dead_code)] String),
}

impl ChatMessage {
    pub fn new(sender: String, content: String) -> Self {
        Self {
            sender,
            content,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            is_markdown: true,
            reasoning: None,
            tool_calls: Vec::new(),
            show_reasoning: true, // Show reasoning by default
            cached_lines: None,
            cached_width: None,
            is_streaming: false,
            streaming_complete: false,
        }
    }

    /// Create a new streaming message
    pub fn new_streaming(sender: String) -> Self {
        Self {
            sender,
            content: String::new(),
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            is_markdown: true,
            reasoning: None,
            tool_calls: Vec::new(),
            show_reasoning: true,
            cached_lines: None,
            cached_width: None,
            is_streaming: true,
            streaming_complete: false,
        }
    }

    /// Append content from a streaming chunk
    pub fn append_chunk(&mut self, chunk_content: &str, chunk_type: &ChunkType) {
        match chunk_type {
            ChunkType::Text => {
                self.content.push_str(chunk_content);
                self.cached_lines = None; // Invalidate cache
                self.cached_width = None; // Invalidate width cache
            }
            ChunkType::ToolCall => {
                // Parse tool call information from chunk_content
                if let Some(tool_call) = self.parse_tool_call_chunk(chunk_content) {
                    self.tool_calls.push(tool_call);
                } else {
                    // Fallback: add as regular content
                    self.content.push_str("\n");
                    self.content.push_str(chunk_content);
                }
                self.cached_lines = None;
                self.cached_width = None;
            }
            ChunkType::ToolResponse => {
                // Parse and update the last tool call with the result and status
                if let Some((result, status)) = self.parse_tool_result_chunk(chunk_content) {
                    if let Some(last_tool) = self.tool_calls.last_mut() {
                        if last_tool.result.is_none() {
                            last_tool.result = Some(result);
                            last_tool.status = status;
                        }
                    } else {
                        // Fallback: add as regular content
                        self.content.push_str("\n");
                        self.content.push_str(chunk_content);
                    }
                } else {
                    // Fallback: add as regular content
                    self.content.push_str("\n");
                    self.content.push_str(chunk_content);
                }
                self.cached_lines = None;
                self.cached_width = None;
            }
            ChunkType::Complete => {
                self.is_streaming = false;
                self.streaming_complete = true;
            }
            ChunkType::Error => {
                self.content.push_str("\n❌ Error: ");
                self.content.push_str(chunk_content);
                self.is_streaming = false;
                self.streaming_complete = true;
                self.cached_lines = None;
                self.cached_width = None;
            }
            _ => {
                // Handle other chunk types as needed
                debug!("Unhandled chunk type: {:?}", chunk_type);
            }
        }
    }

    /// Parse tool call information from chunk content
    fn parse_tool_call_chunk(&self, chunk_content: &str) -> Option<ToolCall> {
        // Try to parse structured tool call data first
        if chunk_content.starts_with("🔧 Calling") {
            let parts: Vec<&str> = chunk_content.split(" with args: ").collect();
            if parts.len() >= 2 {
                let tool_name = parts[0].replace("🔧 Calling ", "");
                let arguments = parts[1].to_string();
                return Some(ToolCall {
                    name: tool_name,
                    arguments,
                    result: None,
                    status: ToolStatus::Running,
                });
            }
        }

        // Try to parse JSON-structured tool call data
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(chunk_content) {
            if let Some(tool_obj) = parsed.as_object() {
                if let (Some(name), Some(args)) = (
                    tool_obj.get("tool_name").and_then(|v| v.as_str()),
                    tool_obj.get("tool_args"),
                ) {
                    return Some(ToolCall {
                        name: name.to_string(),
                        arguments: serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string()),
                        result: None,
                        status: ToolStatus::Running,
                    });
                }
            }
        }

        None
    }

    /// Parse tool result from chunk content and return both result and status
    fn parse_tool_result_chunk(&self, chunk_content: &str) -> Option<(String, ToolStatus)> {
        // Handle formatted tool results
        if chunk_content.starts_with("✅ Tool result: ") {
            let result = chunk_content.replace("✅ Tool result: ", "");
            return Some((result, ToolStatus::Completed));
        }

        // Handle error results
        if chunk_content.starts_with("❌ Tool error: ") {
            let error = chunk_content.replace("❌ Tool error: ", "");
            return Some((error.clone(), ToolStatus::Failed(error)));
        }

        // Handle JSON-structured tool results
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(chunk_content) {
            if let Some(result_obj) = parsed.as_object() {
                if let Some(result) = result_obj.get("tool_result") {
                    let result_str =
                        serde_json::to_string(result).unwrap_or_else(|_| chunk_content.to_string());

                    // Check if there's an error field
                    if let Some(error) = result_obj.get("error").and_then(|v| v.as_str()) {
                        return Some((result_str, ToolStatus::Failed(error.to_string())));
                    } else {
                        return Some((result_str, ToolStatus::Completed));
                    }
                }
            }
        }

        // Return raw content as fallback (assume success)
        Some((chunk_content.to_string(), ToolStatus::Completed))
    }

    pub fn new_plain(sender: String, content: String) -> Self {
        Self {
            sender,
            content,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            is_markdown: false,
            reasoning: None,
            tool_calls: Vec::new(),
            show_reasoning: true, // Show reasoning by default
            cached_lines: None,
            cached_width: None,
            is_streaming: false,
            streaming_complete: false,
        }
    }

    #[allow(dead_code)]
    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.reasoning = Some(reasoning);
        self
    }

    pub fn add_tool_call(&mut self, tool_call: ToolCall) {
        self.tool_calls.push(tool_call);
    }

    pub fn toggle_reasoning(&mut self) {
        self.show_reasoning = !self.show_reasoning;
        self.cached_lines = None; // Invalidate cache when reasoning visibility changes
        self.cached_width = None; // Also invalidate width cache
    }

    pub fn get_or_render_lines_with_width(
        &mut self,
        markdown_renderer: &SimpleMarkdownRenderer,
        width: usize,
    ) -> &Vec<Line<'static>> {
        // Invalidate cache if width is different or cache doesn't exist
        if self.cached_lines.is_none() || self.cached_width != Some(width) {
            let mut lines = vec![];

            // Header line
            let sender_style = if self.sender == "You" {
                Style::default().fg(Color::Cyan)
            } else if self.sender == "System" {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };

            let header = Line::from(vec![
                Span::styled(
                    format!("[{}] ", self.timestamp),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(format!("{}: ", self.sender), sender_style),
            ]);
            lines.push(header);

            // Show reasoning if present and toggled on
            if let Some(reasoning) = &self.reasoning {
                let reasoning_indicator = if self.show_reasoning {
                    "🧠 Reasoning (Ctrl+R to hide):"
                } else {
                    "🧠 Reasoning available (Ctrl+R to show)"
                };

                lines.push(Line::from(Span::styled(
                    reasoning_indicator.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::ITALIC),
                )));

                if self.show_reasoning {
                    let reasoning_lines = reasoning.lines();
                    for reasoning_line in reasoning_lines {
                        // Wrap reasoning lines to fit width
                        let wrapped_lines = wrap_text(reasoning_line, width.saturating_sub(6)); // Account for indent
                        for wrapped_line in wrapped_lines {
                            lines.push(Line::from(Span::styled(
                                format!("  💭 {}", wrapped_line),
                                Style::default().fg(Color::Yellow),
                            )));
                        }
                    }
                    lines.push(Line::from("".to_string()));
                }
            }

            // Show tool calls if present - detailed display
            if !self.tool_calls.is_empty() {
                for tool_call in &self.tool_calls {
                    // Format: [TOOL] Used `tool_name`: `{args}` -> `{result}`
                    let status_icon = match &tool_call.status {
                        ToolStatus::Running => "⏳",
                        ToolStatus::Completed => "✅",
                        ToolStatus::Failed(_) => "❌",
                    };
                    
                    let tool_text = if let Some(result) = &tool_call.result {
                        // Show tool call with result
                        format!("[TOOL] {} Used `{}`: `{}` -> `{}`", 
                               status_icon, tool_call.name, tool_call.arguments, result)
                    } else {
                        // Show tool call without result (still running)
                        format!("[TOOL] {} Used `{}`: `{}`", 
                               status_icon, tool_call.name, tool_call.arguments)
                    };
                    
                    let tool_color = match &tool_call.status {
                        ToolStatus::Running => Color::Yellow,
                        ToolStatus::Completed => Color::Cyan,
                        ToolStatus::Failed(_) => Color::Red,
                    };
                    
                    // Wrap tool text if it's too long
                    let wrapped_lines = wrap_text(&tool_text, width.saturating_sub(2));
                    for wrapped_line in wrapped_lines {
                        lines.push(Line::from(Span::styled(
                            wrapped_line,
                            Style::default()
                                .fg(tool_color)
                                .add_modifier(Modifier::ITALIC),
                        )));
                    }
                }
                lines.push(Line::from("".to_string())); // Empty line for spacing
            }

            // Main content with width-aware wrapping
            if self.is_markdown {
                let markdown_text = markdown_renderer.render(&self.content);
                // Process each markdown line and wrap if necessary
                for line in markdown_text.lines {
                    let line_text = spans_to_text(&line.spans);
                    if line_text.len() > width {
                        // Line is too long, need to wrap
                        let wrapped_lines = wrap_text(&line_text, width);
                        for wrapped_line in wrapped_lines {
                            lines.push(Line::from(Span::raw(wrapped_line)));
                        }
                    } else {
                        // Convert the spans to owned strings to satisfy 'static lifetime
                        let owned_spans: Vec<Span<'static>> = line
                            .spans
                            .into_iter()
                            .map(|span| Span::styled(span.content.into_owned(), span.style))
                            .collect();
                        lines.push(Line::from(owned_spans));
                    }
                }
            } else {
                let content_lines = self.content.lines();
                for content_line in content_lines {
                    let wrapped_lines = wrap_text(content_line, width);
                    for wrapped_line in wrapped_lines {
                        lines.push(Line::from(Span::raw(wrapped_line)));
                    }
                }
            }

            self.cached_lines = Some(lines);
            self.cached_width = Some(width);
        }

        self.cached_lines.as_ref().unwrap()
    }

    #[allow(dead_code)]
    pub fn get_or_render_lines(
        &mut self,
        markdown_renderer: &SimpleMarkdownRenderer,
    ) -> &Vec<Line<'static>> {
        // Use the new width-aware method with a default width
        self.get_or_render_lines_with_width(markdown_renderer, 80)
    }
}

pub struct Conversation {
    agent: Option<Arc<RwLock<Box<dyn Agent>>>>,
    llm_service: Option<Arc<LLMService>>,
    messages: Vec<ChatMessage>,
    textarea: TextArea<'static>,
    event_sender: mpsc::UnboundedSender<AppEvent>,
    focused_component: FocusedComponent,
    show_help: bool,
    processing: bool,
    rat_skin: SimpleMarkdownRenderer,
    scroll_state: ScrollbarState,
    /// Vertical scroll offset in lines for the chat history
    scroll_offset: u16,
    // Streaming with ResponseStreamManager
    stream_manager: Arc<ResponseStreamManager>,
    current_streaming_message_idx: Option<usize>,
    /// Streaming state
    is_streaming: bool,
    /// Spinner for tool execution
    spinner_frame: usize,
    /// Spinner frames
    spinner_frames: [char; 7],
    chat_area: Option<Rect>, // Store chat area for mouse handling
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusedComponent {
    Input,
    History,
}

impl Conversation {
    pub fn new(event_sender: mpsc::UnboundedSender<AppEvent>) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type your message...");
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input (Enter to send, Tab to switch focus)"),
        );

        let rat_skin = SimpleMarkdownRenderer::default();

        Self {
            agent: None,
            llm_service: None,
            messages: Vec::new(),
            textarea,
            event_sender,
            focused_component: FocusedComponent::Input,
            show_help: false,
            processing: false,
            rat_skin,
            scroll_state: ScrollbarState::default(),
            scroll_offset: 0,
            // Initialize streaming components
            stream_manager: Arc::new(ResponseStreamManager::new()),
            current_streaming_message_idx: None,
            is_streaming: false,
            spinner_frame: 0,
            spinner_frames: ['✴', '✦', '✶', '✺', '✶', '✦', '✴'],
            chat_area: None,
        }
    }

    pub fn set_agent(&mut self, agent: Box<dyn Agent>) {
        info!("Setting agent: {} ({})", agent.name(), agent.agent_id());

        // Add welcome message
        let welcome_msg = ChatMessage::new(
            agent.name().to_string(),
            format!(
                "Hello! I'm **{}**, your *{}* agent. How can I help you today?",
                agent.name(),
                agent.role()
            ),
        );
        self.messages.push(welcome_msg);

        // Auto-scroll to bottom
        if !self.messages.is_empty() {
            self.scroll_to_bottom();
        }

        self.agent = Some(Arc::new(RwLock::new(agent)));
    }

    /// Set the LLM service for direct streaming (bypassing agent)
    pub fn set_llm_service(&mut self, llm_service: Arc<LLMService>) {
        self.llm_service = Some(llm_service);
        info!("LLM service set for direct streaming");
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                // Check if click is within the chat area
                if let Some(area) = self.chat_area {
                    if mouse.column >= area.x
                        && mouse.column < area.x + area.width
                        && mouse.row >= area.y
                        && mouse.row < area.y + area.height
                    {
                        // Focus the history component
                        self.focused_component = FocusedComponent::History;
                        self.update_focus_styling();
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.focused_component == FocusedComponent::History {
                    if self.scroll_offset > 0 {
                        self.scroll_offset = self.scroll_offset.saturating_sub(3); // Scroll faster with mouse
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focused_component == FocusedComponent::History {
                    // Use default width for mouse scroll calculation
                    let total_lines = self.calculate_total_lines(80);
                    let visible_height = 20; // Default estimate
                    
                    if total_lines > visible_height {
                        let max_scroll = total_lines - visible_height;
                        self.scroll_offset = (self.scroll_offset + 3).min(max_scroll as u16);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                self.focused_component = match self.focused_component {
                    FocusedComponent::Input => FocusedComponent::History,
                    FocusedComponent::History => FocusedComponent::Input,
                };
                self.update_focus_styling();
            }
            KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            _ => match self.focused_component {
                FocusedComponent::Input => self.handle_input_key(key)?,
                FocusedComponent::History => self.handle_history_key(key)?,
            },
        }
        Ok(())
    }

    fn update_focus_styling(&mut self) {
        let (title, style) = match self.focused_component {
            FocusedComponent::Input => (
                "Input (Enter to send, Tab to switch focus)",
                Style::default().fg(Color::Cyan),
            ),
            FocusedComponent::History => (
                "Input (Tab to switch focus)",
                Style::default().fg(Color::Gray),
            ),
        };

        self.textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(style),
        );
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                let text = self.textarea.lines().join("\n");
                if !text.trim().is_empty() && !self.processing {
                    // Add user message to history
                    let user_msg = ChatMessage::new_plain("You".to_string(), text.clone());
                    self.messages.push(user_msg);

                    // Clear input
                    self.textarea = TextArea::default();
                    self.textarea.set_placeholder_text("Type your message...");
                    self.update_focus_styling();

                    // Auto-scroll to bottom
                    if !self.messages.is_empty() {
                        self.scroll_to_bottom();
                    }

                    self.event_sender.send(AppEvent::MessageSent(text))?;
                    // Note: processing state and message addition is now handled by background task
                }
            }
            _ => {
                // Forward key event directly to textarea
                self.textarea.input(key);
            }
        }
        Ok(())
    }

    fn handle_history_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // Calculate total lines to prevent over-scrolling
                // Use a default width - will be corrected in render
                let total_lines = self.calculate_total_lines(80);
                let visible_height = 20; // Default estimate
                
                if total_lines > visible_height {
                    let max_scroll = total_lines - visible_height;
                    self.scroll_offset = (self.scroll_offset + 1).min(max_scroll as u16);
                }
            }
            KeyCode::Char('r')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                // Toggle reasoning for the most recent message with reasoning
                if let Some(message) = self
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|m| m.reasoning.is_some())
                {
                    message.toggle_reasoning();
                }
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                self.scroll_to_bottom();
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                // Use a default width - will be corrected in render
                let total_lines = self.calculate_total_lines(80);
                let visible_height = 20; // Default estimate
                
                if total_lines > visible_height {
                    let max_scroll = total_lines - visible_height;
                    self.scroll_offset = (self.scroll_offset + 10).min(max_scroll as u16);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn send_message_to_agent(&mut self, message: String) -> Result<()> {
        // Always prefer the agent's own processing over direct LLM service
        if let Some(agent) = &self.agent {
            debug!("Sending message to agent: {}", message);

            // Start processing indicator
            self.event_sender.send(AppEvent::AgentProcessingStarted)?;
            self.processing = true;

            let agent_clone = agent.clone();
            let event_sender_clone = self.event_sender.clone();

            // Spawn agent processing on a separate task
            tokio::spawn(async move {
                let agent_id = agent_clone.read().await.agent_id().to_string();
                let agent_message = AgentMessage::new_chat("user".to_string(), agent_id, message);

                match agent_clone
                    .write()
                    .await
                    .process_message(agent_message)
                    .await
                {
                    Ok(response) => {
                        if response.success {
                            let _ = event_sender_clone
                                .send(AppEvent::AgentResponseReceived(response));
                        } else {
                            let error_msg = response
                                .error
                                .unwrap_or_else(|| "Unknown error".to_string());
                            let _ =
                                event_sender_clone.send(AppEvent::AgentResponseError(error_msg));
                        }
                    }
                    Err(e) => {
                        let _ = event_sender_clone
                            .send(AppEvent::AgentResponseError(format!("Agent error: {}", e)));
                    }
                }

                // Notify processing finished
                let _ = event_sender_clone.send(AppEvent::AgentProcessingFinished);
            });
            
            // Auto-scroll to bottom
            self.scroll_to_bottom();
        } else if let Some(_llm_service) = &self.llm_service {
            debug!("No agent available, falling back to direct LLM service: {}", message);
            // Fallback to direct LLM service only if no agent is available
            self.send_message_to_llm_service_fallback(message).await?;
        } else {
            debug!("No agent or LLM service available");
            let error_msg = ChatMessage::new_plain("System".to_string(), "No agent or LLM service available".to_string());
            self.messages.push(error_msg);
            self.scroll_to_bottom();
        }

        Ok(())
    }

    /// Fallback method for direct LLM service (no agent available)
    async fn send_message_to_llm_service_fallback(&mut self, message: String) -> Result<()> {
        if let Some(llm_service) = &self.llm_service {
            debug!("Sending message with streaming to LLM service: {}", message);

            // Start processing indicator
            self.event_sender.send(AppEvent::AgentProcessingStarted)?;
            self.processing = true;
            self.is_streaming = true;

            // Create streaming message
            let streaming_message = ChatMessage::new_streaming("AI".to_string());
            self.messages.push(streaming_message);
            self.current_streaming_message_idx = Some(self.messages.len() - 1);

            // Prepare messages for LLM
            let mut conversation_messages = Vec::new();
            conversation_messages.push(InternalChatMessage::User { content: message });

            // Start streaming
            let llm_service_clone = llm_service.clone();
            let stream_manager_clone = self.stream_manager.clone();
            let event_sender_clone = self.event_sender.clone();
            let session_id = format!("session_{}", chrono::Utc::now().timestamp_millis());

            tokio::spawn(async move {
                match stream_manager_clone
                    .stream_genai_response(session_id, llm_service_clone, conversation_messages)
                    .await
                {
                    Ok(mut stream) => {
                        // Process streaming chunks
                        while let Some(chunk) = stream.next().await {
                            // if chunk is of type streamcomplete
                            if chunk.chunk_type == ChunkType::Complete {
                                let _ = event_sender_clone.send(AppEvent::StreamingComplete);
                                break;
                            }
                            let _ = event_sender_clone.send(AppEvent::StreamingChunk(chunk));
                        }
                        let _ = event_sender_clone.send(AppEvent::StreamingComplete);
                    }
                    Err(e) => {
                        let _ = event_sender_clone.send(AppEvent::StreamingError(e.to_string()));
                    }
                }
            });
            
            // Auto-scroll to bottom after setting up streaming
            self.scroll_to_bottom();
        }

        Ok(())
    }

    /// Handle streaming chunk events
    pub fn handle_streaming_chunk(
        &mut self,
        chunk: luts_framework::streaming::ResponseChunk,
    ) -> Result<()> {
        if let Some(idx) = self.current_streaming_message_idx {
            if let Some(message) = self.messages.get_mut(idx) {
                message.append_chunk(&chunk.content, &chunk.chunk_type);

                // Auto-scroll to follow streaming
                if !self.messages.is_empty() {
                    self.scroll_to_bottom();
                }

                debug!(
                    "Updated streaming message with chunk: {:?}",
                    chunk.chunk_type
                );
            }
        }
        Ok(())
    }

    /// Handle streaming completion
    pub fn handle_streaming_complete(&mut self) -> Result<()> {
        if let Some(idx) = self.current_streaming_message_idx {
            if let Some(message) = self.messages.get_mut(idx) {
                message.is_streaming = false;
                message.streaming_complete = true;
            }
        }

        self.current_streaming_message_idx = None;
        self.is_streaming = false;
        self.processing = false;

        info!("Streaming completed");
        Ok(())
    }

    /// Handle streaming error
    pub fn handle_streaming_error(&mut self, error: String) -> Result<()> {
        if let Some(idx) = self.current_streaming_message_idx {
            if let Some(message) = self.messages.get_mut(idx) {
                message.content.push_str(&format!("\n❌ Error: {}", error));
                message.is_streaming = false;
                message.streaming_complete = true;
                message.cached_lines = None;
                message.cached_width = None;
            }
        }

        self.current_streaming_message_idx = None;
        self.is_streaming = false;
        self.processing = false;

        info!("Streaming error: {}", error);
        Ok(())
    }
    pub async fn handle_agent_response(&mut self, response: luts_framework::agents::MessageResponse) -> Result<()> {
        if let Some(agent) = &self.agent {
            let agent_name = agent.read().await.name().to_string();
            
            // Create a message with the response content
            let mut agent_msg = ChatMessage::new(agent_name, response.content);
            
            // Add tool calls to the message if any were executed
            for tool_call_info in response.tool_calls {
                let tool_status = if tool_call_info.success {
                    ToolStatus::Completed
                } else {
                    ToolStatus::Failed("Tool execution failed".to_string())
                };
                
                let tool_call = ToolCall {
                    name: tool_call_info.tool_name,
                    arguments: serde_json::to_string(&tool_call_info.tool_args)
                        .unwrap_or_else(|_| "{}".to_string()),
                    result: Some(tool_call_info.tool_result),
                    status: tool_status,
                };
                
                agent_msg.add_tool_call(tool_call);
            }
            
            self.messages.push(agent_msg);
        }

        // Auto-scroll to bottom
        if !self.messages.is_empty() {
            self.scroll_to_bottom();
        }

        Ok(())
    }

    /// Handle agent error events from background thread
    pub fn handle_agent_error(&mut self, error: String) {
        let error_msg = ChatMessage::new_plain("System".to_string(), format!("Error: {}", error));
        self.messages.push(error_msg);

        // Auto-scroll to bottom
        if !self.messages.is_empty() {
            self.scroll_to_bottom();
        }
    }

    /// Handle processing state changes
    pub fn set_processing(&mut self, processing: bool) {
        self.processing = processing;
    }

    /// Update spinner animation
    pub fn update_spinner(&mut self) {
        if self.is_streaming || self.processing {
            self.spinner_frame = (self.spinner_frame + 1) % self.spinner_frames.len();
        }
    }

    /// Get current spinner character
    pub fn get_spinner_char(&self) -> char {
        if self.is_streaming || self.processing {
            self.spinner_frames[self.spinner_frame]
        } else {
            ' '
        }
    }

    /// Check if spinner should be visible
    #[allow(dead_code)]
    pub fn spinner_active(&self) -> bool {
        self.is_streaming || self.processing
    }

    /// Get processing state (for external checks)
    pub fn is_processing(&self) -> bool {
        self.processing
    }
    
    /// Get agent reference for context viewer integration
    pub fn agent(&self) -> Option<Arc<RwLock<Box<dyn Agent>>>> {
        self.agent.clone()
    }
    
    /// Get LLM service reference for context viewer integration
    pub fn llm_service(&self) -> Option<Arc<LLMService>> {
        self.llm_service.clone()
    }
    
    /// Get message history as strings for context viewer
    pub fn get_message_history(&self) -> Vec<String> {
        self.messages.iter().map(|msg| {
            format!("{}: {}", msg.sender, msg.content)
        }).collect()
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),         // Header
                Constraint::Percentage(80), // Chat history
                Constraint::Min(3),         // Input
                Constraint::Min(1),         // Status
            ])
            .split(size);

        // Store the chat area for mouse handling
        self.chat_area = Some(main_chunks[1]);

        // Render header
        self.render_header(frame, main_chunks[0]);

        // Render chat history
        self.render_chat_history(frame, main_chunks[1]);

        // Render input
        frame.render_widget(&self.textarea, main_chunks[2]);

        // Render status bar
        self.render_status(frame, main_chunks[3]);

        // Show help if requested
        if self.show_help {
            show_popup(
                frame,
                "Help - Conversation",
                "Navigation:\n\
                 Tab         - Switch focus (Input/History)\n\
                 Enter       - Send message (when input focused)\n\
                 ↑/k         - Scroll up (when history focused)\n\
                 ↓/j         - Scroll down (when history focused)\n\
                 Home        - Go to first message\n\
                 End         - Go to last message\n\
                 Click       - Focus history and select message\n\
                 \n\
                 Message Features:\n\
                 Ctrl+R      - Toggle reasoning for selected message\n\
                 \n\
                 Mode Switching:\n\
                 Ctrl+B      - Memory Blocks (view/edit AI memory)\n\
                 Ctrl+W      - Context Window (view AI context composition)\n\
                 Ctrl+T      - Tool Activity (monitor AI tool usage)\n\
                 F2          - Configuration\n\
                 Esc         - Back to agent selection\n\
                 \n\
                 System:\n\
                 F1          - Toggle this help\n\
                 Ctrl+Q      - Quit application\n\
                 \n\
                 Note: AI processing runs in background - you can\n\
                 navigate and switch modes while AI is thinking!",
                (65, 45),
            );
        }
    }

    fn render_chat_history(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_component == FocusedComponent::History;

        // Calculate available width for text wrapping (subtract borders and scrollbar)
        let available_width = area.width.saturating_sub(4) as usize; // 2 for borders, 2 for scrollbar

        // Recalculate total lines with actual available width
        let total_lines = self.calculate_total_lines(available_width);
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders

        // Ensure scroll offset is within bounds
        if total_lines > visible_height {
            let max_scroll = total_lines - visible_height;
            self.scroll_offset = self.scroll_offset.min(max_scroll as u16);
        } else {
            self.scroll_offset = 0;
        }

        // Create all lines from all messages
        let mut all_lines: Vec<Line<'static>> = Vec::new();

        for msg in &mut self.messages {
            let msg_lines = msg.get_or_render_lines_with_width(&self.rat_skin, available_width);
            all_lines.extend(msg_lines.clone());
            // Add an empty line between messages for better readability
            all_lines.push(Line::from(""));
        }

        // Remove the last empty line if we added one
        if !all_lines.is_empty() && all_lines.last().unwrap().spans.is_empty() {
            all_lines.pop();
        }

        let style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        // Use Paragraph instead of List to support proper scrolling
        let total_lines = all_lines.len();
        let text = Text::from(all_lines);
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Chat History")
                    .border_style(style),
            )
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false }) // Don't trim to preserve formatting
            .scroll((self.get_scroll_offset(), 0)); // Scroll vertically based on messages

        frame.render_widget(paragraph, area);

        // Update scrollbar with proper positioning
        // The scrollbar needs to know:
        // 1. Total content length (total_lines)
        // 2. Current viewport position (scroll_offset)
        // 3. Viewport size (visible_height)
        
        if total_lines > visible_height {
            // Only show scrollbar if content is larger than viewport
            self.scroll_state = self.scroll_state
                .content_length(total_lines.saturating_sub(visible_height))
                .position(self.get_scroll_offset() as usize);
        } else {
            // No scrolling needed - hide scrollbar by setting content length to 0
            self.scroll_state = self.scroll_state
                .content_length(0)
                .position(0);
        }

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut self.scroll_state,
        );
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        // Note: This is a synchronous render method, but we need async access to the agent
        // For now, we'll use try_read() for non-blocking access
        let (title, tools) = if let Some(agent) = &self.agent {
            if let Ok(agent_guard) = agent.try_read() {
                let agent_title = format!(
                    "Conversation with {} ({})",
                    agent_guard.name(),
                    agent_guard.role()
                );
                let tool_list = agent_guard.get_available_tools();
                let tools_str = if tool_list.is_empty() {
                    "Pure reasoning".to_string()
                } else {
                    tool_list.join(", ")
                };
                (agent_title, tools_str)
            } else {
                (
                    "Conversation (Agent Busy)".to_string(),
                    "Loading...".to_string(),
                )
            }
        } else {
            ("No Agent Selected".to_string(), "N/A".to_string())
        };

        let content = Text::from(vec![
            Line::from(vec![
                Span::styled("Agent: ", Style::default().fg(Color::Cyan)),
                Span::styled(title, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Tools: ", Style::default().fg(Color::Cyan)),
                Span::styled(tools, Style::default().fg(Color::Yellow)),
            ]),
        ]);

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Agent Information")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect) {
        let status_text = if self.is_streaming {
            // Show streaming indicator
            let spinner_char = self.get_spinner_char();
            format!("{} Streaming response...", spinner_char)
        } else if self.processing {
            // Show spinner when processing
            let spinner_char = self.get_spinner_char();
            format!("{} Processing...", spinner_char)
        } else {
            match self.focused_component {
                FocusedComponent::Input => {
            "Type your message | Tab: Switch to history | Ctrl+B: Blocks | Ctrl+W: Context | Ctrl+T: Tools | Ctrl+L: Logs | F1: Help | Esc: Agent selection".to_string()
                }
                FocusedComponent::History => {
            "Navigate history | Tab: Switch to input | Ctrl+B: Blocks | Ctrl+W: Context | Ctrl+T: Tools | Ctrl+L: Logs | F1: Help | Esc: Agent selection".to_string()
                }
            }
        };

        let style = if self.is_streaming {
            Style::default().fg(Color::Cyan)
        } else if self.processing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };

        let paragraph = Paragraph::new(status_text)
            .style(style)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Get the current scroll offset for the paragraph widget
    fn get_scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    /// Calculate the total number of lines in all messages with given width
    fn calculate_total_lines(&mut self, available_width: usize) -> usize {
        let mut total_lines = 0;
        
        for msg in &mut self.messages {
            let msg_lines = msg.get_or_render_lines_with_width(&self.rat_skin, available_width);
            total_lines += msg_lines.len() + 1; // +1 for empty line between messages
        }
        
        total_lines.saturating_sub(1) // Remove the last empty line
    }

    /// Scroll to the bottom of the chat history
    fn scroll_to_bottom(&mut self) {
        // Use a default width if we don't have access to the actual render area
        // This will be overridden in render_chat_history with the actual width
        let default_width = 80;
        let default_height = 20; // Default visible height estimate
        let total_lines = self.calculate_total_lines(default_width);
        
        if total_lines > default_height {
            // Set scroll offset to show the bottom content
            self.scroll_offset = (total_lines - default_height) as u16;
        } else {
            self.scroll_offset = 0;
        }
    }

    /// Scroll to the bottom with specific width (called from render)
    #[allow(dead_code)]
    fn scroll_to_bottom_with_width(&mut self, available_width: usize, visible_height: usize) {
        let total_lines = self.calculate_total_lines(available_width);
        
        if total_lines > visible_height {
            // Set scroll offset to show the bottom content
            self.scroll_offset = (total_lines - visible_height) as u16;
        } else {
            self.scroll_offset = 0;
        }
    }
}
