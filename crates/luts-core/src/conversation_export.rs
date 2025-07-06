//! Conversation export and import system
//!
//! This module provides comprehensive conversation export/import capabilities,
//! supporting multiple formats with metadata preservation and format conversion.

use crate::llm::InternalChatMessage;
use crate::memory::{MemoryBlock, MemoryManager, MemoryQuery};
use crate::summarization::ConversationSummary;
use crate::token_manager::{TokenUsage, TokenManager, UsageFilter};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Represents a complete conversation for export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableConversation {
    /// Conversation metadata
    pub metadata: ConversationMetadata,
    /// Conversation messages
    pub messages: Vec<ExportableMessage>,
    /// Associated memory blocks
    pub memory_blocks: Vec<MemoryBlock>,
    /// Conversation summaries
    pub summaries: Vec<ConversationSummary>,
    /// Token usage data
    pub token_usage: Vec<TokenUsage>,
    /// Export information
    pub export_info: ExportInfo,
}

/// Metadata about the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    /// Unique conversation ID
    pub id: String,
    /// Human-readable title
    pub title: String,
    /// Description of the conversation
    pub description: Option<String>,
    /// User ID
    pub user_id: String,
    /// Session ID
    pub session_id: String,
    /// Conversation start time
    pub started_at: DateTime<Utc>,
    /// Last message time
    pub last_message_at: DateTime<Utc>,
    /// Total message count
    pub message_count: usize,
    /// Conversation tags
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: HashMap<String, String>,
    /// Language of the conversation
    pub language: Option<String>,
    /// Conversation status (active, archived, etc.)
    pub status: ConversationStatus,
    /// Participants in the conversation
    pub participants: Vec<String>,
}

/// Status of a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationStatus {
    Active,
    Archived,
    Completed,
    Paused,
    Deleted,
}

/// Exportable message format with rich metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportableMessage {
    /// Message ID
    pub id: String,
    /// Message type
    pub message_type: MessageType,
    /// Message content
    pub content: String,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Message author/source
    pub author: String,
    /// Message metadata
    pub metadata: MessageMetadata,
    /// References to other messages
    pub references: Vec<String>,
    /// Message attachments
    pub attachments: Vec<MessageAttachment>,
}

/// Type of message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    User,
    Assistant,
    System,
    Tool,
    Error,
    Note,
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Token count for this message
    pub token_count: Option<u32>,
    /// Processing time in milliseconds
    pub processing_time_ms: Option<u64>,
    /// Model used for generation (if assistant message)
    pub model: Option<String>,
    /// Temperature used (if assistant message)
    pub temperature: Option<f32>,
    /// Confidence score
    pub confidence: Option<f64>,
    /// Message importance/priority
    pub importance: MessageImportance,
    /// Whether this message is bookmarked
    pub is_bookmarked: bool,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

/// Message importance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageImportance {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for MessageImportance {
    fn default() -> Self {
        Self::Normal
    }
}

/// Message attachments (files, images, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    /// Attachment ID
    pub id: String,
    /// File name
    pub filename: String,
    /// MIME type
    pub mime_type: String,
    /// File size in bytes
    pub size_bytes: usize,
    /// File content (base64 encoded for small files)
    pub content: Option<String>,
    /// File path (for large files)
    pub file_path: Option<PathBuf>,
    /// Attachment metadata
    pub metadata: HashMap<String, String>,
}

/// Export operation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportInfo {
    /// When this export was created
    pub exported_at: DateTime<Utc>,
    /// Export format used
    pub format: ExportFormat,
    /// Export version
    pub version: String,
    /// Exporter information
    pub exporter: String,
    /// Export settings used
    pub settings: ExportSettings,
    /// File size of the export
    pub file_size_bytes: Option<usize>,
    /// Compression used
    pub compression: Option<String>,
}

/// Available export formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Yaml,
    Csv,
    Markdown,
    Html,
    Txt,
    Xml,
    Jsonl, // JSON Lines
}

