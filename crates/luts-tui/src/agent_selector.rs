//! Agent selection TUI component

use crate::{
    components::{SelectableList, show_popup},
    events::AppEvent,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use luts_core::agents::PersonalityAgentBuilder;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tokio::sync::mpsc;

pub struct AgentSelector {
    agent_list: SelectableList,
    agent_details: Vec<(String, String, String)>, // (id, name, description)
    event_sender: mpsc::UnboundedSender<AppEvent>,
    show_help: bool,
    list_area: Option<Rect>, // Store the list area for mouse handling
    last_click: Option<(u16, u16, std::time::Instant)>, // Track last click for double-click detection
}

impl AgentSelector {
    pub fn new(event_sender: mpsc::UnboundedSender<AppEvent>) -> Self {
        let personalities = PersonalityAgentBuilder::list_personalities();
        let agent_names: Vec<String> = personalities
            .iter()
            .map(|(_, name, _)| name.to_string())
            .collect();

        let agent_list = SelectableList::new("Available Agents".to_string(), agent_names);

        Self {
            agent_list,
            agent_details: personalities
                .into_iter()
                .map(|(id, name, desc)| (id.to_string(), name.to_string(), desc.to_string()))
                .collect(),
            event_sender,
            show_help: false,
            list_area: None,
            last_click: None,
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                // Check if click is within the list area
                if let Some(area) = self.list_area {
                    if mouse.column >= area.x && mouse.column < area.x + area.width
                        && mouse.row >= area.y && mouse.row < area.y + area.height {
                        // Calculate which item was clicked based on position
                        // Account for border (1 row) and title (1 row), plus the highlight symbol offset
                        let relative_row = mouse.row.saturating_sub(area.y + 1); // +1 for top border only
                        let clicked_index = relative_row.saturating_sub(1) as usize; // -1 for title row
                        
                        if clicked_index < self.agent_details.len() {
                            self.agent_list.state.select(Some(clicked_index));
                            
                            // Check for double-click
                            let now = std::time::Instant::now();
                            let is_double_click = if let Some((last_col, last_row, last_time)) = self.last_click {
                                mouse.column == last_col 
                                    && mouse.row == last_row 
                                    && now.duration_since(last_time).as_millis() < 500 // 500ms double-click threshold
                            } else {
                                false
                            };
                            
                            if is_double_click {
                                // Double-click: select the agent
                                if let Some((agent_id, _, _)) = self.agent_details.get(clicked_index) {
                                    self.event_sender.send(AppEvent::AgentSelected(agent_id.clone()))?;
                                }
                                self.last_click = None; // Reset to prevent triple-click
                            } else {
                                // Single click: just update selection and record click
                                self.last_click = Some((mouse.column, mouse.row, now));
                            }
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                self.agent_list.previous();
            }
            MouseEventKind::ScrollDown => {
                self.agent_list.next();
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.agent_list.previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.agent_list.next();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(selected_index) = self.agent_list.state.selected() {
                    if let Some((agent_id, _, _)) = self.agent_details.get(selected_index) {
                        self.event_sender
                            .send(AppEvent::AgentSelected(agent_id.clone()))?;
                    }
                }
            }
            KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(size);

        // Store the list area for mouse handling
        self.list_area = Some(chunks[0]);

        // Render agent list
        self.agent_list.render(chunks[0], frame.buffer_mut(), true);

        // Render agent details
        self.render_agent_details(frame, chunks[1]);

        // Show help if requested
        if self.show_help {
            show_popup(
                frame,
                "Help - Agent Selection",
                "Navigation:\n\
                 ↑/k         - Move up\n\
                 ↓/j         - Move down\n\
                 Enter/Space - Select agent and start conversation\n\
                 Click       - Select item\n\
                 Double-click- Select agent and start conversation\n\
                 \n\
                 Mode Switching:\n\
                 Ctrl+B      - Memory Blocks (view/edit AI memory)\n\
                 Ctrl+T      - Tool Activity (monitor AI tool usage)\n\
                 F2          - Configuration\n\
                 \n\
                 System:\n\
                 F1          - Toggle this help\n\
                 Ctrl+Q      - Quit application",
                (60, 40),
            );
        }
    }

    fn render_agent_details(&self, frame: &mut Frame, area: Rect) {
        let selected_index = self.agent_list.state.selected().unwrap_or(0);
        let (agent_id, agent_name, description) = self
            .agent_details
            .get(selected_index)
            .cloned()
            .unwrap_or_else(|| {
                (
                    "unknown".to_string(),
                    "Unknown".to_string(),
                    "No description".to_string(),
                )
            });

        let content = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::styled(agent_name, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::Cyan)),
                Span::styled(agent_id, Style::default().fg(Color::Gray)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Description:",
                Style::default().fg(Color::Cyan),
            )]),
            Line::from(description),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tools: ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    self.get_agent_tools(selected_index),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Agent Details")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn get_agent_tools(&self, index: usize) -> String {
        match index {
            0 => "search, website".to_string(),       // Dr. Research
            1 => "calc".to_string(),                  // Logic
            2 => "none (pure reasoning)".to_string(), // Spark
            3 => "calc, search, website".to_string(), // Maestro
            4 => "calc, search".to_string(),          // Practical
            _ => "unknown".to_string(),
        }
    }
}
