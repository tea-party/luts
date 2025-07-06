//! Configuration system for LUTS TUI
//!
//! This module provides a comprehensive configuration system that allows users to customize:
//! - UI themes and colors
//! - Keybindings
//! - Block templates and presets
//! - Default settings
//! - Provider configurations

use crate::blocks::{BlockType, ContextBlock};
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
    /// Block templates and presets
    pub blocks: BlockConfig,
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
    /// Block type colors
    pub block_system: String,
    pub block_user: String,
    pub block_memory: String,
    pub block_tool: String,
    pub block_response: String,
    pub block_example: String,
    pub block_dynamic: String,
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
    /// Block mode keybindings
    pub block_mode: BlockModeKeybindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalKeybindings {
    pub quit: Vec<String>,
    pub help: Vec<String>,
    pub switch_to_agent_selection: Vec<String>,
    pub switch_to_conversation: Vec<String>,
    pub switch_to_block_mode: Vec<String>,
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
pub struct BlockModeKeybindings {
    pub toggle_block_status: Vec<String>,
    pub create_block: Vec<String>,
    pub process_next: Vec<String>,
    pub reset_workflow: Vec<String>,
    pub switch_panel: Vec<String>,
}

/// Block templates and presets configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockConfig {
    /// Pre-defined block templates
    pub templates: HashMap<String, BlockTemplate>,
    /// Workflow presets
    pub workflows: HashMap<String, WorkflowPreset>,
    /// Default block settings
    pub defaults: BlockDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTemplate {
    pub name: String,
    pub description: String,
    pub block_type: BlockType,
    pub content_template: String,
    pub default_priority: u8,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPreset {
    pub name: String,
    pub description: String,
    pub blocks: Vec<WorkflowBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowBlock {
    pub template_id: String,
    pub content: Option<String>,
    pub dependencies: Vec<String>,
    pub priority: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefaults {
    pub auto_complete_dependencies: bool,
    pub show_dependency_warnings: bool,
    pub default_priority: u8,
    pub auto_scroll_to_ready: bool,
}

/// Default application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Default data directory
    pub data_dir: String,
    /// Default LLM provider
    pub provider: String,
    /// Default agent
    pub default_agent: Option<String>,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Maximum history length
    pub max_history_length: usize,
    /// Enable streaming by default
    pub enable_streaming: bool,
    /// Default log level
    pub log_level: String,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Display name
    pub display_name: String,
    /// API configuration
    pub api: ApiConfig,
    /// Model-specific settings
    pub model_settings: ModelSettings,
    /// Tool configuration
    pub tools: ToolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Base URL for the API
    pub base_url: Option<String>,
    /// API key environment variable name
    pub api_key_env: Option<String>,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Maximum retries
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    /// Default model name
    pub model: String,
    /// Temperature setting
    pub temperature: Option<f64>,
    /// Maximum tokens
    pub max_tokens: Option<u32>,
    /// Top-p setting
    pub top_p: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Enable tools by default
    pub enabled: bool,
    /// Available tool list
    pub available_tools: Vec<String>,
    /// Tool-specific settings
    pub tool_settings: HashMap<String, serde_json::Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            keybindings: KeybindingConfig::default(),
            blocks: BlockConfig::default(),
            defaults: DefaultsConfig::default(),
            providers: Self::default_providers(),
        }
    }
}

impl Config {
    /// Load configuration from file, creating default if it doesn't exist
    pub fn load<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config_path = config_path.as_ref();

        if config_path.exists() {
            let config_str = fs::read_to_string(config_path)
                .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

            toml::from_str(&config_str)
                .with_context(|| format!("Failed to parse config file: {:?}", config_path))
        } else {
            // Create default config and save it
            let config = Config::default();
            config.save(config_path)?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, config_path: P) -> Result<()> {
        let config_path = config_path.as_ref();

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let config_str = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(config_path, config_str)
            .with_context(|| format!("Failed to write config file: {:?}", config_path))
    }

    /// Get config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Failed to get config directory")?;
        Ok(config_dir.join("luts").join("config.toml"))
    }

    /// Get theme color as ratatui Color
    pub fn get_color(&self, color_name: &str) -> Color {
        match color_name.to_lowercase().as_str() {
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
            _ => {
                // Try to parse as hex color
                if color_name.starts_with('#') && color_name.len() == 7 {
                    if let Ok(rgb) = u32::from_str_radix(&color_name[1..], 16) {
                        let r = ((rgb >> 16) & 0xFF) as u8;
                        let g = ((rgb >> 8) & 0xFF) as u8;
                        let b = (rgb & 0xFF) as u8;
                        return Color::Rgb(r, g, b);
                    }
                }
                Color::White // Fallback
            }
        }
    }

    /// Create a context block from a template
    pub fn create_block_from_template(
        &self,
        template_id: &str,
        custom_content: Option<String>,
    ) -> Option<ContextBlock> {
        let template = self.blocks.templates.get(template_id)?;

        let content = custom_content.unwrap_or_else(|| template.content_template.clone());
        let block_id = format!("{}-{}", template_id, chrono::Utc::now().timestamp());

        let mut block = ContextBlock::new(
            block_id,
            template.name.clone(),
            template.block_type.clone(),
            content,
        )
        .with_priority(template.default_priority)
        .with_tags(template.tags.clone());

        // Add metadata
        for (key, value) in &template.metadata {
            block = block.with_metadata(key.clone(), value.clone());
        }

        Some(block)
    }

    /// Load a workflow preset
    pub fn load_workflow_preset(&self, preset_id: &str) -> Option<Vec<ContextBlock>> {
        let preset = self.blocks.workflows.get(preset_id)?;
        let mut blocks = Vec::new();

        for workflow_block in &preset.blocks {
            if let Some(mut block) = self.create_block_from_template(
                &workflow_block.template_id,
                workflow_block.content.clone(),
            ) {
                // Set dependencies
                block = block.with_dependencies(workflow_block.dependencies.clone());

                // Override priority if specified
                if let Some(priority) = workflow_block.priority {
                    block = block.with_priority(priority);
                }

                blocks.push(block);
            }
        }

        Some(blocks)
    }

    /// Get default providers
    fn default_providers() -> HashMap<String, ProviderConfig> {
        let mut providers = HashMap::new();

        providers.insert(
            "gemini-2.5-pro".to_string(),
            ProviderConfig {
                display_name: "Google Gemini 2.5 Pro".to_string(),
                api: ApiConfig {
                    base_url: None,
                    api_key_env: Some("GEMINI_API_KEY".to_string()),
                    timeout: 30,
                    max_retries: 3,
                },
                model_settings: ModelSettings {
                    model: "gemini-2.5-pro".to_string(),
                    temperature: Some(0.7),
                    max_tokens: Some(4096),
                    top_p: Some(0.9),
                },
                tools: ToolConfig {
                    enabled: true,
                    available_tools: vec![
                        "search".to_string(),
                        "calculator".to_string(),
                        "website".to_string(),
                    ],
                    tool_settings: HashMap::new(),
                },
            },
        );

        providers.insert(
            "deepseek-r1".to_string(),
            ProviderConfig {
                display_name: "DeepSeek R1".to_string(),
                api: ApiConfig {
                    base_url: Some("https://api.deepseek.com".to_string()),
                    api_key_env: Some("DEEPSEEK_API_KEY".to_string()),
                    timeout: 30,
                    max_retries: 3,
                },
                model_settings: ModelSettings {
                    model: "deepseek-r1".to_string(),
                    temperature: Some(0.8),
                    max_tokens: Some(4096),
                    top_p: Some(0.95),
                },
                tools: ToolConfig {
                    enabled: true,
                    available_tools: vec!["search".to_string(), "calculator".to_string()],
                    tool_settings: HashMap::new(),
                },
            },
        );

        providers
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            border_focused: "cyan".to_string(),
            border_unfocused: "gray".to_string(),
            text_primary: "white".to_string(),
            text_secondary: "gray".to_string(),
            text_accent: "cyan".to_string(),
            success: "green".to_string(),
            warning: "yellow".to_string(),
            error: "red".to_string(),
            info: "blue".to_string(),
            block_system: "blue".to_string(),
            block_user: "green".to_string(),
            block_memory: "yellow".to_string(),
            block_tool: "cyan".to_string(),
            block_response: "red".to_string(),
            block_example: "magenta".to_string(),
            block_dynamic: "gray".to_string(),
        }
    }
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            global: GlobalKeybindings {
                quit: vec!["Ctrl+q".to_string(), "Ctrl+c".to_string()],
                help: vec!["F1".to_string()],
                switch_to_agent_selection: vec!["Ctrl+1".to_string()],
                switch_to_conversation: vec!["Ctrl+2".to_string()],
                switch_to_block_mode: vec!["Ctrl+b".to_string()],
            },
            agent_selection: AgentSelectionKeybindings {
                select_agent: vec!["Enter".to_string(), " ".to_string()],
                move_up: vec!["Up".to_string(), "Ctrl+k".to_string()],
                move_down: vec!["Down".to_string(), "Ctrl+j".to_string()],
            },
            conversation: ConversationKeybindings {
                send_message: vec!["Enter".to_string()],
                switch_focus: vec!["Tab".to_string()],
                scroll_up: vec!["Up".to_string(), "Ctrl+k".to_string()],
                scroll_down: vec!["Down".to_string(), "Ctrl+j".to_string()],
                clear_input: vec!["Ctrl+l".to_string()],
            },
            block_mode: BlockModeKeybindings {
                toggle_block_status: vec!["Enter".to_string()],
                create_block: vec!["Ctrl+n".to_string()],
                process_next: vec!["Ctrl+p".to_string()],
                reset_workflow: vec!["Ctrl+r".to_string()],
                switch_panel: vec!["Tab".to_string()],
            },
        }
    }
}