/// Export settings and options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSettings {
    /// Include message metadata
    pub include_metadata: bool,
    /// Include memory blocks
    pub include_memory_blocks: bool,
    /// Include summaries
    pub include_summaries: bool,
    /// Include token usage data
    pub include_token_usage: bool,
    /// Include attachments
    pub include_attachments: bool,
    /// Compress large text content
    pub compress_content: bool,
    /// Maximum file size for inline attachments
    pub max_inline_attachment_size: usize,
    /// Date range filter
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Message type filters
    pub message_type_filter: Option<Vec<MessageType>>,
    /// Include system messages
    pub include_system_messages: bool,
    /// Pretty print JSON/YAML
    pub pretty_print: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            include_metadata: true,
            include_memory_blocks: true,
            include_summaries: true,
            include_token_usage: true,
            include_attachments: true,
            compress_content: false,
            max_inline_attachment_size: 1024 * 1024, // 1MB
            date_range: None,
            message_type_filter: None,
            include_system_messages: true,
            pretty_print: true,
        }
    }
}

/// Import operation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    /// When this import was performed
    pub imported_at: DateTime<Utc>,
    /// Source file format
    pub source_format: ExportFormat,
    /// Import settings used
    pub settings: ImportSettings,
    /// Number of messages imported
    pub messages_imported: usize,
    /// Number of memory blocks imported
    pub memory_blocks_imported: usize,
    /// Import warnings/issues
    pub warnings: Vec<String>,
    /// Whether import was successful
    pub success: bool,
}

/// Import settings and options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSettings {
    /// Merge with existing conversation
    pub merge_mode: ImportMergeMode,
    /// Handle duplicate messages
    pub duplicate_handling: DuplicateHandling,
    /// Preserve original timestamps
    pub preserve_timestamps: bool,
    /// Preserve original IDs
    pub preserve_ids: bool,
    /// Import attachments
    pub import_attachments: bool,
    /// Validate data integrity
    pub validate_data: bool,
    /// Auto-assign new user/session IDs
    pub auto_assign_ids: bool,
}

/// How to merge imported conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportMergeMode {
    /// Replace existing conversation
    Replace,
    /// Append to existing conversation
    Append,
    /// Create new conversation
    CreateNew,
    /// Merge by timestamp
    MergeByTimestamp,
}

/// How to handle duplicate messages during import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DuplicateHandling {
    /// Skip duplicate messages
    Skip,
    /// Overwrite existing messages
    Overwrite,
    /// Create duplicate with new ID
    CreateNew,
    /// Merge metadata
    MergeMetadata,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self {
            merge_mode: ImportMergeMode::CreateNew,
            duplicate_handling: DuplicateHandling::Skip,
            preserve_timestamps: true,
            preserve_ids: false,
            import_attachments: true,
            validate_data: true,
            auto_assign_ids: true,
        }
    }
}

/// Conversation export/import manager
pub struct ConversationExporter {
    /// Storage directory for exports
    storage_dir: PathBuf,
    /// Memory manager for accessing memory blocks
    memory_manager: Option<Arc<MemoryManager>>,
    /// Token manager for accessing usage data
    token_manager: Option<Arc<TokenManager>>,
    /// Export templates and configurations
    templates: RwLock<HashMap<String, ExportSettings>>,
}

