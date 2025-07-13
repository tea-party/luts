//! Context Window Viewer TUI component
//!
//! This module provides a comprehensive viewer for the current context window,
//! showing how memory blocks and core blocks are assembled for AI processing.

use crate::{components::show_popup, events::AppEvent, markdown::SimpleMarkdownRenderer};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use luts_framework::{
    agents::Agent,
};
use luts_core::{
    context::{
        core_blocks::{CoreBlockConfig, CoreBlockManager, CoreBlockType},
        window_manager::{
            ContextWindowConfig, ContextWindowManager, ContextWindowStats, SelectionStrategy,
        },
    },
    llm::LLMService,
    memory::{SurrealMemoryStore, SurrealConfig, MemoryManager},
    utils::tokens::TokenManager,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, ScrollbarState, Wrap},
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusedPanel {
    CoreBlocks,
    DynamicBlocks,
    ContextPreview,
    TokenUsage,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Overview,
    CoreBlocks,
    DynamicBlocks,
    TokenAnalysis,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EditMode {
    None,
    EditingCoreBlock(CoreBlockType),
}

pub struct ContextViewer {
    context_manager: Option<ContextWindowManager>,
    core_block_manager: Option<CoreBlockManager>,
    agent: Option<Arc<RwLock<Box<dyn Agent>>>>,
    llm_service: Option<Arc<LLMService>>,
    memory_manager: Arc<MemoryManager>,
    focused_panel: FocusedPanel,
    view_mode: ViewMode,
    edit_mode: EditMode,
    core_blocks_state: ListState,
    dynamic_blocks_state: ListState,
    #[allow(dead_code)]
    scroll_state: ScrollbarState,
    _event_sender: mpsc::UnboundedSender<AppEvent>,
    show_help: bool,
    #[allow(dead_code)]
    markdown_renderer: SimpleMarkdownRenderer,
    user_id: String,
    session_id: String,
    data_dir: String, // Store the actual data directory

    // Cache data for synchronous rendering
    cached_stats: Option<ContextWindowStats>,
    cached_context: String,
    conversation_history: Vec<String>,
    needs_refresh: bool,

    // Editing state
    edit_content: String,
    edit_cursor_pos: usize,
    show_edit_help: bool,
}

impl ContextViewer {
    pub fn new(event_sender: mpsc::UnboundedSender<AppEvent>) -> Result<Self> {
        // Create a temporary memory manager - will be replaced when initialize_with_data_dir is called
        let temp_config = SurrealConfig::File {
            path: PathBuf::from("./temp/memory.db"),
            namespace: "temp".to_string(),
            database: "memory".to_string(),
        };
        
        let temp_store = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                SurrealMemoryStore::new(temp_config).await.unwrap_or_else(|_| {
                    // If this fails, we'll replace it later anyway
                    panic!("Failed to create temporary memory store")
                })
            })
        });

        let user_id = "default_user".to_string();
        let session_id = "context_viewer_session".to_string();

        let mut core_blocks_state = ListState::default();
        core_blocks_state.select(Some(0));

        let mut dynamic_blocks_state = ListState::default();
        dynamic_blocks_state.select(Some(0));

        Ok(Self {
            context_manager: None, // Will be initialized when agent is set
            core_block_manager: None, // Will be initialized when agent is set
            agent: None,
            llm_service: None,
            memory_manager: Arc::new(MemoryManager::new(temp_store)),
            focused_panel: FocusedPanel::CoreBlocks,
            view_mode: ViewMode::Overview,
            edit_mode: EditMode::None,
            core_blocks_state,
            dynamic_blocks_state,
            scroll_state: ScrollbarState::default(),
            _event_sender: event_sender,
            show_help: false,
            markdown_renderer: SimpleMarkdownRenderer::default(),
            user_id,
            session_id,
            data_dir: "./temp".to_string(), // Will be replaced when initialize_with_data_dir is called
            cached_stats: None,
            cached_context: "# Core Context\n\nNo agent loaded yet. Please select an agent from the main menu to see context information.".to_string(),
            conversation_history: vec![],
            needs_refresh: true,
            edit_content: String::new(),
            edit_cursor_pos: 0,
            show_edit_help: false,
        })
    }

    /// Initialize with proper data directory and memory manager
    pub fn initialize_with_data_dir(&mut self, data_dir: &str) -> Result<()> {
        // Store the data directory for use by context manager initialization
        self.data_dir = data_dir.to_string();
        
        // Initialize storage and managers with SurrealDB using the proper data directory
        let data_path = PathBuf::from(data_dir);
        let surreal_config = SurrealConfig::File {
            path: data_path.join("memory.db"),
            namespace: "luts".to_string(),
            database: "memory".to_string(),
        };
        
        let surreal_store = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                SurrealMemoryStore::new(surreal_config).await.expect("Failed to create SurrealDB store")
            })
        });
        
        self.memory_manager = Arc::new(MemoryManager::new(surreal_store));
        info!("Context viewer initialized with data directory: {}", data_dir);
        Ok(())
    }

    /// Set the agent and initialize context manager
    pub fn set_agent(&mut self, agent: Arc<RwLock<Box<dyn Agent>>>) {
        self.agent = Some(agent);
        self.initialize_context_manager();
        self.needs_refresh = true;
        info!("Agent set for context viewer");
    }

    /// Set the LLM service
    pub fn set_llm_service(&mut self, llm_service: Arc<LLMService>) {
        self.llm_service = Some(llm_service);
        info!("LLM service set for context viewer");
    }

    /// Check if context needs refreshing
    pub fn needs_refresh(&self) -> bool {
        self.needs_refresh
    }

    /// Initialize the context window manager when we have an agent
    fn initialize_context_manager(&mut self) {
        if self.agent.is_some() {
            let data_dir = PathBuf::from(&self.data_dir);
            let token_manager = Arc::new(RwLock::new(TokenManager::new(data_dir)));

            let context_config = ContextWindowConfig {
                max_total_tokens: 8000,
                core_block_tokens: 3000,
                conversation_tokens: 3000,
                dynamic_memory_tokens: 2000,
                max_dynamic_blocks: 10,
                min_relevance_score: 0.3,
                auto_manage: true,
                update_interval: 30,
            };

            let core_config = CoreBlockConfig {
                total_token_budget: 3000,
                auto_create_missing: true,
                auto_update_enabled: true,
                min_active_blocks: 3,
                max_active_blocks: 8,
            };

            let context_manager = ContextWindowManager::new(
                &self.user_id,
                &self.session_id,
                self.memory_manager.clone(),
                token_manager,
                Some(context_config),
                Some(core_config.clone()),
            );

            // Initialize core block manager
            let mut core_block_manager = CoreBlockManager::new(&self.user_id, Some(core_config));
            core_block_manager
                .initialize()
                .expect("Failed to initialize core blocks");

            self.context_manager = Some(context_manager);
            self.core_block_manager = Some(core_block_manager);
            info!("Context window manager and core block manager initialized with data dir: {}", self.data_dir);
        }
    }

    /// Update conversation history from the conversation component
    pub fn update_conversation_history(&mut self, messages: Vec<String>) {
        if messages != self.conversation_history {
            self.conversation_history = messages;
            self.needs_refresh = true;
        }
    }

    pub async fn refresh_context(&mut self) -> Result<()> {
        if let Some(context_manager) = &mut self.context_manager {
            // Update context with current conversation
            context_manager
                .update_context(self.conversation_history.clone())
                .await?;

            // Get fresh stats
            let stats = context_manager.get_stats().await;
            self.cached_stats = Some(stats);

            // Get formatted context
            let formatted_context = context_manager.get_formatted_context().await?;
            self.cached_context = formatted_context;

            self.needs_refresh = false;
            info!("Context refreshed with real data");
        } else {
            // No context manager yet - just mark as refreshed but keep placeholder text
            self.needs_refresh = false;
            info!("Context refresh requested but no context manager available");
        }
        Ok(())
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            KeyCode::F(2) => {
                self.show_edit_help = !self.show_edit_help;
            }
            KeyCode::F(5) => {
                self.refresh_context().await?;
            }
            KeyCode::Esc => {
                // Exit edit mode or help
                if matches!(self.edit_mode, EditMode::EditingCoreBlock(_)) {
                    self.exit_edit_mode();
                } else {
                    self.show_help = false;
                    self.show_edit_help = false;
                }
            }
            KeyCode::Enter => {
                // Enter edit mode for selected core block
                if matches!(self.edit_mode, EditMode::None)
                    && self.focused_panel == FocusedPanel::CoreBlocks
                {
                    if let Some(selected) = self.core_blocks_state.selected() {
                        let core_types = CoreBlockType::all_types();
                        if let Some(core_type) = core_types.get(selected) {
                            self.start_edit_mode(*core_type);
                        }
                    }
                } else if matches!(self.edit_mode, EditMode::EditingCoreBlock(_)) {
                    // Insert newline in edit mode
                    self.edit_content.insert(self.edit_cursor_pos, '\n');
                    self.edit_cursor_pos += 1;
                }
            }
            KeyCode::Tab => {
                if matches!(self.edit_mode, EditMode::None) {
                    self.focused_panel = match self.focused_panel {
                        FocusedPanel::CoreBlocks => FocusedPanel::DynamicBlocks,
                        FocusedPanel::DynamicBlocks => FocusedPanel::ContextPreview,
                        FocusedPanel::ContextPreview => FocusedPanel::TokenUsage,
                        FocusedPanel::TokenUsage => FocusedPanel::CoreBlocks,
                    };
                }
            }
            KeyCode::Char('1') => {
                if matches!(self.edit_mode, EditMode::None) {
                    self.view_mode = ViewMode::Overview;
                }
            }
            KeyCode::Char('2') => {
                if matches!(self.edit_mode, EditMode::None) {
                    self.view_mode = ViewMode::CoreBlocks;
                }
            }
            KeyCode::Char('3') => {
                if matches!(self.edit_mode, EditMode::None) {
                    self.view_mode = ViewMode::DynamicBlocks;
                }
            }
            KeyCode::Char('4') => {
                if matches!(self.edit_mode, EditMode::None) {
                    self.view_mode = ViewMode::TokenAnalysis;
                }
            }
            KeyCode::Char('s') => {
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    if matches!(self.edit_mode, EditMode::EditingCoreBlock(_)) {
                        // Save current edit
                        self.save_current_edit();
                    } else {
                        // Cycle through selection strategies
                        if let Some(context_manager) = &mut self.context_manager {
                            let current_strategy = SelectionStrategy::Balanced; // Would get from manager
                            let new_strategy = match current_strategy {
                                SelectionStrategy::ByRelevance => SelectionStrategy::ByRecency,
                                SelectionStrategy::ByRecency => SelectionStrategy::Balanced,
                                SelectionStrategy::Balanced => SelectionStrategy::ByFrequency,
                                SelectionStrategy::ByFrequency => SelectionStrategy::Diversified,
                                SelectionStrategy::Diversified => SelectionStrategy::ByRelevance,
                            };
                            context_manager.set_selection_strategy(new_strategy);
                            self.needs_refresh = true;
                            info!("Changed selection strategy to: {:?}", new_strategy);
                        } else {
                            info!(
                                "Cannot change selection strategy - no context manager available"
                            );
                        }
                    }
                } else if matches!(self.edit_mode, EditMode::EditingCoreBlock(_)) {
                    // Insert 's' character in edit mode
                    self.edit_content.insert(self.edit_cursor_pos, 's');
                    self.edit_cursor_pos += 1;
                }
            }
            _ => {
                if matches!(self.edit_mode, EditMode::EditingCoreBlock(_)) {
                    self.handle_edit_key(key)?;
                } else {
                    self.handle_navigation_key(key)?;
                }
            }
        }
        Ok(())
    }

    pub fn handle_mouse_event(&mut self, _mouse: MouseEvent) -> Result<()> {
        // Mouse handling for different panels
        Ok(())
    }

    /// Start editing a core block
    fn start_edit_mode(&mut self, core_type: CoreBlockType) {
        self.edit_mode = EditMode::EditingCoreBlock(core_type);

        // Load current content for editing
        if let Some(manager) = &mut self.core_block_manager {
            if let Some(block) = manager.get_block(core_type) {
                self.edit_content = block.get_text_content().unwrap_or("").to_string();
            } else {
                // Use default template for new blocks
                self.edit_content = core_type.default_template().to_string();
            }
        } else {
            self.edit_content = core_type.default_template().to_string();
        }

        self.edit_cursor_pos = self.edit_content.len();
        info!("Started editing core block: {:?}", core_type);
    }

    /// Exit edit mode without saving
    fn exit_edit_mode(&mut self) {
        self.edit_mode = EditMode::None;
        self.edit_content.clear();
        self.edit_cursor_pos = 0;
        info!("Exited edit mode");
    }

    /// Save the current edit to the core block
    fn save_current_edit(&mut self) {
        if let EditMode::EditingCoreBlock(core_type) = self.edit_mode {
            if let Some(manager) = &mut self.core_block_manager {
                match manager.update_block(core_type, self.edit_content.clone()) {
                    Ok(()) => {
                        info!("Saved changes to {:?} core block", core_type);
                        self.needs_refresh = true;
                        self.exit_edit_mode();
                    }
                    Err(e) => {
                        info!("Failed to save core block: {}", e);
                    }
                }
            } else {
                info!("Cannot save - no core block manager available");
            }
        }
    }

    /// Handle navigation keys when not in edit mode
    fn handle_navigation_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => match self.focused_panel {
                FocusedPanel::CoreBlocks => {
                    let selected = self.core_blocks_state.selected().unwrap_or(0);
                    if selected > 0 {
                        self.core_blocks_state.select(Some(selected - 1));
                    }
                }
                FocusedPanel::DynamicBlocks => {
                    let selected = self.dynamic_blocks_state.selected().unwrap_or(0);
                    if selected > 0 {
                        self.dynamic_blocks_state.select(Some(selected - 1));
                    }
                }
                _ => {}
            },
            KeyCode::Down | KeyCode::Char('j') => {
                match self.focused_panel {
                    FocusedPanel::CoreBlocks => {
                        let selected = self.core_blocks_state.selected().unwrap_or(0);
                        let max_items = CoreBlockType::all_types().len().saturating_sub(1);
                        if selected < max_items {
                            self.core_blocks_state.select(Some(selected + 1));
                        }
                    }
                    FocusedPanel::DynamicBlocks => {
                        let selected = self.dynamic_blocks_state.selected().unwrap_or(0);
                        // Would get actual count from context window
                        let max_items = 5; // Placeholder
                        if selected < max_items {
                            self.dynamic_blocks_state.select(Some(selected + 1));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle keys when in edit mode
    fn handle_edit_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Left => {
                if self.edit_cursor_pos > 0 {
                    self.edit_cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.edit_cursor_pos < self.edit_content.len() {
                    self.edit_cursor_pos += 1;
                }
            }
            KeyCode::Home => {
                // Move to start of current line
                let before_cursor = &self.edit_content[..self.edit_cursor_pos];
                if let Some(line_start) = before_cursor.rfind('\n') {
                    self.edit_cursor_pos = line_start + 1;
                } else {
                    self.edit_cursor_pos = 0;
                }
            }
            KeyCode::End => {
                // Move to end of current line
                let after_cursor = &self.edit_content[self.edit_cursor_pos..];
                if let Some(line_end) = after_cursor.find('\n') {
                    self.edit_cursor_pos += line_end;
                } else {
                    self.edit_cursor_pos = self.edit_content.len();
                }
            }
            KeyCode::Up => {
                // Move up one line (simplified)
                let before_cursor = &self.edit_content[..self.edit_cursor_pos];
                if let Some(prev_newline) = before_cursor.rfind('\n') {
                    let current_col = self.edit_cursor_pos - prev_newline - 1;
                    let before_prev = &self.edit_content[..prev_newline];
                    if let Some(prev_prev_newline) = before_prev.rfind('\n') {
                        let prev_line_start = prev_prev_newline + 1;
                        let prev_line_len = prev_newline - prev_prev_newline - 1;
                        let new_col = current_col.min(prev_line_len);
                        self.edit_cursor_pos = prev_line_start + new_col;
                    } else {
                        // First line
                        let new_col = current_col.min(prev_newline);
                        self.edit_cursor_pos = new_col;
                    }
                }
            }
            KeyCode::Down => {
                // Move down one line (simplified)
                let after_cursor = &self.edit_content[self.edit_cursor_pos..];
                if let Some(next_newline) = after_cursor.find('\n') {
                    let before_cursor = &self.edit_content[..self.edit_cursor_pos];
                    let current_line_start =
                        before_cursor.rfind('\n').map(|pos| pos + 1).unwrap_or(0);
                    let current_col = self.edit_cursor_pos - current_line_start;

                    let next_line_start = self.edit_cursor_pos + next_newline + 1;
                    let remaining = &self.edit_content[next_line_start..];
                    let next_line_len = remaining.find('\n').unwrap_or(remaining.len());
                    let new_col = current_col.min(next_line_len);
                    self.edit_cursor_pos = next_line_start + new_col;
                }
            }
            KeyCode::Char(c) => {
                self.edit_content.insert(self.edit_cursor_pos, c);
                self.edit_cursor_pos += 1;
            }
            KeyCode::Backspace => {
                if self.edit_cursor_pos > 0 {
                    self.edit_content.remove(self.edit_cursor_pos - 1);
                    self.edit_cursor_pos -= 1;
                }
            }
            KeyCode::Delete => {
                if self.edit_cursor_pos < self.edit_content.len() {
                    self.edit_content.remove(self.edit_cursor_pos);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame) {
        // Note: Cannot call async refresh_context here since render is not async
        // The refresh will be handled in key event processing

        let size = frame.area();

        match self.view_mode {
            ViewMode::Overview => self.render_overview(frame, size),
            ViewMode::CoreBlocks => self.render_core_blocks_detail(frame, size),
            ViewMode::DynamicBlocks => self.render_dynamic_blocks_detail(frame, size),
            ViewMode::TokenAnalysis => self.render_token_analysis(frame, size),
        }

        // Show help if requested
        if self.show_help {
            self.render_help(frame);
        }

        // Show edit help if requested
        if self.show_edit_help {
            self.render_edit_help(frame);
        }
    }

    fn render_overview(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Main content
                Constraint::Length(3), // Footer
            ])
            .split(area);

        // Render header
        self.render_header(frame, main_chunks[0]);

        // Split main area into panels
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Core blocks
                Constraint::Percentage(25), // Dynamic blocks
                Constraint::Percentage(30), // Context preview
                Constraint::Percentage(20), // Token usage
            ])
            .split(main_chunks[1]);

        // Render panels
        self.render_core_blocks_panel(frame, content_chunks[0]);
        self.render_dynamic_blocks_panel(frame, content_chunks[1]);
        self.render_context_preview_panel(frame, content_chunks[2]);
        self.render_token_usage_panel(frame, content_chunks[3]);

        // Render footer
        self.render_footer(frame, main_chunks[2]);
    }

    fn render_core_blocks_detail(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Core blocks list
                Constraint::Percentage(60), // Selected block content
            ])
            .split(area);

        self.render_core_blocks_panel(frame, chunks[0]);
        self.render_selected_core_block(frame, chunks[1]);
    }

    fn render_dynamic_blocks_detail(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Dynamic blocks list
                Constraint::Percentage(60), // Selected block content
            ])
            .split(area);

        self.render_dynamic_blocks_panel(frame, chunks[0]);
        self.render_selected_dynamic_block(frame, chunks[1]);
    }

    fn render_token_analysis(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Token breakdown
                Constraint::Percentage(60), // Detailed analysis
            ])
            .split(area);

        self.render_token_breakdown(frame, chunks[0]);
        self.render_token_analysis_detail(frame, chunks[1]);
    }

    fn render_header(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let (total_tokens, max_tokens, utilization, active_blocks, dynamic_count, agent_status) =
            if let Some(stats) = &self.cached_stats {
                (
                    stats.total_tokens,
                    stats.max_tokens,
                    stats.utilization,
                    stats.core_block_stats.active_blocks,
                    stats.dynamic_blocks_count,
                    "Agent Loaded",
                )
            } else if self.agent.is_some() {
                // Agent is loaded but context not yet generated
                (0, 8000, 0.0, 0, 0, "Agent Ready (F5 to refresh)")
            } else {
                (0, 8000, 0.0, 0, 0, "No Agent Selected")
            };

        let content = vec![Line::from(vec![
            Span::styled(
                "Context Window Viewer",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("Status: {}", agent_status),
                Style::default().fg(if self.agent.is_some() { Color::Green } else { Color::Red }),
            ),
            Span::styled(" | ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("Tokens: {}/{}", total_tokens, max_tokens),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(" | ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("Utilization: {:.1}%", utilization),
                Style::default().fg(if utilization > 90.0 {
                    Color::Red
                } else if utilization > 70.0 {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ),
            Span::styled(" | ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("Core: {} | Dynamic: {}", active_blocks, dynamic_count),
                Style::default().fg(Color::White),
            ),
        ])];

        let paragraph = Paragraph::new(Text::from(content))
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_core_blocks_panel(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::CoreBlocks;

        let items: Vec<ListItem> = CoreBlockType::all_types()
            .into_iter()
            .enumerate()
            .map(|(i, block_type)| {
                let (is_active, has_content) = if let Some(manager) = &mut self.core_block_manager {
                    if let Some(block) = manager.get_block(block_type) {
                        (
                            block.is_active,
                            block
                                .get_text_content()
                                .map(|c| !c.trim().is_empty())
                                .unwrap_or(false),
                        )
                    } else {
                        (false, false)
                    }
                } else {
                    // Fallback when no manager available
                    (i < 3, true) // Assume first 3 are active for display
                };

                let status = if is_active { "●" } else { "○" };
                let color = if is_active { Color::Green } else { Color::Gray };

                // Show edit indicator
                let edit_indicator =
                    if let EditMode::EditingCoreBlock(editing_type) = self.edit_mode {
                        if editing_type == block_type {
                            " [EDITING]"
                        } else if !has_content {
                            " [EMPTY]"
                        } else {
                            ""
                        }
                    } else if !has_content {
                        " [EMPTY]"
                    } else {
                        ""
                    };

                let content = Line::from(vec![
                    Span::styled(status, Style::default().fg(color)),
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        format!("{:?}{}", block_type, edit_indicator),
                        Style::default().fg(
                            if let EditMode::EditingCoreBlock(editing_type) = self.edit_mode {
                                if editing_type == block_type {
                                    Color::Yellow
                                } else {
                                    Color::White
                                }
                            } else {
                                Color::White
                            },
                        ),
                    ),
                ]);

                ListItem::new(content)
            })
            .collect();

        let style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        let title = if let EditMode::EditingCoreBlock(_) = self.edit_mode {
            "Core Blocks [EDITING]"
        } else {
            "Core Blocks (Enter=Edit)"
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(style),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.core_blocks_state);
    }

    fn render_dynamic_blocks_panel(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::DynamicBlocks;

        // For now, show placeholder dynamic blocks
        let items: Vec<ListItem> = (0..5)
            .map(|i| {
                let relevance = 0.9 - (i as f32 * 0.1);
                let content = Line::from(vec![
                    Span::styled(
                        format!("{:.2}", relevance),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(" | ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("Memory Block {}", i + 1),
                        Style::default().fg(Color::White),
                    ),
                ]);
                ListItem::new(content)
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
                    .title("Dynamic Blocks")
                    .border_style(style),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.dynamic_blocks_state);
    }

    fn render_context_preview_panel(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::ContextPreview;

        let preview = if self.cached_context.len() > 500 {
            format!(
                "{}...\n\n[Truncated - {} total characters]",
                &self.cached_context[..500],
                self.cached_context.len()
            )
        } else if self.agent.is_some() && self.cached_context.contains("No agent loaded") {
            // Agent is loaded but context not refreshed yet
            format!(
                "# Context Preview\n\nAgent is loaded but context hasn't been refreshed yet.\n\nPress F5 to refresh and load current context window.\n\nCurrent conversation has {} messages.",
                self.conversation_history.len()
            )
        } else {
            self.cached_context.clone()
        };

        let style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        let paragraph = Paragraph::new(preview)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Context Preview")
                    .border_style(style),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_token_usage_panel(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::TokenUsage;

        let content = if let Some(stats) = &self.cached_stats {
            vec![
                Line::from(vec![Span::styled(
                    "Total Usage:",
                    Style::default().fg(Color::Cyan),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Core: ", Style::default().fg(Color::Blue)),
                    Span::styled(
                        format!("{}", stats.token_breakdown.core_blocks),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Conv: ", Style::default().fg(Color::Green)),
                    Span::styled(
                        format!("{}", stats.token_breakdown.conversation),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Mem: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{}", stats.token_breakdown.dynamic_memory),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("Total: {}", stats.token_breakdown.total),
                    Style::default().fg(Color::Cyan),
                )]),
            ]
        } else {
            vec![
                Line::from(vec![Span::styled(
                    "Token Usage",
                    Style::default().fg(Color::Cyan),
                )]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "No context manager loaded",
                    Style::default().fg(Color::Gray),
                )]),
                Line::from(vec![Span::styled(
                    "Select an agent to view token usage",
                    Style::default().fg(Color::Gray),
                )]),
            ]
        };

        let style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        let paragraph = Paragraph::new(Text::from(content))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Token Usage")
                    .border_style(style),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_selected_core_block(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if let EditMode::EditingCoreBlock(editing_type) = self.edit_mode {
            // Render editor
            self.render_core_block_editor(frame, area, editing_type);
        } else if let Some(selected) = self.core_blocks_state.selected() {
            let core_types = CoreBlockType::all_types();
            if let Some(block_type) = core_types.get(selected) {
                // Show current content or template
                let content = if let Some(manager) = &mut self.core_block_manager {
                    if let Some(block) = manager.get_block(*block_type) {
                        block
                            .get_text_content()
                            .unwrap_or("[No content]")
                            .to_string()
                    } else {
                        format!(
                            "[Not created yet]\n\nDefault template:\n{}",
                            block_type.default_template()
                        )
                    }
                } else {
                    block_type.default_template().to_string()
                };

                let paragraph = Paragraph::new(content)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("{:?} Content (Enter to edit)", block_type)),
                    )
                    .wrap(Wrap { trim: true });

                frame.render_widget(paragraph, area);
                return;
            }
        }

        let paragraph = Paragraph::new("No core block selected").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Core Block Content"),
        );
        frame.render_widget(paragraph, area);
    }

    fn render_core_block_editor(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        block_type: CoreBlockType,
    ) {
        // Split area for content and help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Editor content
                Constraint::Length(3), // Help line
            ])
            .split(area);

        // Render editor content with cursor
        let editor_lines = self.render_editor_content_with_cursor();
        let paragraph = Paragraph::new(Text::from(editor_lines))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Editing {:?}", block_type))
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, chunks[0]);

        // Render help
        let help_content = "Ctrl+S: Save | Esc: Cancel | F2: Edit Help";
        let help_paragraph = Paragraph::new(help_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Edit Controls"),
            )
            .style(Style::default().fg(Color::Gray));

        frame.render_widget(help_paragraph, chunks[1]);
    }

    fn render_editor_content_with_cursor(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Split content into lines
        let text_lines: Vec<&str> = self.edit_content.lines().collect();

        // Calculate cursor line and column
        let before_cursor = &self.edit_content[..self.edit_cursor_pos];
        let cursor_line = before_cursor.matches('\n').count();
        let line_start = before_cursor.rfind('\n').map(|pos| pos + 1).unwrap_or(0);
        let cursor_col = self.edit_cursor_pos - line_start;

        for (line_idx, line_text) in text_lines.iter().enumerate() {
            if line_idx == cursor_line {
                // This is the line with the cursor
                let mut spans = Vec::new();

                if cursor_col == 0 {
                    // Cursor at beginning of line
                    spans.push(Span::styled(
                        "█",
                        Style::default().bg(Color::White).fg(Color::Black),
                    ));
                    spans.push(Span::styled(
                        line_text.to_string(),
                        Style::default().fg(Color::White),
                    ));
                } else if cursor_col >= line_text.len() {
                    // Cursor at end of line
                    spans.push(Span::styled(
                        line_text.to_string(),
                        Style::default().fg(Color::White),
                    ));
                    spans.push(Span::styled(
                        "█",
                        Style::default().bg(Color::White).fg(Color::Black),
                    ));
                } else {
                    // Cursor in middle of line
                    let before = &line_text[..cursor_col];
                    let at_cursor = line_text.chars().nth(cursor_col).unwrap_or(' ');
                    let after = &line_text[cursor_col + at_cursor.len_utf8()..];

                    spans.push(Span::styled(
                        before.to_string(),
                        Style::default().fg(Color::White),
                    ));
                    spans.push(Span::styled(
                        at_cursor.to_string(),
                        Style::default().bg(Color::White).fg(Color::Black),
                    ));
                    spans.push(Span::styled(
                        after.to_string(),
                        Style::default().fg(Color::White),
                    ));
                }

                lines.push(Line::from(spans));
            } else {
                // Regular line without cursor
                lines.push(Line::from(Span::styled(
                    line_text.to_string(),
                    Style::default().fg(Color::White),
                )));
            }
        }

        // Handle case where content is empty or cursor is at the very end after last newline
        if text_lines.is_empty() || (cursor_line >= text_lines.len()) {
            lines.push(Line::from(Span::styled(
                "█",
                Style::default().bg(Color::White).fg(Color::Black),
            )));
        }

        lines
    }

    fn render_selected_dynamic_block(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let content = if let Some(selected) = self.dynamic_blocks_state.selected() {
            format!(
                "Dynamic Block {} Details\n\nThis would show the actual memory block content, metadata, and relevance information.",
                selected + 1
            )
        } else {
            "No dynamic block selected".to_string()
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Dynamic Block Content"),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_token_breakdown(&mut self, frame: &mut Frame<'_>, area: Rect) {
        if let Some(stats) = &self.cached_stats {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .split(area);

            // Core blocks gauge
            let core_ratio = stats.token_breakdown.core_blocks as f64 / stats.max_tokens as f64;
            let core_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Core Blocks"))
                .gauge_style(Style::default().fg(Color::Blue))
                .ratio(core_ratio)
                .label(format!(
                    "{}/{}",
                    stats.token_breakdown.core_blocks, stats.max_tokens
                ));
            frame.render_widget(core_gauge, chunks[0]);

            // Conversation gauge
            let conv_ratio = stats.token_breakdown.conversation as f64 / stats.max_tokens as f64;
            let conv_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Conversation"))
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(conv_ratio)
                .label(format!(
                    "{}/{}",
                    stats.token_breakdown.conversation, stats.max_tokens
                ));
            frame.render_widget(conv_gauge, chunks[1]);

            // Dynamic memory gauge
            let mem_ratio = stats.token_breakdown.dynamic_memory as f64 / stats.max_tokens as f64;
            let mem_gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Dynamic Memory"),
                )
                .gauge_style(Style::default().fg(Color::Yellow))
                .ratio(mem_ratio)
                .label(format!(
                    "{}/{}",
                    stats.token_breakdown.dynamic_memory, stats.max_tokens
                ));
            frame.render_widget(mem_gauge, chunks[2]);

            // Total gauge
            let total_ratio = stats.total_tokens as f64 / stats.max_tokens as f64;
            let total_color = if total_ratio > 0.9 {
                Color::Red
            } else if total_ratio > 0.7 {
                Color::Yellow
            } else {
                Color::Green
            };
            let total_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Total Usage"))
                .gauge_style(Style::default().fg(total_color))
                .ratio(total_ratio)
                .label(format!(
                    "{}/{} ({:.1}%)",
                    stats.total_tokens,
                    stats.max_tokens,
                    total_ratio * 100.0
                ));
            frame.render_widget(total_gauge, chunks[3]);
        } else {
            // Show placeholder when no stats available
            let placeholder = Paragraph::new("No context data available\n\nSelect an agent from the conversation to view token breakdown")
                .block(Block::default().borders(Borders::ALL).title("Token Breakdown"))
                .style(Style::default().fg(Color::Gray))
                .wrap(Wrap { trim: true });
            frame.render_widget(placeholder, area);
        }
    }

    fn render_token_analysis_detail(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let content = if let Some(stats) = &self.cached_stats {
            format!(
                "Token Analysis\n\n\
                Context Window Configuration:\n\
                • Max Total Tokens: {}\n\
                • Core Block Budget: {} tokens\n\
                • Conversation Budget: {} tokens\n\
                • Dynamic Memory Budget: {} tokens\n\n\
                Current Usage:\n\
                • Core Blocks: {} tokens ({:.1}%)\n\
                • Conversation: {} tokens ({:.1}%)\n\
                • Dynamic Memory: {} tokens ({:.1}%)\n\
                • Total: {} tokens ({:.1}%)\n\n\
                Memory Block Statistics:\n\
                • Active Core Blocks: {}\n\
                • Dynamic Blocks: {}\n\
                • Efficiency: {:.1}% utilization\n\n\
                Recommendations:\n\
                {}",
                stats.max_tokens,
                3000, // From config - would get from actual config
                3000,
                2000,
                stats.token_breakdown.core_blocks,
                (stats.token_breakdown.core_blocks as f32 / stats.max_tokens as f32) * 100.0,
                stats.token_breakdown.conversation,
                (stats.token_breakdown.conversation as f32 / stats.max_tokens as f32) * 100.0,
                stats.token_breakdown.dynamic_memory,
                (stats.token_breakdown.dynamic_memory as f32 / stats.max_tokens as f32) * 100.0,
                stats.total_tokens,
                stats.utilization,
                stats.core_block_stats.active_blocks,
                stats.dynamic_blocks_count,
                stats.utilization,
                if stats.utilization > 90.0 {
                    "• Consider reducing conversation history\n• Deactivate non-essential core blocks\n• Increase relevance threshold for dynamic blocks"
                } else if stats.utilization < 50.0 {
                    "• Context window has room for more memory blocks\n• Consider lowering relevance threshold\n• Add more conversation history"
                } else {
                    "• Context window usage is optimal\n• Good balance between different block types"
                }
            )
        } else {
            "Token Analysis\n\n\
            No context data available.\n\n\
            To view detailed token analysis:\n\
            1. Select an agent from the conversation\n\
            2. Send at least one message\n\
            3. Return to this context viewer\n\n\
            The context window manager will then show:\n\
            • Real token usage breakdown\n\
            • Active memory blocks\n\
            • Optimization recommendations\n\
            • Dynamic memory selection details"
                .to_string()
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Detailed Analysis"),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let content = vec![Line::from(vec![
            Span::styled("Views: ", Style::default().fg(Color::Gray)),
            Span::styled("1", Style::default().fg(Color::Yellow)),
            Span::styled("-Overview ", Style::default().fg(Color::Gray)),
            Span::styled("2", Style::default().fg(Color::Yellow)),
            Span::styled("-Core ", Style::default().fg(Color::Gray)),
            Span::styled("3", Style::default().fg(Color::Yellow)),
            Span::styled("-Dynamic ", Style::default().fg(Color::Gray)),
            Span::styled("4", Style::default().fg(Color::Yellow)),
            Span::styled("-Tokens | ", Style::default().fg(Color::Gray)),
            Span::styled("S", Style::default().fg(Color::Yellow)),
            Span::styled("-Strategy | ", Style::default().fg(Color::Gray)),
            Span::styled("F5", Style::default().fg(Color::Yellow)),
            Span::styled("-Refresh | ", Style::default().fg(Color::Gray)),
            Span::styled("F1", Style::default().fg(Color::Yellow)),
            Span::styled("-Help", Style::default().fg(Color::Gray)),
        ])];

        let paragraph = Paragraph::new(Text::from(content))
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_help(&self, frame: &mut Frame) {
        show_popup(
            frame,
            "Help - Context Window Viewer",
            "Context Window Viewer Help\n\
             \n\
             Views:\n\
             1 - Overview (all panels)\n\
             2 - Core Blocks detail\n\
             3 - Dynamic Blocks detail\n\
             4 - Token Analysis detail\n\
             \n\
             Navigation:\n\
             Tab       - Switch between panels\n\
             ↑/k, ↓/j  - Navigate lists\n\
             Enter     - Edit selected core block\n\
             \n\
             Editing Core Blocks:\n\
             Enter     - Start editing selected block\n\
             Ctrl+S    - Save changes\n\
             Esc       - Cancel editing\n\
             F2        - Show detailed edit help\n\
             \n\
             Actions:\n\
             S         - Cycle selection strategy\n\
             F5        - Refresh context window\n\
             F1        - Toggle this help\n\
             F2        - Toggle edit help\n\
             \n\
             Core Block Types:\n\
             • SystemPrompt - AI instructions & behavior\n\
             • UserPersona - User information & preferences\n\
             • TaskContext - Current project context\n\
             • KeyFacts - Important facts to remember\n\
             • UserPreferences - Settings & preferences\n\
             • ConversationSummary - Session summary\n\
             • ActiveGoals - Current objectives\n\
             • WorkingMemory - Temporary notes\n\
             \n\
             Status Indicators:\n\
             ● Active    ○ Inactive    [EDITING] Currently editing\n\
             [EMPTY] No content    • • • Has content\n\
             \n\
             Color Coding:\n\
             • Green - Active/good status\n\
             • Yellow - Warning/editing\n\
             • Red - Critical/high usage\n\
             • Blue - Core block related\n\
             • Cyan - Selected/focused",
            (85, 75),
        );
    }

    fn render_edit_help(&self, frame: &mut Frame) {
        show_popup(
            frame,
            "Edit Help - Core Block Editor",
            "Core Block Editor Help\n\
             \n\
             Core blocks are persistent context that the AI\n\
             always sees. Edit them to customize AI behavior.\n\
             \n\
             Essential Blocks (always active):\n\
             • SystemPrompt - Controls AI personality & rules\n\
             • UserPersona - Information about you\n\
             • WorkingMemory - Current session context\n\
             \n\
             Optional Blocks (can be activated/deactivated):\n\
             • TaskContext - Current project details\n\
             • KeyFacts - Important information to remember\n\
             • UserPreferences - Your settings & preferences\n\
             • ConversationSummary - Auto-generated summaries\n\
             • ActiveGoals - Your current objectives\n\
             \n\
             Editor Controls:\n\
             ← → ↑ ↓      - Move cursor\n\
             Home/End     - Move to line start/end\n\
             Ctrl+S       - Save changes (persistent!)\n\
             Esc          - Cancel editing\n\
             Backspace    - Delete char before cursor\n\
             Delete       - Delete char after cursor\n\
             Enter        - Insert newline\n\
             \n\
             Examples:\n\
             SystemPrompt: \"You are a helpful coding assistant\n\
             with expertise in Rust. Always provide examples.\"\n\
             \n\
             UserPersona: \"User is a senior developer working\n\
             on a terminal UI project using ratatui.\"\n\
             \n\
             Changes persist across conversations!\n\
             The AI will always see your custom blocks.",
            (75, 70),
        );
    }
}
