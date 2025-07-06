//! Event handling for the TUI application

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseEvent};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16), // width, height
    Tick,
    Quit,
    AgentSelected(String),
    MessageSent(String),
    AgentResponseReceived(String),
    AgentResponseError(String),
    AgentProcessingStarted,
    AgentProcessingFinished,
    // Streaming events with ResponseChunk
    StreamingChunk(luts_core::response_streaming::ResponseChunk),
    StreamingComplete,
    StreamingError(String),
}

pub struct EventHandler {
    sender: mpsc::UnboundedSender<AppEvent>,
    receiver: mpsc::UnboundedReceiver<AppEvent>,
    last_tick: Instant,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            last_tick: Instant::now(),
            tick_rate,
        }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.sender.clone()
    }

    pub async fn next_event(&mut self) -> Result<AppEvent> {
        let timeout = self
            .tick_rate
            .checked_sub(self.last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        tokio::select! {
            // Handle incoming events from the channel
            event = self.receiver.recv() => {
                if let Some(event) = event {
                    Ok(event)
                } else {
                    Ok(AppEvent::Quit)
                }
            }

            // Handle terminal events
            _ = tokio::time::sleep(timeout) => {
                if crossterm::event::poll(Duration::from_millis(0))? {
                    match event::read()? {
                        Event::Key(key) => Ok(AppEvent::Key(key)),
                        Event::Mouse(mouse) => Ok(AppEvent::Mouse(mouse)),
                        Event::Resize(width, height) => Ok(AppEvent::Resize(width, height)),
                        _ => Ok(AppEvent::Tick),
                    }
                } else {
                    self.last_tick = Instant::now();
                    Ok(AppEvent::Tick)
                }
            }
        }
    }
}

/// Handle global key events
pub fn handle_key_event(key: KeyEvent) -> Option<AppEvent> {
    match key.code {
        // Only Ctrl+Q exits the application entirely
        KeyCode::Char('q') | KeyCode::Char('c')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            Some(AppEvent::Quit)
        }
        _ => None,
    }
}
