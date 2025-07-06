//! Conversation TUI component for chatting with agents

use crate::{components::show_popup, events::AppEvent, markdown::SimpleMarkdownRenderer};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use futures_util::StreamExt;
use luts_core::agents::{Agent, AgentMessage};
use luts_core::llm::{InternalChatMessage, LLMService};
use luts_core::response_streaming::{ChunkType, ResponseStreamManager};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
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
    Failed(String),
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
                    tool_obj.get("tool_args")
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
                    let result_str = serde_json::to_string(result).unwrap_or_else(|_| chunk_content.to_string());
                    
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
            is_streaming: false,
            streaming_complete: false,
        }
    }

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
    }

    pub fn get_or_render_lines_with_width(
        &mut self,
        markdown_renderer: &SimpleMarkdownRenderer,
        width: usize,
    ) -> &Vec<Line<'static>> {
        // Invalidate cache if width is different (simple approach)
        if self.cached_lines.is_none() {
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

            // Show tool calls if present
            if !self.tool_calls.is_empty() {
                lines.push(Line::from(Span::styled(
                    "🔧 Tool Calls:".to_string(),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )));

                for tool_call in &self.tool_calls {
                    // Choose icon and color based on status
                    let (status_icon, status_color) = match &tool_call.status {
                        ToolStatus::Running => ("⏳", Color::Yellow),
                        ToolStatus::Completed => ("✅", Color::Green),
                        ToolStatus::Failed(_) => ("❌", Color::Red),
                    };

                    let tool_line = format!("{} {} {}({})", 
                        status_icon, 
                        "🛠", 
                        tool_call.name, 
                        tool_call.arguments
                    );
                    
                    let wrapped_lines = wrap_text(&tool_line, width.saturating_sub(2)); // Account for indent
                    for (i, wrapped_line) in wrapped_lines.iter().enumerate() {
                        if i == 0 {
                            lines.push(Line::from(vec![
                                Span::styled("  ".to_string(), Style::default()),
                                Span::styled(
                                    wrapped_line.clone(),
                                    Style::default()
                                        .fg(status_color)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                Span::styled("    ".to_string(), Style::default()),
                                Span::styled(
                                    wrapped_line.clone(),
                                    Style::default().fg(Color::Gray),
                                ),
                            ]));
                        }
                    }

                    if let Some(result) = &tool_call.result {
                        let result_color = match &tool_call.status {
                            ToolStatus::Completed => Color::Green,
                            ToolStatus::Failed(_) => Color::Red,
                            ToolStatus::Running => Color::Yellow,
                        };
                        
                        let wrapped_results = wrap_text(result, width.saturating_sub(6)); // Account for indent
                        for wrapped_result in wrapped_results {
                            lines.push(Line::from(Span::styled(
                                format!("    → {}", wrapped_result),
                                Style::default().fg(result_color),
                            )));
                        }
                    }
                }
                lines.push(Line::from("".to_string()));
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
        }

        self.cached_lines.as_ref().unwrap()
    }

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
    list_state: ListState,
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
            list_state: ListState::default(),
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
            self.list_state.select(Some(self.messages.len() - 1));
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
                        // Focus the history component and calculate which message was clicked
                        self.focused_component = FocusedComponent::History;
                        self.update_focus_styling();

                        // Calculate which message was clicked (account for borders and content height)
                        let relative_row = mouse.row.saturating_sub(area.y + 1); // +1 for top border

                        // For simplicity, just set selection based on click position
                        // A more sophisticated implementation would account for variable message heights
                        let clicked_index = relative_row.saturating_sub(1) as usize; // -1 for title
                        if !self.messages.is_empty() && clicked_index < self.messages.len() {
                            self.list_state.select(Some(clicked_index));
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.focused_component == FocusedComponent::History {
                    let len = self.messages.len();
                    if len > 0 {
                        let selected = self.list_state.selected().unwrap_or(len);
                        if selected > 0 {
                            self.list_state.select(Some(selected - 1));
                        }
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focused_component == FocusedComponent::History {
                    let len = self.messages.len();
                    if len > 0 {
                        let selected = self.list_state.selected().unwrap_or(0);
                        if selected < len - 1 {
                            self.list_state.select(Some(selected + 1));
                        }
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
                        self.list_state.select(Some(self.messages.len() - 1));
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
        let len = self.messages.len();
        if len == 0 {
            return Ok(());
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = self.list_state.selected().unwrap_or(len);
                if selected > 0 {
                    self.list_state.select(Some(selected - 1));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let selected = self.list_state.selected().unwrap_or(0);
                if selected < len - 1 {
                    self.list_state.select(Some(selected + 1));
                }
            }
            KeyCode::Char('r')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                // Toggle reasoning for selected message (or latest if none selected)
                let selected = self.list_state.selected().unwrap_or(len.saturating_sub(1));
                if let Some(message) = self.messages.get_mut(selected) {
                    if message.reasoning.is_some() {
                        message.toggle_reasoning();
                    }
                }
            }
            KeyCode::Home => {
                self.list_state.select(Some(0));
            }
            KeyCode::End => {
                self.list_state.select(Some(len - 1));
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn send_message_to_agent(&mut self, message: String) -> Result<()> {
        if let Some(llm_service) = &self.llm_service {
            debug!("Sending message with streaming to LLM service: {}", message);

            // Start processing indicator
            self.event_sender.send(AppEvent::AgentProcessingStarted)?;
            self.processing = true;
            self.is_streaming = true;

            // Create streaming message
            let agent_name = if let Some(agent) = &self.agent {
                agent.read().await.name().to_string()
            } else {
                "AI".to_string()
            };

            let streaming_message = ChatMessage::new_streaming(agent_name);
            self.messages.push(streaming_message);
            self.current_streaming_message_idx = Some(self.messages.len() - 1);

            // Auto-scroll to bottom
            if !self.messages.is_empty() {
                self.list_state.select(Some(self.messages.len() - 1));
            }

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
                            let _ = event_sender_clone.send(AppEvent::StreamingChunk(chunk));
                        }
                        let _ = event_sender_clone.send(AppEvent::StreamingComplete);
                    }
                    Err(e) => {
                        let _ = event_sender_clone.send(AppEvent::StreamingError(e.to_string()));
                    }
                }
            });
        } else {
            debug!("No LLM service available, falling back to agent processing");
            // Fallback to original agent processing if no LLM service
            self.send_message_to_agent_fallback(message).await?;
        }

        Ok(())
    }

    /// Fallback method for non-streaming agent processing
    async fn send_message_to_agent_fallback(&mut self, message: String) -> Result<()> {
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
                                .send(AppEvent::AgentResponseReceived(response.content));
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
        }

        Ok(())
    }

    /// Handle streaming chunk events
    pub fn handle_streaming_chunk(
        &mut self,
        chunk: luts_core::response_streaming::ResponseChunk,
    ) -> Result<()> {
        if let Some(idx) = self.current_streaming_message_idx {
            if let Some(message) = self.messages.get_mut(idx) {
                message.append_chunk(&chunk.content, &chunk.chunk_type);

                // Auto-scroll to follow streaming
                if !self.messages.is_empty() {
                    self.list_state.select(Some(self.messages.len() - 1));
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
            }
        }

        self.current_streaming_message_idx = None;
        self.is_streaming = false;
        self.processing = false;

        info!("Streaming error: {}", error);
        Ok(())
    }
    pub async fn handle_agent_response(&mut self, response: String) -> Result<()> {
        if let Some(agent) = &self.agent {
            let agent_name = agent.read().await.name().to_string();
            let agent_msg = ChatMessage::new(agent_name, response);
            self.messages.push(agent_msg);
        }

        // Auto-scroll to bottom
        if !self.messages.is_empty() {
            self.list_state.select(Some(self.messages.len() - 1));
        }

        Ok(())
    }

    /// Handle agent error events from background thread
    pub fn handle_agent_error(&mut self, error: String) {
        let error_msg = ChatMessage::new_plain("System".to_string(), format!("Error: {}", error));
        self.messages.push(error_msg);

        // Auto-scroll to bottom
        if !self.messages.is_empty() {
            self.list_state.select(Some(self.messages.len() - 1));
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
    pub fn spinner_active(&self) -> bool {
        self.is_streaming || self.processing
    }

    /// Get processing state (for external checks)
    pub fn is_processing(&self) -> bool {
        self.processing
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

        // Create list items from messages using cached rendering with width
        let items: Vec<ListItem> = self
            .messages
            .iter_mut()
            .map(|msg| {
                let lines = msg
                    .get_or_render_lines_with_width(&self.rat_skin, available_width)
                    .clone();
                ListItem::new(Text::from(lines))
            })
            .collect();

        let style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Chat History")
                    .border_style(style),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(list, area, &mut self.list_state);

        // Render scrollbar
        self.scroll_state = self.scroll_state.content_length(self.messages.len());
        if let Some(selected) = self.list_state.selected() {
            self.scroll_state = self.scroll_state.position(selected);
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
            "Type your message | Tab: Switch to history | Ctrl+B: Blocks | Ctrl+T: Tools | Ctrl+L: Logs | F1: Help | Esc: Agent selection".to_string()
                }
                FocusedComponent::History => {
            "Navigate history | Tab: Switch to input | Ctrl+B: Blocks | Ctrl+T: Tools | Ctrl+L: Logs | F1: Help | Esc: Agent selection".to_string()
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
}
