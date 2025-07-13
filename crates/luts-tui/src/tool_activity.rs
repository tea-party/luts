//! Tool activity panel for monitoring AI tool usage in real-time

use crate::{components::show_popup, events::AppEvent};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusedPanel {
    ToolList,
    ToolDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEntry {
    pub id: String,
    pub tool_name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub timestamp: u64,
    pub duration_ms: Option<u64>,
    pub status: ToolCallStatus,
    pub agent_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCallStatus {
    Starting,
    InProgress,
    Completed,
    Failed(String),
}

impl ToolCallEntry {
    #[allow(dead_code)]
    pub fn new(tool_name: String, arguments: String, agent_name: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            id: format!("tool_{}_{}", timestamp, rand::random::<u32>()),
            tool_name,
            arguments,
            result: None,
            timestamp,
            duration_ms: None,
            status: ToolCallStatus::Starting,
            agent_name,
        }
    }

    #[allow(dead_code)]
    pub fn set_in_progress(&mut self) {
        self.status = ToolCallStatus::InProgress;
    }

    #[allow(dead_code)]
    pub fn set_completed(&mut self, result: String, duration_ms: u64) {
        self.result = Some(result);
        self.duration_ms = Some(duration_ms);
        self.status = ToolCallStatus::Completed;
    }

    #[allow(dead_code)]
    pub fn set_failed(&mut self, error: String, duration_ms: u64) {
        self.duration_ms = Some(duration_ms);
        self.status = ToolCallStatus::Failed(error);
    }

    pub fn get_status_icon(&self) -> &'static str {
        match self.status {
            ToolCallStatus::Starting => "🟡",
            ToolCallStatus::InProgress => "🔄",
            ToolCallStatus::Completed => "✅",
            ToolCallStatus::Failed(_) => "❌",
        }
    }

    pub fn get_status_color(&self) -> Color {
        match self.status {
            ToolCallStatus::Starting => Color::Yellow,
            ToolCallStatus::InProgress => Color::Blue,
            ToolCallStatus::Completed => Color::Green,
            ToolCallStatus::Failed(_) => Color::Red,
        }
    }

    pub fn format_timestamp(&self) -> String {
        let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(
            (self.timestamp / 1000) as i64,
            ((self.timestamp % 1000) * 1_000_000) as u32,
        )
        .unwrap_or_default();
        datetime.format("%H:%M:%S").to_string()
    }
}

pub struct ToolActivityPanel {
    tool_calls: Vec<ToolCallEntry>,
    focused_panel: FocusedPanel,
    tool_list_state: ListState,
    scroll_state: ScrollbarState,
    _event_sender: mpsc::UnboundedSender<AppEvent>,
    show_help: bool,
    tool_list_area: Option<Rect>,
}