impl ConversationExporter {
    /// Create a new conversation exporter
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            memory_manager: None,
            token_manager: None,
            templates: RwLock::new(HashMap::new()),
        }
    }

    /// Create exporter with full component access
    pub fn new_with_components(
        storage_dir: PathBuf,
        memory_manager: Option<Arc<MemoryManager>>,
        token_manager: Option<Arc<TokenManager>>,
    ) -> Self {
        Self {
            storage_dir,
            memory_manager,
            token_manager,
            templates: RwLock::new(HashMap::new()),
        }
    }

    /// Export a conversation to the specified format
    pub async fn export_conversation(
        &self,
        messages: Vec<InternalChatMessage>,
        metadata: ConversationMetadata,
        output_path: &Path,
        format: ExportFormat,
        settings: ExportSettings,
    ) -> Result<ExportInfo> {
        info!("Exporting conversation {} to {:?} format", metadata.id, format);

        // Convert internal messages to exportable format
        let exportable_messages = self.convert_messages_to_exportable(messages, &settings).await?;

        // Collect additional data based on settings
        let memory_blocks = if settings.include_memory_blocks {
            self.collect_memory_blocks(&metadata.user_id, &metadata.session_id).await?
        } else {
            Vec::new()
        };

        let summaries = if settings.include_summaries {
            // Would integrate with summarization service
            Vec::new()
        } else {
            Vec::new()
        };

        let token_usage = if settings.include_token_usage {
            self.collect_token_usage(&metadata.user_id, &metadata.session_id).await?
        } else {
            Vec::new()
        };

        let export_info = ExportInfo {
            exported_at: Utc::now(),
            format: format.clone(),
            version: "1.0".to_string(),
            exporter: "LUTS ConversationExporter".to_string(),
            settings: settings.clone(),
            file_size_bytes: None,
            compression: None,
        };

        let exportable_conversation = ExportableConversation {
            metadata,
            messages: exportable_messages,
            memory_blocks,
            summaries,
            token_usage,
            export_info: export_info.clone(),
        };

        // Export to the specified format
        self.write_export(&exportable_conversation, output_path, &format, &settings).await?;

        info!("Successfully exported conversation to {:?}", output_path);
        Ok(export_info)
    }

    /// Import a conversation from file
    pub async fn import_conversation(
        &self,
        input_path: &Path,
        format: ExportFormat,
        settings: ImportSettings,
    ) -> Result<(ExportableConversation, ImportInfo)> {
        info!("Importing conversation from {:?} (format: {:?})", input_path, format);

        let content = tokio::fs::read_to_string(input_path).await?;
        let mut conversation = self.parse_import(&content, &format).await?;

        let mut warnings = Vec::new();
        let messages_imported = conversation.messages.len();
        let memory_blocks_imported = conversation.memory_blocks.len();

        // Apply import settings
        if settings.auto_assign_ids {
            conversation.metadata.id = format!("imported_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
            for (i, message) in conversation.messages.iter_mut().enumerate() {
                message.id = format!("msg_{}_{}", conversation.metadata.id, i);
            }
        }

        if !settings.preserve_timestamps {
            let now = Utc::now();
            conversation.metadata.started_at = now;
            conversation.metadata.last_message_at = now;
            for message in &mut conversation.messages {
                message.timestamp = now;
            }
        }

        // Validate data if requested
        if settings.validate_data {
            let validation_warnings = self.validate_conversation_data(&conversation).await;
            warnings.extend(validation_warnings);
        }

        let import_info = ImportInfo {
            imported_at: Utc::now(),
            source_format: format,
            settings,
            messages_imported,
            memory_blocks_imported,
            warnings,
            success: true,
        };

        info!("Successfully imported conversation: {} messages, {} memory blocks", 
               messages_imported, memory_blocks_imported);

        Ok((conversation, import_info))
    }

    /// Convert internal messages to exportable format
    async fn convert_messages_to_exportable(
        &self,
        messages: Vec<InternalChatMessage>,
        settings: &ExportSettings,
    ) -> Result<Vec<ExportableMessage>> {
        let mut exportable_messages = Vec::new();

        for (i, message) in messages.into_iter().enumerate() {
            let (message_type, content, author) = match message {
                InternalChatMessage::User { content } => (MessageType::User, content, "User".to_string()),
                InternalChatMessage::Assistant { content, .. } => (MessageType::Assistant, content, "Assistant".to_string()),
                InternalChatMessage::System { content } => {
                    if !settings.include_system_messages {
                        continue;
                    }
                    (MessageType::System, content, "System".to_string())
                }
                InternalChatMessage::Tool { tool_name, content, .. } => {
                    (MessageType::Tool, content, format!("Tool({})", tool_name))
                }
            };

            // Apply message type filter
            if let Some(ref filter) = settings.message_type_filter {
                if !filter.contains(&message_type) {
                    continue;
                }
            }

            let exportable_message = ExportableMessage {
                id: format!("msg_{}", i),
                message_type,
                content,
                timestamp: Utc::now(), // Would use actual timestamp in real implementation
                author,
                metadata: MessageMetadata {
                    token_count: None, // Would calculate if token manager available
                    processing_time_ms: None,
                    model: None,
                    temperature: None,
                    confidence: None,
                    importance: MessageImportance::default(),
                    is_bookmarked: false,
                    custom: HashMap::new(),
                },
                references: Vec::new(),
                attachments: Vec::new(),
            };

            exportable_messages.push(exportable_message);
        }

        Ok(exportable_messages)
    }

    /// Collect memory blocks for the conversation
    async fn collect_memory_blocks(&self, user_id: &str, session_id: &str) -> Result<Vec<MemoryBlock>> {
        if let Some(memory_manager) = &self.memory_manager {
            let query = MemoryQuery {
                user_id: Some(user_id.to_string()),
                session_id: Some(session_id.to_string()),
                ..Default::default()
            };
            memory_manager.search(&query).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Collect token usage data for the conversation
    async fn collect_token_usage(&self, user_id: &str, session_id: &str) -> Result<Vec<TokenUsage>> {
        if let Some(token_manager) = &self.token_manager {
            let filter = UsageFilter {
                user_id: Some(user_id.to_string()),
                session_id: Some(session_id.to_string()),
                ..Default::default()
            };
            token_manager.get_usage_history(Some(filter)).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Write export to file in the specified format
    async fn write_export(
        &self,
        conversation: &ExportableConversation,
        output_path: &Path,
        format: &ExportFormat,
        settings: &ExportSettings,
    ) -> Result<()> {
        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        match format {
            ExportFormat::Json => {
                let json = if settings.pretty_print {
                    serde_json::to_string_pretty(conversation)?
                } else {
                    serde_json::to_string(conversation)?
                };
                tokio::fs::write(output_path, json).await?;
            }
            ExportFormat::Yaml => {
                let yaml = serde_yaml::to_string(conversation)?;
                tokio::fs::write(output_path, yaml).await?;
            }
            ExportFormat::Csv => {
                let csv = self.convert_to_csv(conversation)?;
                tokio::fs::write(output_path, csv).await?;
            }
            ExportFormat::Markdown => {
                let markdown = self.convert_to_markdown(conversation);
                tokio::fs::write(output_path, markdown).await?;
            }
            ExportFormat::Html => {
                let html = self.convert_to_html(conversation);
                tokio::fs::write(output_path, html).await?;
            }
            ExportFormat::Txt => {
                let txt = self.convert_to_text(conversation);
                tokio::fs::write(output_path, txt).await?;
            }
            ExportFormat::Xml => {
                let xml = self.convert_to_xml(conversation)?;
                tokio::fs::write(output_path, xml).await?;
            }
            ExportFormat::Jsonl => {
                let jsonl = self.convert_to_jsonl(conversation)?;
                tokio::fs::write(output_path, jsonl).await?;
            }
        }

        Ok(())
    }

    /// Parse imported conversation from string content
    async fn parse_import(&self, content: &str, format: &ExportFormat) -> Result<ExportableConversation> {
        match format {
            ExportFormat::Json => {
                Ok(serde_json::from_str(content)?)
            }
            ExportFormat::Yaml => {
                Ok(serde_yaml::from_str(content)?)
            }
            ExportFormat::Jsonl => {
                self.parse_jsonl(content)
            }
            _ => {
                Err(anyhow::anyhow!("Import not yet supported for format: {:?}", format))
            }
        }
    }

    /// Convert conversation to CSV format
    fn convert_to_csv(&self, conversation: &ExportableConversation) -> Result<String> {
        let mut csv = String::new();
        csv.push_str("timestamp,author,type,content,token_count,importance\n");

        for message in &conversation.messages {
            let content_escaped = message.content.replace('"', "\"\"").replace('\n', " ").replace('\r', " ");
            let token_count = message.metadata.token_count.map_or("".to_string(), |c| c.to_string());
            let importance = format!("{:?}", message.metadata.importance);

            csv.push_str(&format!(
                "{},{},{:?},\"{}\",{},{}\n",
                message.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                message.author,
                message.message_type,
                content_escaped,
                token_count,
                importance
            ));
        }

        Ok(csv)
    }

    /// Convert conversation to Markdown format
    fn convert_to_markdown(&self, conversation: &ExportableConversation) -> String {
        let mut markdown = String::new();

        markdown.push_str(&format!("# {}\n\n", conversation.metadata.title));
        markdown.push_str(&format!("**Started:** {}\n", conversation.metadata.started_at.format("%Y-%m-%d %H:%M:%S UTC")));
        markdown.push_str(&format!("**User:** {}\n", conversation.metadata.user_id));
        markdown.push_str(&format!("**Session:** {}\n", conversation.metadata.session_id));
        markdown.push_str(&format!("**Messages:** {}\n\n", conversation.metadata.message_count));

        if let Some(ref description) = conversation.metadata.description {
            markdown.push_str(&format!("**Description:** {}\n\n", description));
        }

        if !conversation.metadata.tags.is_empty() {
            markdown.push_str(&format!("**Tags:** {}\n\n", conversation.metadata.tags.join(", ")));
        }

        markdown.push_str("## Conversation\n\n");

        for message in &conversation.messages {
            let author_emoji = match message.message_type {
                MessageType::User => "👤",
                MessageType::Assistant => "🤖",
                MessageType::System => "⚙️",
                MessageType::Tool => "🔧",
                MessageType::Error => "❌",
                MessageType::Note => "📝",
            };

            markdown.push_str(&format!(
                "### {} {} ({})\n\n{}\n\n",
                author_emoji,
                message.author,
                message.timestamp.format("%H:%M:%S"),
                message.content
            ));
        }

        if !conversation.memory_blocks.is_empty() {
            markdown.push_str(&format!("## Memory Blocks ({})\n\n", conversation.memory_blocks.len()));
        }

        if !conversation.summaries.is_empty() {
            markdown.push_str(&format!("## Summaries ({})\n\n", conversation.summaries.len()));
        }

        markdown
    }

    /// Convert conversation to HTML format
    fn convert_to_html(&self, conversation: &ExportableConversation) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!("<title>{}</title>\n", conversation.metadata.title));
        html.push_str("<style>\nbody { font-family: Arial, sans-serif; margin: 40px; }\n");
        html.push_str(".message { margin: 20px 0; padding: 10px; border-left: 3px solid #ccc; }\n");
        html.push_str(".user { border-left-color: #007bff; }\n");
        html.push_str(".assistant { border-left-color: #28a745; }\n");
        html.push_str(".system { border-left-color: #ffc107; }\n");
        html.push_str(".tool { border-left-color: #17a2b8; }\n");
        html.push_str("</style>\n</head>\n<body>\n");

        html.push_str(&format!("<h1>{}</h1>\n", conversation.metadata.title));
        html.push_str(&format!("<p><strong>Started:</strong> {}</p>\n", 
                              conversation.metadata.started_at.format("%Y-%m-%d %H:%M:%S UTC")));
        html.push_str(&format!("<p><strong>Messages:</strong> {}</p>\n", conversation.metadata.message_count));

        for message in &conversation.messages {
            let class = match message.message_type {
                MessageType::User => "user",
                MessageType::Assistant => "assistant",
                MessageType::System => "system",
                MessageType::Tool => "tool",
                _ => "message",
            };

            html.push_str(&format!(
                "<div class=\"message {}\">\n<strong>{}</strong> <small>({})</small>\n<p>{}</p>\n</div>\n",
                class,
                message.author,
                message.timestamp.format("%H:%M:%S"),
                message.content.replace('\n', "<br>")
            ));
        }

        html.push_str("</body>\n</html>");
        html
    }

    /// Convert conversation to plain text format
    fn convert_to_text(&self, conversation: &ExportableConversation) -> String {
        let mut text = String::new();

        text.push_str(&format!("{}\n", conversation.metadata.title));
        text.push_str(&format!("Started: {}\n", conversation.metadata.started_at.format("%Y-%m-%d %H:%M:%S UTC")));
        text.push_str(&format!("Messages: {}\n\n", conversation.metadata.message_count));

        text.push_str(&"=".repeat(80));
        text.push('\n');

        for message in &conversation.messages {
            text.push_str(&format!(
                "[{}] {}: {}\n\n",
                message.timestamp.format("%H:%M:%S"),
                message.author,
                message.content
            ));
        }

        text
    }

    /// Convert conversation to XML format
    fn convert_to_xml(&self, conversation: &ExportableConversation) -> Result<String> {
        let mut xml = String::new();

        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<conversation>\n");
        xml.push_str(&format!("  <title>{}</title>\n", conversation.metadata.title));
        xml.push_str(&format!("  <id>{}</id>\n", conversation.metadata.id));
        xml.push_str(&format!("  <user_id>{}</user_id>\n", conversation.metadata.user_id));
        xml.push_str(&format!("  <started_at>{}</started_at>\n", conversation.metadata.started_at.to_rfc3339()));

        xml.push_str("  <messages>\n");
        for message in &conversation.messages {
            xml.push_str("    <message>\n");
            xml.push_str(&format!("      <id>{}</id>\n", message.id));
            xml.push_str(&format!("      <type>{:?}</type>\n", message.message_type));
            xml.push_str(&format!("      <author>{}</author>\n", message.author));
            xml.push_str(&format!("      <timestamp>{}</timestamp>\n", message.timestamp.to_rfc3339()));
            xml.push_str(&format!("      <content><![CDATA[{}]]></content>\n", message.content));
            xml.push_str("    </message>\n");
        }
        xml.push_str("  </messages>\n");
        xml.push_str("</conversation>\n");

        Ok(xml)
    }

    /// Convert conversation to JSON Lines format
    fn convert_to_jsonl(&self, conversation: &ExportableConversation) -> Result<String> {
        let mut jsonl = String::new();

        for message in &conversation.messages {
            let json_line = serde_json::to_string(message)?;
            jsonl.push_str(&json_line);
            jsonl.push('\n');
        }

        Ok(jsonl)
    }

    /// Parse JSON Lines format
    fn parse_jsonl(&self, content: &str) -> Result<ExportableConversation> {
        let mut messages = Vec::new();

        for line in content.lines() {
            if !line.trim().is_empty() {
                let message: ExportableMessage = serde_json::from_str(line)?;
                messages.push(message);
            }
        }

        // Create minimal metadata for JSONL import
        let metadata = ConversationMetadata {
            id: format!("jsonl_import_{}", Utc::now().timestamp()),
            title: "Imported JSONL Conversation".to_string(),
            description: None,
            user_id: "imported_user".to_string(),
            session_id: "imported_session".to_string(),
            started_at: Utc::now(),
            last_message_at: Utc::now(),
            message_count: messages.len(),
            tags: Vec::new(),
            properties: HashMap::new(),
            language: None,
            status: ConversationStatus::Active,
            participants: Vec::new(),
        };

        let export_info = ExportInfo {
            exported_at: Utc::now(),
            format: ExportFormat::Jsonl,
            version: "1.0".to_string(),
            exporter: "LUTS ConversationExporter".to_string(),
            settings: ExportSettings::default(),
            file_size_bytes: None,
            compression: None,
        };

        Ok(ExportableConversation {
            metadata,
            messages,
            memory_blocks: Vec::new(),
            summaries: Vec::new(),
            token_usage: Vec::new(),
            export_info,
        })
    }

    /// Validate imported conversation data
    async fn validate_conversation_data(&self, conversation: &ExportableConversation) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check for empty conversation
        if conversation.messages.is_empty() {
            warnings.push("Conversation contains no messages".to_string());
        }

        // Check for message ID duplicates
        let mut message_ids = std::collections::HashSet::new();
        for message in &conversation.messages {
            if !message_ids.insert(&message.id) {
                warnings.push(format!("Duplicate message ID found: {}", message.id));
            }
        }

        // Check for invalid timestamps
        for message in &conversation.messages {
            if message.timestamp > Utc::now() {
                warnings.push(format!("Message {} has future timestamp", message.id));
            }
        }

        // Check metadata consistency
        if conversation.metadata.message_count != conversation.messages.len() {
            warnings.push(format!(
                "Metadata message count ({}) doesn't match actual count ({})",
                conversation.metadata.message_count,
                conversation.messages.len()
            ));
        }

        warnings
    }

    /// Save export template
    pub async fn save_export_template(&self, name: String, settings: ExportSettings) -> Result<()> {
        self.templates.write().await.insert(name.clone(), settings);
        info!("Saved export template: {}", name);
        Ok(())
    }

    /// Load export template
    pub async fn load_export_template(&self, name: &str) -> Option<ExportSettings> {
        self.templates.read().await.get(name).cloned()
    }

    /// List available export templates
    pub async fn list_export_templates(&self) -> Vec<String> {
        self.templates.read().await.keys().cloned().collect()
    }
}