impl Default for BlockConfig {
    fn default() -> Self {
        let mut templates = HashMap::new();

        // Legal expert templates
        templates.insert("legal-system".to_string(), BlockTemplate {
            name: "Legal Expert System".to_string(),
            description: "System prompt for legal document analysis".to_string(),
            block_type: BlockType::System,
            content_template: "You are a senior legal expert specializing in contract review and analysis. You provide clear, methodical analysis of legal documents and identify potential risks and issues.".to_string(),
            default_priority: 10,
            tags: vec!["legal".to_string(), "expert".to_string()],
            metadata: HashMap::new(),
        });

        templates.insert("clause-classifier".to_string(), BlockTemplate {
            name: "Clause Classification Tool".to_string(),
            description: "Tool for classifying contract clauses by risk level".to_string(),
            block_type: BlockType::Tool,
            content_template: "Analyze each clause in the provided contract and classify them as: standard, high-risk, unclear, or needs-review. Provide reasoning for each classification.".to_string(),
            default_priority: 8,
            tags: vec!["legal".to_string(), "classification".to_string()],
            metadata: HashMap::new(),
        });

        templates.insert(
            "legal-research".to_string(),
            BlockTemplate {
                name: "Legal Research Memory".to_string(),
                description: "Memory block for legal precedents and research".to_string(),
                block_type: BlockType::Memory,
                content_template:
                    "Legal precedents and case law relevant to the contract analysis.".to_string(),
                default_priority: 6,
                tags: vec!["legal".to_string(), "research".to_string()],
                metadata: HashMap::new(),
            },
        );

        // Code review templates
        templates.insert("code-reviewer".to_string(), BlockTemplate {
            name: "Code Reviewer System".to_string(),
            description: "System prompt for code review analysis".to_string(),
            block_type: BlockType::System,
            content_template: "You are an experienced software engineer conducting code reviews. Focus on code quality, security, performance, and maintainability.".to_string(),
            default_priority: 10,
            tags: vec!["code".to_string(), "review".to_string()],
            metadata: HashMap::new(),
        });

        templates.insert("security-scan".to_string(), BlockTemplate {
            name: "Security Analysis Tool".to_string(),
            description: "Tool for identifying security vulnerabilities".to_string(),
            block_type: BlockType::Tool,
            content_template: "Analyze the provided code for potential security vulnerabilities including SQL injection, XSS, authentication issues, and data validation problems.".to_string(),
            default_priority: 9,
            tags: vec!["security".to_string(), "analysis".to_string()],
            metadata: HashMap::new(),
        });

        // Writing assistant templates
        templates.insert("writing-assistant".to_string(), BlockTemplate {
            name: "Writing Assistant System".to_string(),
            description: "System prompt for writing assistance".to_string(),
            block_type: BlockType::System,
            content_template: "You are a professional writing assistant. Help improve clarity, structure, grammar, and style while maintaining the author's voice and intent.".to_string(),
            default_priority: 10,
            tags: vec!["writing".to_string(), "assistant".to_string()],
            metadata: HashMap::new(),
        });

        // Workflow presets
        let mut workflows = HashMap::new();

        workflows.insert(
            "legal-contract-review".to_string(),
            WorkflowPreset {
                name: "Legal Contract Review".to_string(),
                description: "Complete workflow for reviewing legal contracts".to_string(),
                blocks: vec![
                    WorkflowBlock {
                        template_id: "legal-system".to_string(),
                        content: None,
                        dependencies: vec![],
                        priority: Some(10),
                    },
                    WorkflowBlock {
                        template_id: "clause-classifier".to_string(),
                        content: None,
                        dependencies: vec!["legal-system".to_string()],
                        priority: Some(8),
                    },
                    WorkflowBlock {
                        template_id: "legal-research".to_string(),
                        content: None,
                        dependencies: vec!["clause-classifier".to_string()],
                        priority: Some(6),
                    },
                ],
            },
        );

        workflows.insert(
            "code-security-review".to_string(),
            WorkflowPreset {
                name: "Code Security Review".to_string(),
                description: "Security-focused code review workflow".to_string(),
                blocks: vec![
                    WorkflowBlock {
                        template_id: "code-reviewer".to_string(),
                        content: None,
                        dependencies: vec![],
                        priority: Some(10),
                    },
                    WorkflowBlock {
                        template_id: "security-scan".to_string(),
                        content: None,
                        dependencies: vec!["code-reviewer".to_string()],
                        priority: Some(9),
                    },
                ],
            },
        );

        Self {
            templates,
            workflows,
            defaults: BlockDefaults {
                auto_complete_dependencies: true,
                show_dependency_warnings: true,
                default_priority: 5,
                auto_scroll_to_ready: true,
            },
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            provider: "gemini-2.5-pro".to_string(),
            default_agent: None,
            auto_save_interval: 300, // 5 minutes
            max_history_length: 1000,
            enable_streaming: true,
            log_level: "info".to_string(),
        }
    }
}
