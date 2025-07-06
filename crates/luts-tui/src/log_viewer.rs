use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::collections::VecDeque;
use tracing::{Level, Subscriber};
use tracing_subscriber::Layer;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Local};

/// A log entry with timestamp and level
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: Level,
    pub target: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(level: Level, target: String, message: String) -> Self {
        Self {
            timestamp: Local::now(),
            level,
            target,
            message,
        }
    }

    pub fn get_color(&self) -> Color {
        match self.level {
            Level::ERROR => Color::Red,
            Level::WARN => Color::Yellow,
            Level::INFO => Color::Green,
            Level::DEBUG => Color::Blue,
            Level::TRACE => Color::Gray,
        }
    }

    pub fn get_level_str(&self) -> &'static str {
        match self.level {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN ",
            Level::INFO => "INFO ",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        }
    }
}

/// Shared log buffer that can be accessed from multiple threads
#[derive(Debug, Clone)]
pub struct LogBuffer {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    max_entries: usize,
}

impl LogBuffer {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    pub fn add_entry(&self, entry: LogEntry) {
        if let Ok(mut entries) = self.entries.lock() {
            if entries.len() >= self.max_entries {
                entries.pop_front();
            }
            entries.push_back(entry);
        }
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.lock() {
            entries.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

/// Custom tracing layer that captures logs to our buffer
pub struct LogBufferLayer {
    buffer: LogBuffer,
}

impl LogBufferLayer {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for LogBufferLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let mut visitor = LogVisitor::new();
        event.record(&mut visitor);
        
        let entry = LogEntry::new(
            *metadata.level(),
            metadata.target().to_string(),
            visitor.message,
        );
        
        self.buffer.add_entry(entry);
    }
}

/// Visitor to extract the message from tracing events
struct LogVisitor {
    message: String,
}

impl LogVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

/// Log viewer panel for the TUI
pub struct LogViewer {
    /// Log buffer to read from
    log_buffer: LogBuffer,
    /// Current scroll position
    scroll_state: ListState,
    /// Scrollbar state
    scrollbar_state: ScrollbarState,
    /// Filter level (only show logs at this level or higher)
    filter_level: Level,
    /// Whether to auto-scroll to bottom
    auto_scroll: bool,
}

impl LogViewer {
    pub fn new(log_buffer: LogBuffer) -> Self {
        Self {
            log_buffer,
            scroll_state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
            filter_level: Level::DEBUG,
            auto_scroll: true,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => {
                self.auto_scroll = false;
                if let Some(selected) = self.scroll_state.selected() {
                    if selected > 0 {
                        self.scroll_state.select(Some(selected - 1));
                    }
                } else {
                    self.scroll_state.select(Some(0));
                }
                true
            }
            KeyCode::Down => {
                self.auto_scroll = false;
                let entries = self.get_filtered_entries();
                if let Some(selected) = self.scroll_state.selected() {
                    if selected < entries.len().saturating_sub(1) {
                        self.scroll_state.select(Some(selected + 1));
                    }
                } else if !entries.is_empty() {
                    self.scroll_state.select(Some(0));
                }
                true
            }
            KeyCode::PageUp => {
                self.auto_scroll = false;
                if let Some(selected) = self.scroll_state.selected() {
                    self.scroll_state.select(Some(selected.saturating_sub(10)));
                }
                true
            }
            KeyCode::PageDown => {
                self.auto_scroll = false;
                let entries = self.get_filtered_entries();
                if let Some(selected) = self.scroll_state.selected() {
                    self.scroll_state.select(Some((selected + 10).min(entries.len().saturating_sub(1))));
                }
                true
            }
            KeyCode::Home => {
                self.auto_scroll = false;
                self.scroll_state.select(Some(0));
                true
            }
            KeyCode::End => {
                self.auto_scroll = true;
                let entries = self.get_filtered_entries();
                if !entries.is_empty() {
                    self.scroll_state.select(Some(entries.len() - 1));
                }
                true
            }
            KeyCode::Char('c') => {
                self.log_buffer.clear();
                self.scroll_state.select(None);
                true
            }
            KeyCode::Char('a') => {
                self.auto_scroll = !self.auto_scroll;
                true
            }
            KeyCode::Char('1') => {
                self.filter_level = Level::ERROR;
                true
            }
            KeyCode::Char('2') => {
                self.filter_level = Level::WARN;
                true
            }
            KeyCode::Char('3') => {
                self.filter_level = Level::INFO;
                true
            }
            KeyCode::Char('4') => {
                self.filter_level = Level::DEBUG;
                true
            }
            KeyCode::Char('5') => {
                self.filter_level = Level::TRACE;
                true
            }
            _ => false,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(2)])
            .split(area);

        // Main log area
        let entries = self.get_filtered_entries();
        
        // Auto-scroll to bottom if enabled
        if self.auto_scroll && !entries.is_empty() {
            self.scroll_state.select(Some(entries.len() - 1));
        }

        let items: Vec<ListItem> = entries
            .iter()
            .map(|entry| {
                let timestamp = entry.timestamp.format("%H:%M:%S%.3f");
                let level_str = entry.get_level_str();
                let color = entry.get_color();
                
                let line = Line::from(vec![
                    Span::styled(format!("{} ", timestamp), Style::default().fg(Color::Gray)),
                    Span::styled(format!("[{}] ", level_str), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{}: ", entry.target), Style::default().fg(Color::Cyan)),
                    Span::raw(&entry.message),
                ]);
                
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Logs (Level: {:?}, Auto-scroll: {})", self.filter_level, if self.auto_scroll { "ON" } else { "OFF" }))
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(list, chunks[0], &mut self.scroll_state);

        // Scrollbar
        if !entries.is_empty() {
            self.scrollbar_state = self.scrollbar_state.content_length(entries.len());
            if let Some(selected) = self.scroll_state.selected() {
                self.scrollbar_state = self.scrollbar_state.position(selected);
            }
            
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            
            f.render_stateful_widget(scrollbar, chunks[0], &mut self.scrollbar_state);
        }

        // Help text
        let help = Paragraph::new("↑/↓: Scroll | PgUp/PgDn: Page | Home/End: Jump | c: Clear | a: Toggle auto-scroll | 1-5: Filter level | Esc: Back")
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[1]);
    }

    fn get_filtered_entries(&self) -> Vec<LogEntry> {
        self.log_buffer.get_entries()
            .into_iter()
            .filter(|entry| entry.level <= self.filter_level)
            .collect()
    }
}