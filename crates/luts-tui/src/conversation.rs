//! Conversation TUI component for chatting with agents

use crate::{components::show_popup, events::AppEvent, markdown::SimpleMarkdownRenderer};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use luts_core::agents::{Agent, AgentMessage};
use luts_core::llm::LLMService;
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
}

#[derive(Clone, Debug)]
pub struct ToolCall {
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
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
        }
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

    pub fn get_or_render_lines(&mut self, markdown_renderer: &SimpleMarkdownRenderer) -> &Vec<Line<'static>> {
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
                        lines.push(Line::from(Span::styled(
                            format!("  💭 {}", reasoning_line),
                            Style::default().fg(Color::Yellow),
                        )));
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
                    lines.push(Line::from(vec![
                        Span::styled("  🛠 ".to_string(), Style::default().fg(Color::Magenta)),
                        Span::styled(
                            tool_call.name.clone(),
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("({})", tool_call.arguments),
                            Style::default().fg(Color::Gray),
                        ),
                    ]));

                    if let Some(result) = &tool_call.result {
                        lines.push(Line::from(Span::styled(
                            format!("    → {}", result),
                            Style::default().fg(Color::Green),
                        )));
                    }
                }
                lines.push(Line::from("".to_string()));
            }

            // Main content
            if self.is_markdown {
                let markdown_text = markdown_renderer.render(&self.content);
                // Convert the lines to owned strings to satisfy 'static lifetime
                for line in markdown_text.lines {
                    let owned_spans: Vec<Span<'static>> = line.spans
                        .into_iter()
                        .map(|span| Span::styled(span.content.into_owned(), span.style))
                        .collect();
                    lines.push(Line::from(owned_spans));
                }
            } else {
                let content_lines = self.content.lines();
                for content_line in content_lines {
                    lines.push(Line::from(Span::raw(content_line.to_string())));
                }
            }

            self.cached_lines = Some(lines);
        }
        
        self.cached_lines.as_ref().unwrap()
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
    _streaming_content: String,
    _streaming_reasoning: String,
    _current_tool_calls: Vec<ToolCall>,
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
            _streaming_content: String::new(),
            _streaming_reasoning: String::new(),
            _current_tool_calls: Vec::new(),
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

    pub fn set_llm_service(&mut self, llm_service: Arc<LLMService>) {
        self.llm_service = Some(llm_service);
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                // Check if click is within the chat area
                if let Some(area) = self.chat_area {
                    if mouse.column >= area.x && mouse.column < area.x + area.width
                        && mouse.row >= area.y && mouse.row < area.y + area.height {
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

                match agent_clone.write().await.process_message(agent_message).await {
                    Ok(response) => {
                        if response.success {
                            let _ = event_sender_clone.send(AppEvent::AgentResponseReceived(response.content));
                        } else {
                            let error_msg = response
                                .error
                                .unwrap_or_else(|| "Unknown error".to_string());
                            let _ = event_sender_clone.send(AppEvent::AgentResponseError(error_msg));
                        }
                    }
                    Err(e) => {
                        let _ = event_sender_clone.send(AppEvent::AgentResponseError(format!("Agent error: {}", e)));
                    }
                }
                
                // Notify processing finished
                let _ = event_sender_clone.send(AppEvent::AgentProcessingFinished);
            });
        }

        Ok(())
    }

    /// Handle agent response events from background thread
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

        // Create list items from messages using cached rendering
        let items: Vec<ListItem> = self
            .messages
            .iter_mut()
            .map(|msg| {
                let lines = msg.get_or_render_lines(&self.rat_skin).clone();
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
                let agent_title = format!("Conversation with {} ({})", agent_guard.name(), agent_guard.role());
                let tool_list = agent_guard.get_available_tools();
                let tools_str = if tool_list.is_empty() {
                    "Pure reasoning".to_string()
                } else {
                    tool_list.join(", ")
                };
                (agent_title, tools_str)
            } else {
                ("Conversation (Agent Busy)".to_string(), "Loading...".to_string())
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
        let status_text = if self.processing {
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

        let style = if self.processing {
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
