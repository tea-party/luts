//! Configuration system for LUTS TUI
//!
//! This module provides a configuration system that allows users to customize:
//! - UI themes and colors
//! - Keybindings
//! - Default settings
//! - Provider configurations

use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// UI theme configuration
    pub theme: ThemeConfig,
    /// Keybinding configuration
    pub keybindings: KeybindingConfig,
    /// Default application settings
    pub defaults: DefaultsConfig,
    /// Provider-specific configurations
    pub providers: HashMap<String, ProviderConfig>,
}

/// UI Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Color scheme name
    pub name: String,
    /// Border colors for focused/unfocused elements
    pub border_focused: String,
    pub border_unfocused: String,
    /// Text colors
    pub text_primary: String,
    pub text_secondary: String,
    pub text_accent: String,
    /// Status colors
    pub success: String,
    pub warning: String,
    pub error: String,
    pub info: String,
}

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    /// Global keybindings
    pub global: GlobalKeybindings,
    /// Agent selection mode keybindings
    pub agent_selection: AgentSelectionKeybindings,
    /// Conversation mode keybindings
    pub conversation: ConversationKeybindings,
    /// Memory blocks mode keybindings
    pub memory_blocks: MemoryBlocksKeybindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalKeybindings {
    pub quit: Vec<String>,
    pub help: Vec<String>,
    pub switch_to_agent_selection: Vec<String>,
    pub switch_to_conversation: Vec<String>,
    pub switch_to_memory_blocks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSelectionKeybindings {
    pub select_agent: Vec<String>,
    pub move_up: Vec<String>,
    pub move_down: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationKeybindings {
    pub send_message: Vec<String>,
    pub switch_focus: Vec<String>,
    pub scroll_up: Vec<String>,
    pub scroll_down: Vec<String>,
    pub clear_input: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBlocksKeybindings {
    pub create_block: Vec<String>,
    pub edit_block: Vec<String>,
    pub delete_block: Vec<String>,
    pub switch_panel: Vec<String>,
}

/// Default application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Default data directory
    pub data_dir: String,
    /// Default LLM provider
    pub provider: String,
    /// Default agent
    pub agent: Option<String>,
    /// Auto-save settings
    pub auto_save: bool,
    /// Maximum log entries
    pub max_log_entries: usize,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider display name
    pub name: String,
    /// Provider-specific settings
    pub settings: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            keybindings: KeybindingConfig::default(),
            defaults: DefaultsConfig::default(),
            providers: HashMap::new(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            border_focused: "#00FF00".to_string(),
            border_unfocused: "#808080".to_string(),
            text_primary: "#FFFFFF".to_string(),
            text_secondary: "#CCCCCC".to_string(),
            text_accent: "#00FFFF".to_string(),
            success: "#00FF00".to_string(),
            warning: "#FFFF00".to_string(),
            error: "#FF0000".to_string(),
            info: "#0080FF".to_string(),
        }
    }
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            global: GlobalKeybindings::default(),
            agent_selection: AgentSelectionKeybindings::default(),
            conversation: ConversationKeybindings::default(),
            memory_blocks: MemoryBlocksKeybindings::default(),
        }
    }
}

impl Default for GlobalKeybindings {
    fn default() -> Self {
        Self {
            quit: vec!["q".to_string(), "Ctrl+c".to_string()],
            help: vec!["F1".to_string(), "?".to_string()],
            switch_to_agent_selection: vec!["Ctrl+a".to_string()],
            switch_to_conversation: vec!["Ctrl+Enter".to_string()],
            switch_to_memory_blocks: vec!["Ctrl+b".to_string()],
        }
    }
}

impl Default for AgentSelectionKeybindings {
    fn default() -> Self {
        Self {
            select_agent: vec!["Enter".to_string(), "Space".to_string()],
            move_up: vec!["Up".to_string(), "k".to_string()],
            move_down: vec!["Down".to_string(), "j".to_string()],
        }
    }
}

impl Default for ConversationKeybindings {
    fn default() -> Self {
        Self {
            send_message: vec!["Enter".to_string()],
            switch_focus: vec!["Tab".to_string()],
            scroll_up: vec!["Up".to_string()],
            scroll_down: vec!["Down".to_string()],
            clear_input: vec!["Ctrl+l".to_string()],
        }
    }
}

impl Default for MemoryBlocksKeybindings {
    fn default() -> Self {
        Self {
            create_block: vec!["n".to_string(), "Ctrl+n".to_string()],
            edit_block: vec!["e".to_string(), "Enter".to_string()],
            delete_block: vec!["d".to_string(), "Delete".to_string()],
            switch_panel: vec!["Tab".to_string()],
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            provider: "openai".to_string(),
            agent: None,
            auto_save: true,
            max_log_entries: 1000,
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        let config: Config = toml::from_str(&contents)
            .with_context(|| "Failed to parse config file as TOML")?;
        
        Ok(config)
    }

    /// Load configuration with fallback to defaults
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        match Self::load_from_file(&path) {
            Ok(config) => Ok(config),
            Err(_) => {
                // If file doesn't exist or is invalid, create default config
                let config = Self::default();
                // Try to save the default config
                let _ = config.save_to_file(&path);
                Ok(config)
            }
        }
    }

    /// Get default config file path
    pub fn config_path() -> Result<PathBuf> {
        Self::default_config_file()
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.save_to_file(path)
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize config to TOML")?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
        
        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;
        
        Ok(())
    }

    /// Get the configuration directory path
    pub fn config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|dir| dir.join("luts"))
            .with_context(|| "Failed to get config directory")
    }

    /// Get the default config file path
    pub fn default_config_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Parse a color string to ratatui Color
    pub fn parse_color(color_str: &str) -> Color {
        if let Some(stripped) = color_str.strip_prefix('#') {
            if stripped.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&stripped[0..2], 16),
                    u8::from_str_radix(&stripped[2..4], 16),
                    u8::from_str_radix(&stripped[4..6], 16),
                ) {
                    return Color::Rgb(r, g, b);
                }
            }
        }
        
        // Named colors
        match color_str.to_lowercase().as_str() {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "gray" | "grey" => Color::Gray,
            "darkgray" | "darkgrey" => Color::DarkGray,
            "lightred" => Color::LightRed,
            "lightgreen" => Color::LightGreen,
            "lightyellow" => Color::LightYellow,
            "lightblue" => Color::LightBlue,
            "lightmagenta" => Color::LightMagenta,
            "lightcyan" => Color::LightCyan,
            "white" => Color::White,
            _ => Color::White, // Fallback
        }
    }

    /// Get a color from the theme using the parse_color method
    pub fn get_color(&self, color_str: &str) -> Color {
        Self::parse_color(color_str)
    }
}