impl ToolActivityPanel {
    pub fn new(event_sender: mpsc::UnboundedSender<AppEvent>) -> Self {
        let mut tool_list_state = ListState::default();

        // Create some sample tool calls for demonstration
        let sample_calls = vec![
            ToolCallEntry {
                id: "demo_1".to_string(),
                tool_name: "block".to_string(),
                arguments: r#"{"user_id": "demo_user", "block_type": "Fact", "content": "Rust is a systems programming language"}"#.to_string(),
                result: Some(r#"{"success": true, "block_id": "block_abc123", "message": "Created Fact block with ID block_abc123"}"#.to_string()),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 - 5000,
                duration_ms: Some(245),
                status: ToolCallStatus::Completed,
                agent_name: "Dr. Research".to_string(),
            },
            ToolCallEntry {
                id: "demo_2".to_string(),
                tool_name: "retrieve_context".to_string(),
                arguments: r#"{"user_id": "demo_user", "block_types": ["Preference"], "limit": 5}"#.to_string(),
                result: Some(r#"{"blocks": [{"id": "block_pref1", "type": "Preference", "content": "Prefers casual communication style"}]}"#.to_string()),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 - 3000,
                duration_ms: Some(156),
                status: ToolCallStatus::Completed,
                agent_name: "Dr. Research".to_string(),
            },
            ToolCallEntry {
                id: "demo_3".to_string(),
                tool_name: "search".to_string(),
                arguments: r#"{"query": "latest Rust programming features 2025"}"#.to_string(),
                result: None,
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 - 1000,
                duration_ms: None,
                status: ToolCallStatus::InProgress,
                agent_name: "Dr. Research".to_string(),
            },
        ];

        if !sample_calls.is_empty() {
            tool_list_state.select(Some(0));
        }

        Self {
            tool_calls: sample_calls,
            focused_panel: FocusedPanel::ToolList,
            tool_list_state,
            scroll_state: ScrollbarState::default(),
            _event_sender: event_sender,
            show_help: false,
            tool_list_area: None,
        }
    }

    #[allow(dead_code)]
    pub fn add_tool_call(&mut self, tool_call: ToolCallEntry) {
        self.tool_calls.push(tool_call);
        // Auto-scroll to bottom for new entries
        if !self.tool_calls.is_empty() {
            self.tool_list_state.select(Some(self.tool_calls.len() - 1));
        }
    }

    #[allow(dead_code)]
    pub fn update_tool_call(
        &mut self,
        call_id: &str,
        result: Option<String>,
        status: ToolCallStatus,
        duration_ms: Option<u64>,
    ) {
        if let Some(call) = self.tool_calls.iter_mut().find(|c| c.id == call_id) {
            call.status = status;
            if let Some(result) = result {
                call.result = Some(result);
            }
            if let Some(duration) = duration_ms {
                call.duration_ms = Some(duration);
            }
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                if let Some(area) = self.tool_list_area {
                    if mouse.column >= area.x
                        && mouse.column < area.x + area.width
                        && mouse.row >= area.y
                        && mouse.row < area.y + area.height
                    {
                        self.focused_panel = FocusedPanel::ToolList;

                        let relative_row = mouse.row.saturating_sub(area.y + 1);
                        let clicked_index = relative_row.saturating_sub(1) as usize;

                        if clicked_index < self.tool_calls.len() {
                            self.tool_list_state.select(Some(clicked_index));
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.focused_panel == FocusedPanel::ToolList {
                    let selected = self.tool_list_state.selected().unwrap_or(0);
                    if selected > 0 {
                        self.tool_list_state.select(Some(selected - 1));
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focused_panel == FocusedPanel::ToolList {
                    let selected = self.tool_list_state.selected().unwrap_or(0);
                    let max_tools = self.tool_calls.len().saturating_sub(1);
                    if selected < max_tools {
                        self.tool_list_state.select(Some(selected + 1));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            KeyCode::Tab => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::ToolList => FocusedPanel::ToolDetails,
                    FocusedPanel::ToolDetails => FocusedPanel::ToolList,
                };
            }
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.tool_calls.clear();
                self.tool_list_state.select(None);
                info!("Cleared tool call history");
            }
            _ => match self.focused_panel {
                FocusedPanel::ToolList => self.handle_tool_list_key(key)?,
                FocusedPanel::ToolDetails => self.handle_tool_details_key(key)?,
            },
        }
        Ok(())
    }

    fn handle_tool_list_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = self.tool_list_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.tool_list_state.select(Some(selected - 1));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let selected = self.tool_list_state.selected().unwrap_or(0);
                let max_tools = self.tool_calls.len().saturating_sub(1);
                if selected < max_tools {
                    self.tool_list_state.select(Some(selected + 1));
                }
            }
            KeyCode::Home => {
                if !self.tool_calls.is_empty() {
                    self.tool_list_state.select(Some(0));
                }
            }
            KeyCode::End => {
                if !self.tool_calls.is_empty() {
                    self.tool_list_state.select(Some(self.tool_calls.len() - 1));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_tool_details_key(&mut self, _key: KeyEvent) -> Result<()> {
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Tool list
                Constraint::Percentage(50), // Tool details
            ])
            .split(size);

        // Render tool list
        self.render_tool_list(frame, main_chunks[0]);

        // Render tool details
        self.render_tool_details(frame, main_chunks[1]);

        // Show help if requested
        if self.show_help {
            show_popup(
                frame,
                "Help - Tool Activity",
                "Navigation:\\n\\\n                 Tab         - Switch focus between panels\\n\\\n                 ↑/k         - Move up in tool list\\n\\\n                 ↓/j         - Move down in tool list\\n\\\n                 Click       - Focus and select tool call\\n\\\n                 Ctrl+C      - Clear tool call history\\n\\\n                 \\n\\\n                 Tool Status Icons:\\n\\\n                 🟡 Starting    - Tool call initiated\\n\\\n                 🔄 In Progress - Tool is executing\\n\\\n                 ✅ Completed   - Tool finished successfully\\n\\\n                 ❌ Failed      - Tool execution failed\\n\\\n                 \\n\\\n                 Mode Switching:\\n\\\n                 Ctrl+B      - Memory Blocks (view/edit AI memory)\\n\\\n                 F2          - Configuration\\n\\\n                 Esc         - Back to conversation\\n\\\n                 \\n\\\n                 System:\\n\\\n                 F1          - Toggle this help\\n\\\n                 Ctrl+Q      - Quit application\\n\\\n                 \\n\\\n                 This panel shows real-time AI tool usage including\\n\\\n                 search queries, calculations, and website fetches.",
                (80, 70),
            );
        }
    }

    fn render_tool_list(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::ToolList;

        self.tool_list_area = Some(area);

        let items: Vec<ListItem> = self
            .tool_calls
            .iter()
            .map(|tool_call| {
                let duration_text = if let Some(duration) = tool_call.duration_ms {
                    format!(" ({}ms)", duration)
                } else {
                    "".to_string()
                };

                let content = Line::from(vec![
                    Span::styled(
                        tool_call.get_status_icon(),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        format!("[{}] ", tool_call.format_timestamp()),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        &tool_call.tool_name,
                        Style::default()
                            .fg(tool_call.get_status_color())
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(duration_text, Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!(" - {}", tool_call.agent_name),
                        Style::default().fg(Color::Cyan),
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

        let title = format!("Tool Calls ({})", self.tool_calls.len());
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

        frame.render_stateful_widget(list, area, &mut self.tool_list_state);

        // Render scrollbar
        let tools_len = self.tool_calls.len();

        self.scroll_state = self.scroll_state.content_length(tools_len);
        if let Some(selected) = self.tool_list_state.selected() {
            self.scroll_state = self.scroll_state.position(selected);
        }

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut self.scroll_state,
        );
    }

    fn render_tool_details(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::ToolDetails;
        let selected_tool = self
            .tool_list_state
            .selected()
            .and_then(|i| self.tool_calls.get(i));

        let content = if let Some(tool_call) = selected_tool {
            let status_text = match &tool_call.status {
                ToolCallStatus::Starting => "Starting".to_string(),
                ToolCallStatus::InProgress => "In Progress".to_string(),
                ToolCallStatus::Completed => "Completed".to_string(),
                ToolCallStatus::Failed(error) => format!("Failed: {}", error),
            };

            let duration_text = if let Some(duration) = tool_call.duration_ms {
                format!("{}ms", duration)
            } else {
                "N/A".to_string()
            };

            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Tool: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        &tool_call.tool_name,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Agent: ", Style::default().fg(Color::Cyan)),
                    Span::styled(&tool_call.agent_name, Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{} {}", tool_call.get_status_icon(), status_text),
                        Style::default().fg(tool_call.get_status_color()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Time: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        tool_call.format_timestamp(),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Duration: ", Style::default().fg(Color::Cyan)),
                    Span::styled(duration_text, Style::default().fg(Color::Gray)),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Arguments:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            // Pretty print JSON arguments
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&tool_call.arguments) {
                if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                    let pretty_lines: Vec<&str> = pretty.lines().collect();
                    for line in pretty_lines {
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::Yellow),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        &tool_call.arguments,
                        Style::default().fg(Color::Yellow),
                    )));
                }
            } else {
                lines.push(Line::from(Span::styled(
                    &tool_call.arguments,
                    Style::default().fg(Color::Yellow),
                )));
            }

            if let Some(result) = &tool_call.result {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Result:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(""));

                // Pretty print JSON result
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(result) {
                    if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                        let pretty_lines: Vec<&str> = pretty.lines().collect();
                        for line in pretty_lines {
                            lines.push(Line::from(Span::styled(
                                line.to_string(),
                                Style::default().fg(Color::Green),
                            )));
                        }
                    } else {
                        lines.push(Line::from(Span::styled(
                            result,
                            Style::default().fg(Color::Green),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        result,
                        Style::default().fg(Color::Green),
                    )));
                }
            }

            lines
        } else {
            vec![Line::from("No tool call selected")]
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
                    .title("Tool Call Details")
                    .border_style(style),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}
