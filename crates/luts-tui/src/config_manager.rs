//! Configuration management TUI component

use crate::{
    components::show_popup,
    config::Config,
    events::AppEvent,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConfigSection {
    Theme,
    Keybindings,
    Defaults,
    Providers,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FocusedPanel {
    SectionTabs,
    SettingsList,
    SettingEditor,
}

pub struct ConfigManager {
    config: Config,
    config_path: PathBuf,
    current_section: ConfigSection,
    focused_panel: FocusedPanel,
    settings_list_state: ListState,
    _event_sender: mpsc::UnboundedSender<AppEvent>,
    show_help: bool,
    show_save_dialog: bool,
    editing_setting: Option<String>,
    editor_content: String,
    unsaved_changes: bool,
}

impl ConfigManager {
    pub fn new(event_sender: mpsc::UnboundedSender<AppEvent>) -> Result<Self> {
        let config_path = Config::config_path()?;
        let config = Config::load(&config_path)?;

        let mut settings_list_state = ListState::default();
        settings_list_state.select(Some(0));

        info!("Loaded configuration from: {:?}", config_path);

        Ok(Self {
            config,
            config_path,
            current_section: ConfigSection::Theme,
            focused_panel: FocusedPanel::SectionTabs,
            settings_list_state,
            _event_sender: event_sender,
            show_help: false,
            show_save_dialog: false,
            editing_setting: None,
            editor_content: String::new(),
            unsaved_changes: false,
        })
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Down(_) => {
                // Mouse click handling for config manager
                // Could focus different panels or sections based on click location
            }
            MouseEventKind::ScrollUp => {
                if self.focused_panel == FocusedPanel::SettingsList {
                    let settings_count = self.get_current_settings().len();
                    if settings_count > 0 {
                        let selected = self.settings_list_state.selected().unwrap_or(0);
                        if selected > 0 {
                            self.settings_list_state.select(Some(selected - 1));
                        }
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focused_panel == FocusedPanel::SettingsList {
                    let settings_count = self.get_current_settings().len();
                    if settings_count > 0 {
                        let selected = self.settings_list_state.selected().unwrap_or(0);
                        if selected < settings_count.saturating_sub(1) {
                            self.settings_list_state.select(Some(selected + 1));
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        if self.show_save_dialog {
            return self.handle_save_dialog_key(key);
        }

        match key.code {
            KeyCode::F(1) => {
                self.show_help = !self.show_help;
            }
            KeyCode::Tab => {
                self.focused_panel = match self.focused_panel {
                    FocusedPanel::SectionTabs => FocusedPanel::SettingsList,
                    FocusedPanel::SettingsList => FocusedPanel::SettingEditor,
                    FocusedPanel::SettingEditor => FocusedPanel::SectionTabs,
                };
            }
            KeyCode::Char('s') if self.focused_panel != FocusedPanel::SettingEditor 
                && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.save_config()?;
            }
            KeyCode::Char('r') if self.focused_panel != FocusedPanel::SettingEditor 
                && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.reload_config()?;
            }
            KeyCode::Char('d') if self.focused_panel != FocusedPanel::SettingEditor 
                && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                self.reset_to_defaults();
            }
            _ => match self.focused_panel {
                FocusedPanel::SectionTabs => self.handle_section_tabs_key(key)?,
                FocusedPanel::SettingsList => self.handle_settings_list_key(key)?,
                FocusedPanel::SettingEditor => self.handle_setting_editor_key(key)?,
            },
        }
        Ok(())
    }

    fn handle_save_dialog_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.save_config()?;
                self.show_save_dialog = false;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_save_dialog = false;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_section_tabs_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.current_section = match self.current_section {
                    ConfigSection::Theme => ConfigSection::Providers,
                    ConfigSection::Keybindings => ConfigSection::Theme,
                    ConfigSection::Defaults => ConfigSection::Keybindings,
                    ConfigSection::Providers => ConfigSection::Defaults,
                };
                self.settings_list_state.select(Some(0));
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.current_section = match self.current_section {
                    ConfigSection::Theme => ConfigSection::Keybindings,
                    ConfigSection::Keybindings => ConfigSection::Defaults,
                    ConfigSection::Defaults => ConfigSection::Providers,
                    ConfigSection::Providers => ConfigSection::Theme,
                };
                self.settings_list_state.select(Some(0));
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_settings_list_key(&mut self, key: KeyEvent) -> Result<()> {
        let settings_count = self.get_current_settings().len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = self.settings_list_state.selected().unwrap_or(0);
                if selected > 0 {
                    self.settings_list_state.select(Some(selected - 1));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let selected = self.settings_list_state.selected().unwrap_or(0);
                if selected < settings_count.saturating_sub(1) {
                    self.settings_list_state.select(Some(selected + 1));
                }
            }
            KeyCode::Enter => {
                if let Some(selected) = self.settings_list_state.selected() {
                    let settings = self.get_current_settings();
                    if let Some((setting_name, current_value)) = settings.get(selected) {
                        self.editing_setting = Some(setting_name.clone());
                        self.editor_content = current_value.clone();
                        self.focused_panel = FocusedPanel::SettingEditor;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_setting_editor_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.editing_setting = None;
                self.editor_content.clear();
                self.focused_panel = FocusedPanel::SettingsList;
            }
            KeyCode::Enter => {
                if let Some(setting_name) = self.editing_setting.clone() {
                    let editor_content = self.editor_content.clone();
                    self.update_setting(&setting_name, &editor_content)?;
                    self.editing_setting = None;
                    self.editor_content.clear();
                    self.focused_panel = FocusedPanel::SettingsList;
                }
            }
            KeyCode::Char(c) => {
                self.editor_content.push(c);
            }
            KeyCode::Backspace => {
                self.editor_content.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn get_current_settings(&self) -> Vec<(String, String)> {
        match self.current_section {
            ConfigSection::Theme => vec![
                ("Theme Name".to_string(), self.config.theme.name.clone()),
                (
                    "Border Focused".to_string(),
                    self.config.theme.border_focused.clone(),
                ),
                (
                    "Border Unfocused".to_string(),
                    self.config.theme.border_unfocused.clone(),
                ),
                (
                    "Text Primary".to_string(),
                    self.config.theme.text_primary.clone(),
                ),
                (
                    "Text Secondary".to_string(),
                    self.config.theme.text_secondary.clone(),
                ),
                (
                    "Success Color".to_string(),
                    self.config.theme.success.clone(),
                ),
                (
                    "Warning Color".to_string(),
                    self.config.theme.warning.clone(),
                ),
                ("Error Color".to_string(), self.config.theme.error.clone()),
            ],
            ConfigSection::Keybindings => vec![
                (
                    "Global Quit".to_string(),
                    self.config.keybindings.global.quit.join(", "),
                ),
                (
                    "Global Help".to_string(),
                    self.config.keybindings.global.help.join(", "),
                ),
                (
                    "Switch to Memory Blocks".to_string(),
                    self.config
                        .keybindings
                        .global
                        .switch_to_memory_blocks
                        .join(", "),
                ),
                (
                    "Agent Move Up".to_string(),
                    self.config.keybindings.agent_selection.move_up.join(", "),
                ),
                (
                    "Agent Move Down".to_string(),
                    self.config.keybindings.agent_selection.move_down.join(", "),
                ),
                (
                    "Conversation Send".to_string(),
                    self.config.keybindings.conversation.send_message.join(", "),
                ),
                (
                    "Memory Block Create".to_string(),
                    self.config.keybindings.memory_blocks.create_block.join(", "),
                ),
            ],
            ConfigSection::Defaults => vec![
                (
                    "Data Directory".to_string(),
                    self.config.defaults.data_dir.clone(),
                ),
                (
                    "Default Provider".to_string(),
                    self.config.defaults.provider.clone(),
                ),
                ("Default Agent".to_string(), self.config.defaults.agent.as_ref().unwrap_or(&"None".to_string()).clone()),
                ("Auto Save".to_string(), self.config.defaults.auto_save.to_string()),
                ("Max Log Entries".to_string(), self.config.defaults.max_log_entries.to_string()),
            ],
            ConfigSection::Providers => {
                let mut settings = Vec::new();
                for (provider_id, provider_config) in &self.config.providers {
                    settings.push((
                        format!("{} Name", provider_id),
                        provider_config.name.clone(),
                    ));
                    settings.push((
                        format!("{} Settings Count", provider_id),
                        provider_config.settings.len().to_string(),
                    ));
                }
                settings
            }
        }
    }

    fn update_setting(&mut self, setting_name: &str, new_value: &str) -> Result<()> {
        debug!("Updating setting: {} = {}", setting_name, new_value);

        match self.current_section {
            ConfigSection::Theme => match setting_name {
                "Theme Name" => self.config.theme.name = new_value.to_string(),
                "Border Focused" => self.config.theme.border_focused = new_value.to_string(),
                "Border Unfocused" => self.config.theme.border_unfocused = new_value.to_string(),
                "Text Primary" => self.config.theme.text_primary = new_value.to_string(),
                "Text Secondary" => self.config.theme.text_secondary = new_value.to_string(),
                "Success Color" => self.config.theme.success = new_value.to_string(),
                "Warning Color" => self.config.theme.warning = new_value.to_string(),
                "Error Color" => self.config.theme.error = new_value.to_string(),
                _ => {
                    warn!("Unknown theme setting: {}", setting_name);
                    return Ok(());
                }
            },
            ConfigSection::Defaults => match setting_name {
                "Data Directory" => self.config.defaults.data_dir = new_value.to_string(),
                "Default Provider" => self.config.defaults.provider = new_value.to_string(),
                "Default Agent" => {
                    self.config.defaults.agent =
                        if new_value == "None" || new_value.is_empty() {
                            None
                        } else {
                            Some(new_value.to_string())
                        };
                }
                "Auto Save" => {
                    if let Ok(enable) = new_value.parse::<bool>() {
                        self.config.defaults.auto_save = enable;
                    }
                }
                "Max Log Entries" => {
                    if let Ok(entries) = new_value.parse::<usize>() {
                        self.config.defaults.max_log_entries = entries;
                    }
                }
                _ => {
                    warn!("Unknown defaults setting: {}", setting_name);
                    return Ok(());
                }
            },
            _ => {
                // For now, only theme and defaults are editable
                warn!(
                    "Setting editing not yet implemented for section: {:?}",
                    self.current_section
                );
                return Ok(());
            }
        }

        self.unsaved_changes = true;
        info!("Updated setting: {} = {}", setting_name, new_value);
        Ok(())
    }

    fn save_config(&mut self) -> Result<()> {
        self.config.save(&self.config_path)?;
        self.unsaved_changes = false;
        info!("Configuration saved to: {:?}", self.config_path);
        Ok(())
    }

    fn reload_config(&mut self) -> Result<()> {
        self.config = Config::load(&self.config_path)?;
        self.unsaved_changes = false;
        info!("Configuration reloaded from: {:?}", self.config_path);
        Ok(())
    }

    fn reset_to_defaults(&mut self) {
        self.config = Config::default();
        self.unsaved_changes = true;
        info!("Configuration reset to defaults");
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(10),   // Content
                Constraint::Length(3), // Status
            ])
            .split(size);

        // Render section tabs
        self.render_section_tabs(frame, main_chunks[0]);

        // Split content area
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Settings list
                Constraint::Percentage(50), // Setting editor
            ])
            .split(main_chunks[1]);

        // Render settings list
        self.render_settings_list(frame, content_chunks[0]);

        // Render setting editor
        self.render_setting_editor(frame, content_chunks[1]);

        // Render status bar
        self.render_status_bar(frame, main_chunks[2]);

        // Show dialogs
        if self.show_save_dialog {
            self.render_save_dialog(frame);
        }

        if self.show_help {
            show_popup(
                frame,
                "Help - Configuration Manager",
                "Navigation:\n\
                 Tab         - Switch focus between panels\n\
                 ←/→/h/l     - Switch section tabs\n\
                 ↑/↓/k/j     - Navigate settings list\n\
                 Enter       - Edit selected setting\n\
                 Ctrl+s      - Save configuration\n\
                 Ctrl+r      - Reload configuration\n\
                 Ctrl+d      - Reset to defaults\n\
                 F1          - Toggle help\n\
                 Ctrl+q/Esc  - Return to main mode",
                (70, 50),
            );
        }
    }

    fn render_section_tabs(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::SectionTabs;
        let titles = vec!["Theme", "Keybindings", "Defaults", "Providers"];

        let selected_index = match self.current_section {
            ConfigSection::Theme => 0,
            ConfigSection::Keybindings => 1,
            ConfigSection::Defaults => 2,
            ConfigSection::Providers => 3,
        };

        let style = if focused {
            Style::default().fg(self.config.get_color(&self.config.theme.border_focused))
        } else {
            Style::default().fg(self.config.get_color(&self.config.theme.border_unfocused))
        };

        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration Sections")
                    .border_style(style),
            )
            .style(Style::default().fg(self.config.get_color(&self.config.theme.text_primary)))
            .highlight_style(
                Style::default()
                    .fg(self.config.get_color(&self.config.theme.text_accent))
                    .add_modifier(Modifier::BOLD),
            )
            .select(selected_index);

        frame.render_widget(tabs, area);
    }

    fn render_settings_list(&mut self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::SettingsList;
        let settings = self.get_current_settings();

        let items: Vec<ListItem> = settings
            .iter()
            .map(|(name, value)| {
                let content = Line::from(vec![
                    Span::styled(
                        format!("{}: ", name),
                        Style::default().fg(self.config.get_color(&self.config.theme.text_primary)),
                    ),
                    Span::styled(
                        value.clone(),
                        Style::default().fg(self.config.get_color(&self.config.theme.text_primary)),
                    ),
                ]);
                ListItem::new(content)
            })
            .collect();

        let style = if focused {
            Style::default().fg(self.config.get_color(&self.config.theme.border_focused))
        } else {
            Style::default().fg(self.config.get_color(&self.config.theme.border_unfocused))
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Settings")
                    .border_style(style),
            )
            .style(Style::default().fg(self.config.get_color(&self.config.theme.text_primary)))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.settings_list_state);
    }

    fn render_setting_editor(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focused_panel == FocusedPanel::SettingEditor;
        let title = if let Some(setting_name) = &self.editing_setting {
            format!("Editing: {}", setting_name)
        } else {
            "Setting Editor".to_string()
        };

        let content = if self.editing_setting.is_some() {
            format!(
                "{}\n\nPress Enter to save, Esc to cancel",
                self.editor_content
            )
        } else {
            "Select a setting from the list to edit".to_string()
        };

        let style = if focused {
            Style::default().fg(self.config.get_color(&self.config.theme.border_focused))
        } else {
            Style::default().fg(self.config.get_color(&self.config.theme.border_unfocused))
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(style),
            )
            .style(Style::default().fg(self.config.get_color(&self.config.theme.text_primary)))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let mut status_parts = vec![Span::styled(
            format!(
                "Config: {:?} ",
                self.config_path.file_name().unwrap_or_default()
            ),
            Style::default().fg(self.config.get_color(&self.config.theme.text_secondary)),
        )];

        if self.unsaved_changes {
            status_parts.push(Span::styled(
                "[UNSAVED] ",
                Style::default().fg(self.config.get_color(&self.config.theme.warning)),
            ));
        } else {
            status_parts.push(Span::styled(
                "[SAVED] ",
                Style::default().fg(self.config.get_color(&self.config.theme.success)),
            ));
        }

        status_parts.push(Span::styled(
            "Ctrl+s=save Ctrl+r=reload Ctrl+d=defaults F1=help",
            Style::default().fg(self.config.get_color(&self.config.theme.text_secondary)),
        ));

        let content = Text::from(Line::from(status_parts));

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Status")
                    .border_style(
                        Style::default()
                            .fg(self.config.get_color(&self.config.theme.border_unfocused)),
                    ),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_save_dialog(&self, frame: &mut Frame) {
        let area = self.centered_rect(50, 20, frame.area());
        frame.render_widget(Clear, area);

        let content = "Save configuration changes?\n\nPress 'y' to save, 'n' to discard";

        let block = Block::default()
            .title("Save Changes")
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
