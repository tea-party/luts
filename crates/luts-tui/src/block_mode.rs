//! Memory blocks TUI component for Letta-style memory management

use crate::{components::show_popup, events::AppEvent, markdown::SimpleMarkdownRenderer};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use luts_core::memory::{
    BlockId, BlockType, MemoryBlock, MemoryBlockBuilder, MemoryContent, MemoryManager,
    SurrealConfig, SurrealMemoryStore,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusedPanel {
    List,
    Details,
    Editor,
}

pub struct BlockMode {
    _memory_manager: Arc<MemoryManager>,
    memory_blocks: Vec<MemoryBlock>,
    focused_panel: FocusedPanel,
    block_list_state: ListState,
    scroll_state: ScrollbarState,
    _event_sender: mpsc::UnboundedSender<AppEvent>,
    show_help: bool,
    editing_block: Option<BlockId>,
    editor_content: String,
    editor_cursor_pos: usize, // Cursor position in the editor content
    markdown_renderer: SimpleMarkdownRenderer,
    show_create_dialog: bool,
    create_dialog_input: String,
    create_dialog_type: BlockType,
    block_list_area: Option<Rect>,
    user_id: String,
    session_id: String,
}

impl BlockMode {
    pub fn new(event_sender: mpsc::UnboundedSender<AppEvent>) -> Self {
        // Initialize memory manager with SurrealDB store
        let data_dir = PathBuf::from("./data");
        let surreal_config = SurrealConfig::File {
            path: data_dir.join("memory.db"),
            namespace: "luts".to_string(),
            database: "memory".to_string(),
        };
        let surreal_store = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                SurrealMemoryStore::new(surreal_config)
                    .await
                    .expect("Failed to create SurrealDB store")
            })
        });
        let memory_manager = Arc::new(MemoryManager::new(surreal_store));

        let user_id = "default_user".to_string();
        let session_id = "memory_blocks_session".to_string();

        // Create some sample memory blocks to start with
        let memory_blocks = vec![];

        let mut block_list_state = ListState::default();
        if !memory_blocks.is_empty() {
            block_list_state.select(Some(0));
        }

        Self {
            _memory_manager: memory_manager,
            memory_blocks,
            focused_panel: FocusedPanel::List,
            block_list_state,
            scroll_state: ScrollbarState::default(),
            _event_sender: event_sender,
            show_help: false,
            editing_block: None,
            editor_content: String::new(),
            editor_cursor_pos: 0,
            markdown_renderer: SimpleMarkdownRenderer::default(),
            show_create_dialog: false,
            create_dialog_input: String::new(),
            create_dialog_type: BlockType::Message,
            block_list_area: None,
            user_id,
            session_id,
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                // Check if click is within the block list area
                if let Some(area) = self.block_list_area {
                    if mouse.column >= area.x
                        && mouse.column < area.x + area.width
                        && mouse.row >= area.y
                        && mouse.row < area.y + area.height
                    {
                        // Focus the block list and calculate which block was clicked
                        self.focused_panel = FocusedPanel::List;

                        // Calculate which block was clicked (account for borders)
                        let relative_row = mouse.row.saturating_sub(area.y + 1); // +1 for top border
                        let clicked_index = relative_row.saturating_sub(1) as usize; // -1 for title

                        if clicked_index < self.memory_blocks.len() {
                            self.block_list_state.select(Some(clicked_index));
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.focused_panel == FocusedPanel::List {
                    let selected = self.block_list_state.selected().unwrap_or(0);
                    if selected > 0 {
                        self.block_list_state.select(Some(selected - 1));
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focused_panel == FocusedPanel::List {
                    let selected = self.block_list_state.selected().unwrap_or(0);
                    let max_blocks = self.memory_blocks.len().saturating_sub(1);
                    if selected < max_blocks {
                        self.block_list_state.select(Some(selected + 1));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        if self.show_create_dialog {
            return self.handle_create_dialog_key(key);
        }

        match key.code {
            KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            KeyCode::Tab => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::List => FocusedPanel::Details,
                    FocusedPanel::Details => FocusedPanel::Editor,
                    FocusedPanel::Editor => FocusedPanel::List,
                };
            }
            KeyCode::Char('n')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.show_create_dialog = true;
                self.create_dialog_input.clear();
                self.create_dialog_type = BlockType::Message;
            }
            KeyCode::Char('r')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.refresh_memory_blocks();
                info!("Memory blocks refreshed from storage");
            }
            KeyCode::Char('s')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.save_memory_blocks();
                info!("Memory blocks saved to storage");
            }
            KeyCode::Enter => {
                if self.focused_panel == FocusedPanel::List {
                    if let Some(selected) = self.block_list_state.selected() {
                        if let Some(block) = self.memory_blocks.get(selected) {
                            self.editing_block = Some(block.id().clone());
                            self.editor_content =
                                block.content().as_text().unwrap_or("").to_string();
                            self.editor_cursor_pos = self.editor_content.len(); // Start cursor at end
                            self.focused_panel = FocusedPanel::Editor;
                            info!("Started editing memory block: {}", block.id());
                        }
                    }
                }
            }
            KeyCode::Delete => {
                if self.focused_panel == FocusedPanel::List {
                    if let Some(selected) = self.block_list_state.selected() {
                        if selected < self.memory_blocks.len() {
                            let removed_block = self.memory_blocks.remove(selected);
                            info!("Deleted memory block: {}", removed_block.id());

                            // Adjust selection if needed
                            if self.memory_blocks.is_empty() {
                                self.block_list_state.select(None);
                            } else if selected >= self.memory_blocks.len() {
                                self.block_list_state
                                    .select(Some(self.memory_blocks.len() - 1));
                            }
                        }
                    }
                }
            }
            _ => match self.focused_panel {
                FocusedPanel::List => self.handle_block_list_key(key)?,
                FocusedPanel::Details => self.handle_block_details_key(key)?,
                FocusedPanel::Editor => self.handle_block_editor_key(key)?,
            },
        }
        Ok(())
    }

    fn handle_create_dialog_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.show_create_dialog = false;
                self.create_dialog_input.clear();
            }
            KeyCode::F(2) => {
                // Cycle through block types
                self.create_dialog_type = match self.create_dialog_type {
                    BlockType::Message => BlockType::Summary,
                    BlockType::Summary => BlockType::Fact,
                    BlockType::Fact => BlockType::Preference,
                    BlockType::Preference => BlockType::PersonalInfo,
                    BlockType::PersonalInfo => BlockType::Goal,
                    BlockType::Goal => BlockType::Task,
                    BlockType::Task => BlockType::Message,
                    BlockType::Custom(_) => BlockType::Message,
                };
            }
            KeyCode::Enter => {
                if !self.create_dialog_input.trim().is_empty() {
                    let new_block = MemoryBlockBuilder::new()
                        .with_type(self.create_dialog_type)
                        .with_user_id(&self.user_id)
                        .with_session_id(&self.session_id)
                        .with_content(MemoryContent::Text(self.create_dialog_input.clone()))
                        .with_tag("user_created")
                        .build()
                        .unwrap();

                    self.memory_blocks.push(new_block);
                    info!(
                        "Created new {} block with content: {}",
                        self.create_dialog_type, self.create_dialog_input
                    );
                }
                self.show_create_dialog = false;
                self.create_dialog_input.clear();
            }
            KeyCode::Char(c) => {
                self.create_dialog_input.push(c);
            }
            KeyCode::Backspace => {
                self.create_dialog_input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_block_list_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = self.block_list_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.block_list_state.select(Some(selected - 1));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let selected = self.block_list_state.selected().unwrap_or(0);
                let max_blocks = self.memory_blocks.len().saturating_sub(1);
                if selected < max_blocks {
                    self.block_list_state.select(Some(selected + 1));
                }
            }
            KeyCode::Home => {
                if !self.memory_blocks.is_empty() {
                    self.block_list_state.select(Some(0));
                }
            }
            KeyCode::End => {
                if !self.memory_blocks.is_empty() {
                    self.block_list_state
                        .select(Some(self.memory_blocks.len() - 1));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_block_details_key(&mut self, _key: KeyEvent) -> Result<()> {
        // Block details panel is read-only for now
        Ok(())
    }

    fn refresh_memory_blocks(&mut self) {
        // In a real implementation, this would load blocks from storage
        // For now, we'll keep the current blocks as-is
        info!("Memory blocks refresh requested (not yet implemented)");
    }

    fn save_memory_blocks(&mut self) {
        // In a real implementation, this would save blocks to storage
        // For now, we'll just log that save was requested
        info!("Memory blocks save requested (not yet implemented)");
    }

    fn handle_block_editor_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                // Cancel editing
                self.editing_block = None;
                self.editor_content.clear();
                self.editor_cursor_pos = 0;
                self.focused_panel = FocusedPanel::List;
            }
            KeyCode::Char('s')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                // Save changes
                if let Some(block_id) = &self.editing_block {
                    if let Some(block) = self.memory_blocks.iter_mut().find(|b| b.id() == block_id)
                    {
                        block.set_content(MemoryContent::Text(self.editor_content.clone()));
                        info!("Saved changes to memory block: {}", block_id);
                    }
                }
                self.editing_block = None;
                self.editor_content.clear();
                self.editor_cursor_pos = 0;
                self.focused_panel = FocusedPanel::List;
            }
            KeyCode::Left => {
                if self.editor_cursor_pos > 0 {
                    // Handle multi-byte UTF-8 characters properly
                    let mut pos = self.editor_cursor_pos;
                    while pos > 0 && !self.editor_content.is_char_boundary(pos - 1) {
                        pos -= 1;
                    }
                    pos = pos.saturating_sub(1);
                    self.editor_cursor_pos = pos;
                }
            }
            KeyCode::Right => {
                if self.editor_cursor_pos < self.editor_content.len() {
                    // Handle multi-byte UTF-8 characters properly
                    let mut pos = self.editor_cursor_pos;
                    while pos < self.editor_content.len()
                        && !self.editor_content.is_char_boundary(pos + 1)
                    {
                        pos += 1;
                    }
                    if pos < self.editor_content.len() {
                        pos += 1;
                    }
                    self.editor_cursor_pos = pos;
                }
            }
            KeyCode::Up => {
                // Move cursor to previous line
                self.move_cursor_up();
            }
            KeyCode::Down => {
                // Move cursor to next line
                self.move_cursor_down();
            }
            KeyCode::Home => {
                // Move to beginning of current line
                self.move_cursor_to_line_start();
            }
            KeyCode::End => {
                // Move to end of current line
                self.move_cursor_to_line_end();
            }
            KeyCode::Char(c) => {
                // Insert character at cursor position
                self.editor_content.insert(self.editor_cursor_pos, c);
                self.editor_cursor_pos += c.len_utf8();
            }
            KeyCode::Backspace => {
                if self.editor_cursor_pos > 0 {
                    // Handle multi-byte UTF-8 characters properly
                    let mut pos = self.editor_cursor_pos;
                    while pos > 0 && !self.editor_content.is_char_boundary(pos - 1) {
                        pos -= 1;
                    }
                    if pos > 0 {
                        pos -= 1;
                        self.editor_content.remove(pos);
                        self.editor_cursor_pos = pos;
                    }
                }
            }
            KeyCode::Delete => {
                if self.editor_cursor_pos < self.editor_content.len() {
                    // Handle multi-byte UTF-8 characters properly
                    let mut pos = self.editor_cursor_pos;
                    while pos < self.editor_content.len()
                        && !self.editor_content.is_char_boundary(pos + 1)
                    {
                        pos += 1;
                    }
                    if pos < self.editor_content.len() {
                        self.editor_content.remove(self.editor_cursor_pos);
                    }
                }
            }
            KeyCode::Enter => {
                // Insert newline at cursor position
                self.editor_content.insert(self.editor_cursor_pos, '\n');
                self.editor_cursor_pos += 1;
            }
            _ => {}
        }
        Ok(())
    }

    fn move_cursor_up(&mut self) {
        let (line_start, _) = self.get_current_line_bounds();
        if line_start > 0 {
            // Find the previous line
            let prev_line_end = line_start - 1; // Before the '\n'
            let prev_line_start = self.editor_content[..prev_line_end]
                .rfind('\n')
                .map(|pos| pos + 1)
                .unwrap_or(0);

            let current_col = self.editor_cursor_pos - line_start;
            let prev_line_len = prev_line_end - prev_line_start;

            // Try to maintain column position
            let new_col = current_col.min(prev_line_len);
            self.editor_cursor_pos = prev_line_start + new_col;
        }
    }

    fn move_cursor_down(&mut self) {
        let (line_start, line_end) = self.get_current_line_bounds();
        if line_end < self.editor_content.len() {
            // Find the next line
            let next_line_start = line_end + 1; // After the '\n'
            let next_line_end = self.editor_content[next_line_start..]
                .find('\n')
                .map(|pos| next_line_start + pos)
                .unwrap_or(self.editor_content.len());

            let current_col = self.editor_cursor_pos - line_start;
            let next_line_len = next_line_end - next_line_start;

            // Try to maintain column position
            let new_col = current_col.min(next_line_len);
            self.editor_cursor_pos = next_line_start + new_col;
        }
    }

    fn move_cursor_to_line_start(&mut self) {
        let (line_start, _) = self.get_current_line_bounds();
        self.editor_cursor_pos = line_start;
    }

    fn move_cursor_to_line_end(&mut self) {
        let (_, line_end) = self.get_current_line_bounds();
        self.editor_cursor_pos = line_end;
    }

    fn get_current_line_bounds(&self) -> (usize, usize) {
        let before_cursor = &self.editor_content[..self.editor_cursor_pos];
        let line_start = before_cursor.rfind('\n').map(|pos| pos + 1).unwrap_or(0);

        let after_start = &self.editor_content[line_start..];
        let line_end = after_start
            .find('\n')
            .map(|pos| line_start + pos)
            .unwrap_or(self.editor_content.len());

        (line_start, line_end)
    }

    fn render_editor_content_with_cursor(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Split content into lines
        let text_lines: Vec<&str> = self.editor_content.lines().collect();

        // Calculate cursor line and column
        let before_cursor = &self.editor_content[..self.editor_cursor_pos];
        let cursor_line = before_cursor.matches('\n').count();
        let line_start = before_cursor.rfind('\n').map(|pos| pos + 1).unwrap_or(0);
        let cursor_col = self.editor_cursor_pos - line_start;

        for (line_idx, line_text) in text_lines.iter().enumerate() {
            if line_idx == cursor_line {
                // This is the line with the cursor
                let mut spans = Vec::new();

                if cursor_col == 0 {
                    // Cursor at beginning of line
                    spans.push(Span::styled(
                        "█",
                        Style::default().bg(Color::White).fg(Color::Black),
                    )); // Block cursor
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
                    )); // Block cursor
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
                    )); // Block cursor
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

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Block list
                Constraint::Percentage(60), // Details and editor
            ])
            .split(size);

        // Split right side into details and editor
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // Block details
                Constraint::Percentage(30), // Block editor/workflow status
            ])
            .split(main_chunks[1]);

        // Render block list
        self.render_block_list(frame, main_chunks[0]);

        // Render block details
        self.render_block_details(frame, right_chunks[0]);

        // Render workflow status
        self.render_workflow_status(frame, right_chunks[1]);

        // Show dialogs
        if self.show_create_dialog {
            self.render_create_dialog(frame);
        }

        if self.show_help {
            show_popup(
                frame,
                "Help - Memory Blocks Mode",
                "Navigation:\n\
                 Tab        - Switch focus between panels\n\
                 ↑/k        - Move up in block list\n\
                 ↓/j        - Move down in block list\n\
                 Click      - Focus and select block\n\
                 Enter      - Edit selected block content\n\
                 Delete     - Delete selected block\n\
                 Ctrl+N     - Create new memory block\n\
                 Ctrl+S     - Save all blocks to storage\n\
                 Ctrl+R     - Refresh blocks from storage\n\
                 F2         - Change block type (in create dialog)\n\
                 \n\
                 Memory Block Types:\n\
                 MSG - Message blocks for conversations\n\
                 SUM - Summary blocks for condensed info\n\
                 FCT - Fact blocks for persistent knowledge\n\
                 PRF - Preference blocks for user settings\n\
                 INF - Personal info blocks\n\
                 GOL - Goal blocks for objectives\n\
                 TSK - Task blocks for actions\n\
                 \n\
                 Editor Controls (when editing):\n\
                 ← → ↑ ↓      - Move cursor\n\
                 Home/End     - Move to line start/end\n\
                 Ctrl+S       - Save changes\n\
                 Esc          - Cancel editing\n\
                 Backspace    - Delete char before cursor\n\
                 Delete       - Delete char after cursor\n\
                 Enter        - Insert newline\n\
                 \n\
                 Mode Switching:\n\
                 Ctrl+T       - Tool Activity (monitor AI tool usage)\n\
                 F2           - Configuration\n\
                 Esc          - Back to conversation\n\
                 \n\
                 System:\n\
                 F1           - Toggle this help\n\
                 Ctrl+Q       - Quit application",
                (80, 70),
            );
        }
    }

    fn render_block_list(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::List;

        // Store the block list area for mouse handling
        self.block_list_area = Some(area);

        let items: Vec<ListItem> = self
            .memory_blocks
            .iter()
            .map(|block| {
                let type_str = match block.block_type() {
                    BlockType::Message => "MSG",
                    BlockType::Summary => "SUM",
                    BlockType::Fact => "FCT",
                    BlockType::Preference => "PRF",
                    BlockType::PersonalInfo => "INF",
                    BlockType::Goal => "GOL",
                    BlockType::Task => "TSK",
                    BlockType::Custom(_) => "CST",
                };

                let color = match block.block_type() {
                    BlockType::Message => Color::Blue,
                    BlockType::Summary => Color::Green,
                    BlockType::Fact => Color::Yellow,
                    BlockType::Preference => Color::Magenta,
                    BlockType::PersonalInfo => Color::Cyan,
                    BlockType::Goal => Color::Red,
                    BlockType::Task => Color::Gray,
                    BlockType::Custom(_) => Color::White,
                };

                let content_preview = block
                    .content()
                    .as_text()
                    .map(|text| {
                        if text.len() > 50 {
                            format!("{}...", &text[..47])
                        } else {
                            text.to_string()
                        }
                    })
                    .unwrap_or_else(|| "[Binary content]".to_string());

                let content = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", type_str),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(content_preview, Style::default().fg(Color::White)),
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
                    .title("Memory Blocks")
                    .border_style(style),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.block_list_state);

        // Render scrollbar
        let blocks_len = self.memory_blocks.len();

        self.scroll_state = self.scroll_state.content_length(blocks_len);
        if let Some(selected) = self.block_list_state.selected() {
            self.scroll_state = self.scroll_state.position(selected);
        }

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut self.scroll_state,
        );
    }

    fn render_block_details(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::Details;
        let selected_block = self
            .block_list_state
            .selected()
            .and_then(|i| self.memory_blocks.get(i));

        let content = if let Some(block) = selected_block {
            let tags = if block.tags().is_empty() {
                "None".to_string()
            } else {
                block.tags().join(", ")
            };

            let properties = if block.properties().is_empty() {
                "None".to_string()
            } else {
                block
                    .properties()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let reference_ids = if block.reference_ids().is_empty() {
                "None".to_string()
            } else {
                block
                    .reference_ids()
                    .iter()
                    .map(|id| id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let relevance = block
                .relevance()
                .map(|r| format!("{:.2}", r.score()))
                .unwrap_or_else(|| "Not set".to_string());

            vec![
                Line::from(vec![
                    Span::styled("ID: ", Style::default().fg(Color::Cyan)),
                    Span::styled(block.id().as_str(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{}", block.block_type()),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("User ID: ", Style::default().fg(Color::Cyan)),
                    Span::styled(block.user_id(), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Session ID: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        block.session_id().unwrap_or("None"),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Created: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{}", block.created_at()),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Updated: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{}", block.updated_at()),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Tags: ", Style::default().fg(Color::Cyan)),
                    Span::styled(tags, Style::default().fg(Color::Gray)),
                ]),
                Line::from(vec![
                    Span::styled("Properties: ", Style::default().fg(Color::Cyan)),
                    Span::styled(properties, Style::default().fg(Color::Gray)),
                ]),
                Line::from(vec![
                    Span::styled("References: ", Style::default().fg(Color::Cyan)),
                    Span::styled(reference_ids, Style::default().fg(Color::Gray)),
                ]),
                Line::from(vec![
                    Span::styled("Relevance: ", Style::default().fg(Color::Cyan)),
                    Span::styled(relevance, Style::default().fg(Color::Green)),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Content:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ]
        } else {
            vec![Line::from("No block selected")]
        };

        let mut all_content = content;

        // Add block content with markdown rendering if there's a selected block
        if let Some(block) = selected_block {
            if let Some(text) = block.content().as_text() {
                let rendered_content = self.markdown_renderer.render(text);
                all_content.extend(rendered_content.lines);
            } else {
                all_content.push(Line::from("[Non-text content]"));
            }
        }

        let style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        let paragraph = Paragraph::new(Text::from(all_content))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Block Details")
                    .border_style(style),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_workflow_status(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::Editor;

        let content = if let Some(block_id) = &self.editing_block {
            vec![
                Line::from(vec![
                    Span::styled(
                        "Editing Block: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(block_id.as_str(), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Content (Ctrl+S to save, Esc to cancel):",
                    Style::default().fg(Color::Gray),
                )]),
                Line::from(""),
            ]
        } else {
            let total_blocks = self.memory_blocks.len();
            let message_count = self
                .memory_blocks
                .iter()
                .filter(|b| b.block_type() == BlockType::Message)
                .count();
            let fact_count = self
                .memory_blocks
                .iter()
                .filter(|b| b.block_type() == BlockType::Fact)
                .count();
            let preference_count = self
                .memory_blocks
                .iter()
                .filter(|b| b.block_type() == BlockType::Preference)
                .count();
            let goal_count = self
                .memory_blocks
                .iter()
                .filter(|b| b.block_type() == BlockType::Goal)
                .count();

            vec![
                Line::from(vec![Span::styled(
                    "Memory Block Editor",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("📊 Total Blocks: ", Style::default().fg(Color::Blue)),
                    Span::styled(
                        format!("{}", total_blocks),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("💬 Messages: ", Style::default().fg(Color::Green)),
                    Span::styled(
                        format!("{}", message_count),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("  📝 Facts: ", Style::default().fg(Color::Yellow)),
                    Span::styled(format!("{}", fact_count), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("⚙️ Preferences: ", Style::default().fg(Color::Magenta)),
                    Span::styled(
                        format!("{}", preference_count),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled("  🎯 Goals: ", Style::default().fg(Color::Red)),
                    Span::styled(format!("{}", goal_count), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Commands:",
                    Style::default().fg(Color::Cyan),
                )]),
                Line::from(vec![Span::styled(
                    "Enter=edit, Del=delete, Ctrl+n=create, Ctrl+s=save, Ctrl+r=refresh",
                    Style::default().fg(Color::Gray),
                )]),
            ]
        };

        let mut all_content = content;

        // If editing, show the editor content with cursor
        if self.editing_block.is_some() {
            let editor_lines = self.render_editor_content_with_cursor();
            all_content.extend(editor_lines);
        }

        let style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Gray)
        };

        let title = if self.editing_block.is_some() {
            "Block Editor"
        } else {
            "Memory Control"
        };

        let paragraph = Paragraph::new(Text::from(all_content))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(style),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_create_dialog(&self, frame: &mut Frame) {
        let area = self.centered_rect(60, 25, frame.area());
        frame.render_widget(Clear, area);

        let content = format!(
            "Create New Memory Block\n\nType: {} (F2 to change)\n\nContent:\n{}",
            self.create_dialog_type, self.create_dialog_input
        );

        let block = Block::default()
            .title("Create Memory Block")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}
