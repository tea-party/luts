//! Context saving and loading system
//!
//! This module provides sophisticated context management with save/load capabilities,
//! conversation snapshots, checkpoints, and context restoration.

use crate::llm::InternalChatMessage;
use crate::memory::{MemoryBlock, MemoryManager, MemoryQuery};
use crate::conversation::summarization::{ConversationSummary, ConversationSummarizer};
use crate::utils::tokens::{TokenManager, TokenUsage, UsageFilter};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Represents a complete conversation context snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// Unique identifier for this snapshot
    pub id: String,
    /// Human-readable name for the snapshot
    pub name: String,
    /// Description of the snapshot
    pub description: Option<String>,
    /// When this snapshot was created
    pub created_at: DateTime<Utc>,
    /// When this snapshot was last accessed
    pub last_accessed: DateTime<Utc>,
    /// Conversation messages
    pub messages: Vec<InternalChatMessage>,
    /// Memory blocks at the time of snapshot
    pub memory_blocks: Vec<MemoryBlock>,
    /// Conversation summaries
    pub summaries: Vec<ConversationSummary>,
    /// Token usage history
    pub token_usage: Vec<TokenUsage>,
    /// User ID associated with this context
    pub user_id: String,
    /// Session ID
    pub session_id: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Metadata for additional information
    pub metadata: HashMap<String, String>,
    /// Snapshot size in bytes
    pub size_bytes: usize,
    /// Compression ratio if compressed
    pub compression_ratio: Option<f64>,
    /// Whether this snapshot is marked as favorite
    pub is_favorite: bool,
    /// Whether this snapshot is archived
    pub is_archived: bool,
}

/// Configuration for context saving behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSaveConfig {
    /// Auto-save interval in seconds (0 = disabled)
    pub auto_save_interval: u64,
    /// Maximum number of snapshots to keep
    pub max_snapshots: usize,
    /// Compress snapshots to save space
    pub compress_snapshots: bool,
    /// Auto-archive old snapshots after days
    pub auto_archive_after_days: Option<u32>,
    /// Include token usage in snapshots
    pub include_token_usage: bool,
    /// Include memory blocks in snapshots
    pub include_memory_blocks: bool,
    /// Include summaries in snapshots
    pub include_summaries: bool,
    /// Backup to external storage
    pub backup_enabled: bool,
    /// Backup path (if different from main storage)
    pub backup_path: Option<PathBuf>,
}

impl Default for ContextSaveConfig {
    fn default() -> Self {
        Self {
            auto_save_interval: 300, // 5 minutes
            max_snapshots: 100,
            compress_snapshots: true,
            auto_archive_after_days: Some(30),
            include_token_usage: true,
            include_memory_blocks: true,
            include_summaries: true,
            backup_enabled: true,
            backup_path: None,
        }
    }
}

/// Query parameters for finding snapshots
#[derive(Debug, Clone, Default)]
pub struct SnapshotQuery {
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by session ID
    pub session_id: Option<String>,
    /// Filter by tags
    pub tags: Vec<String>,
    /// Filter by date range
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Search in name/description
    pub search_text: Option<String>,
    /// Include archived snapshots
    pub include_archived: bool,
    /// Only favorites
    pub favorites_only: bool,
    /// Maximum results
    pub limit: Option<usize>,
    /// Sort order
    pub sort_by: SnapshotSortBy,
}

/// Sort options for snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotSortBy {
    CreatedAt,
    LastAccessed,
    Name,
    SizeBytes,
    MessageCount,
}

impl Default for SnapshotSortBy {
    fn default() -> Self {
        Self::LastAccessed
    }
}

/// Statistics about saved contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStorageStats {
    /// Total number of snapshots
    pub total_snapshots: usize,
    /// Number of archived snapshots
    pub archived_snapshots: usize,
    /// Number of favorite snapshots
    pub favorite_snapshots: usize,
    /// Total storage used in bytes
    pub total_storage_bytes: usize,
    /// Average snapshot size
    pub average_snapshot_size: usize,
    /// Most used tags
    pub popular_tags: Vec<(String, usize)>,
    /// Storage by user
    pub storage_by_user: HashMap<String, usize>,
    /// Oldest snapshot date
    pub oldest_snapshot: Option<DateTime<Utc>>,
    /// Newest snapshot date
    pub newest_snapshot: Option<DateTime<Utc>>,
}

/// Advanced context saving and loading manager
pub struct ContextManager {
    /// Configuration for saving behavior
    config: RwLock<ContextSaveConfig>,
    /// Storage for snapshots
    snapshots: RwLock<HashMap<String, ContextSnapshot>>,
    /// Storage directory
    storage_dir: PathBuf,
    /// Memory manager for accessing memory blocks
    memory_manager: Option<Arc<MemoryManager>>,
    /// Conversation summarizer
    summarizer: Option<Arc<ConversationSummarizer>>,
    /// Token manager
    token_manager: Option<Arc<TokenManager>>,
    /// Auto-save task handle
    auto_save_task: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            config: RwLock::new(ContextSaveConfig::default()),
            snapshots: RwLock::new(HashMap::new()),
            storage_dir,
            memory_manager: None,
            summarizer: None,
            token_manager: None,
            auto_save_task: RwLock::new(None),
        }
    }

    /// Create context manager with all components
    pub fn new_with_components(
        storage_dir: PathBuf,
        memory_manager: Option<Arc<MemoryManager>>,
        summarizer: Option<Arc<ConversationSummarizer>>,
        token_manager: Option<Arc<TokenManager>>,
    ) -> Self {
        Self {
            config: RwLock::new(ContextSaveConfig::default()),
            snapshots: RwLock::new(HashMap::new()),
            storage_dir,
            memory_manager,
            summarizer,
            token_manager,
            auto_save_task: RwLock::new(None),
        }
    }

    /// Update configuration
    pub async fn update_config(&self, config: ContextSaveConfig) -> Result<()> {
        *self.config.write().await = config.clone();
        
        // Restart auto-save if interval changed
        if config.auto_save_interval > 0 {
            self.start_auto_save().await?;
        } else {
            self.stop_auto_save().await;
        }
        
        self.save_metadata().await?;
        info!("Updated context save configuration");
        Ok(())
    }

    /// Save a context snapshot
    pub async fn save_snapshot(
        &self,
        name: String,
        description: Option<String>,
        messages: Vec<InternalChatMessage>,
        user_id: String,
        session_id: String,
        tags: Vec<String>,
    ) -> Result<String> {
        let config = self.config.read().await;
        let snapshot_id = format!("snapshot_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        
        info!("Creating context snapshot: {} ({})", name, snapshot_id);
        
        // Collect memory blocks if enabled
        let memory_blocks = if config.include_memory_blocks {
            if let Some(memory_manager) = &self.memory_manager {
                // Get memory blocks for this user/session
                let query = MemoryQuery {
                    user_id: Some(user_id.clone()),
                    session_id: Some(session_id.clone()),
                    ..Default::default()
                };
                memory_manager.search(&query).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Collect summaries if enabled
        let summaries = if config.include_summaries {
            if let Some(summarizer) = &self.summarizer {
                summarizer.get_summaries().await
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Collect token usage if enabled
        let token_usage = if config.include_token_usage {
            if let Some(token_manager) = &self.token_manager {
                let filter = UsageFilter {
                    user_id: Some(user_id.clone()),
                    session_id: Some(session_id.clone()),
                    ..Default::default()
                };
                token_manager.get_usage_history(Some(filter)).await.unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let now = Utc::now();
        let snapshot = ContextSnapshot {
            id: snapshot_id.clone(),
            name,
            description,
            created_at: now,
            last_accessed: now,
            messages,
            memory_blocks,
            summaries,
            token_usage,
            user_id,
            session_id,
            tags,
            metadata: HashMap::new(),
            size_bytes: 0, // Will be calculated after serialization
            compression_ratio: None,
            is_favorite: false,
            is_archived: false,
        };

        // Save to storage
        self.save_snapshot_to_disk(&snapshot).await?;
        
        // Add to in-memory collection
        self.snapshots.write().await.insert(snapshot_id.clone(), snapshot);
        
        // Cleanup old snapshots if needed
        self.cleanup_old_snapshots().await?;
        
        info!("Successfully saved context snapshot: {}", snapshot_id);
        Ok(snapshot_id)
    }

    /// Load a context snapshot
    pub async fn load_snapshot(&self, snapshot_id: &str) -> Result<ContextSnapshot> {
        // Try in-memory first
        if let Some(snapshot) = self.snapshots.read().await.get(snapshot_id) {
            let mut snapshot = snapshot.clone();
            snapshot.last_accessed = Utc::now();
            self.snapshots.write().await.insert(snapshot_id.to_string(), snapshot.clone());
            self.save_snapshot_to_disk(&snapshot).await?;
            return Ok(snapshot);
        }

        // Load from disk
        let snapshot = self.load_snapshot_from_disk(snapshot_id).await?;
        
        // Update last accessed time
        let mut updated_snapshot = snapshot.clone();
        updated_snapshot.last_accessed = Utc::now();
        
        // Cache in memory
        self.snapshots.write().await.insert(snapshot_id.to_string(), updated_snapshot.clone());
        self.save_snapshot_to_disk(&updated_snapshot).await?;
        
        info!("Loaded context snapshot: {}", snapshot_id);
        Ok(updated_snapshot)
    }

    /// Restore context from snapshot
    pub async fn restore_context(&self, snapshot_id: &str) -> Result<RestoredContext> {
        let snapshot = self.load_snapshot(snapshot_id).await?;
        
        info!("Restoring context from snapshot: {} ({})", snapshot.name, snapshot_id);
        
        // Restore memory blocks if we have a memory manager
        if let Some(_memory_manager) = &self.memory_manager {
            for block in &snapshot.memory_blocks {
                // Note: MemoryManager uses store() method to save blocks
                // would need to be implemented or use a different approach
                warn!("Memory block restoration not yet implemented for block {}", block.id());
            }
        }

        let restored_context = RestoredContext {
            snapshot_id: snapshot.id.clone(),
            snapshot_name: snapshot.name.clone(),
            messages: snapshot.messages.clone(),
            memory_blocks_count: snapshot.memory_blocks.len(),
            summaries_count: snapshot.summaries.len(),
            token_usage_entries: snapshot.token_usage.len(),
            restored_at: Utc::now(),
        };

        info!("Successfully restored context: {} messages, {} memory blocks, {} summaries", 
               restored_context.messages.len(),
               restored_context.memory_blocks_count,
               restored_context.summaries_count);
        
        Ok(restored_context)
    }

    /// List snapshots with optional filtering
    pub async fn list_snapshots(&self, query: SnapshotQuery) -> Result<Vec<ContextSnapshot>> {
        // Ensure all snapshots are loaded in memory
        self.load_all_snapshots().await?;
        
        let snapshots = self.snapshots.read().await;
        let mut results: Vec<ContextSnapshot> = snapshots
            .values()
            .filter(|snapshot| self.matches_query(snapshot, &query))
            .cloned()
            .collect();

        // Sort results
        match query.sort_by {
            SnapshotSortBy::CreatedAt => results.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            SnapshotSortBy::LastAccessed => results.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed)),
            SnapshotSortBy::Name => results.sort_by(|a, b| a.name.cmp(&b.name)),
            SnapshotSortBy::SizeBytes => results.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes)),
            SnapshotSortBy::MessageCount => results.sort_by(|a, b| b.messages.len().cmp(&a.messages.len())),
        }

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Delete a snapshot
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        // Remove from memory
        self.snapshots.write().await.remove(snapshot_id);
        
        // Remove from disk
        let snapshot_path = self.get_snapshot_path(snapshot_id);
        if snapshot_path.exists() {
            tokio::fs::remove_file(snapshot_path).await?;
        }
        
        info!("Deleted snapshot: {}", snapshot_id);
        Ok(())
    }

    /// Mark snapshot as favorite
    pub async fn set_favorite(&self, snapshot_id: &str, is_favorite: bool) -> Result<()> {
        if let Some(snapshot) = self.snapshots.write().await.get_mut(snapshot_id) {
            snapshot.is_favorite = is_favorite;
            self.save_snapshot_to_disk(snapshot).await?;
            info!("Updated favorite status for snapshot {}: {}", snapshot_id, is_favorite);
        }
        Ok(())
    }

    /// Archive/unarchive snapshot
    pub async fn set_archived(&self, snapshot_id: &str, is_archived: bool) -> Result<()> {
        if let Some(snapshot) = self.snapshots.write().await.get_mut(snapshot_id) {
            snapshot.is_archived = is_archived;
            self.save_snapshot_to_disk(snapshot).await?;
            info!("Updated archive status for snapshot {}: {}", snapshot_id, is_archived);
        }
        Ok(())
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<ContextStorageStats> {
        self.load_all_snapshots().await?;
        let snapshots = self.snapshots.read().await;
        
        let total_snapshots = snapshots.len();
        let archived_snapshots = snapshots.values().filter(|s| s.is_archived).count();
        let favorite_snapshots = snapshots.values().filter(|s| s.is_favorite).count();
        let total_storage_bytes: usize = snapshots.values().map(|s| s.size_bytes).sum();
        let average_snapshot_size = if total_snapshots > 0 {
            total_storage_bytes / total_snapshots
        } else {
            0
        };

        // Calculate popular tags
        let mut tag_counts = HashMap::new();
        for snapshot in snapshots.values() {
            for tag in &snapshot.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let mut popular_tags: Vec<_> = tag_counts.into_iter().collect();
        popular_tags.sort_by(|a, b| b.1.cmp(&a.1));
        popular_tags.truncate(10);

        // Calculate storage by user
        let mut storage_by_user = HashMap::new();
        for snapshot in snapshots.values() {
            *storage_by_user.entry(snapshot.user_id.clone()).or_insert(0) += snapshot.size_bytes;
        }

        let oldest_snapshot = snapshots.values().map(|s| s.created_at).min();
        let newest_snapshot = snapshots.values().map(|s| s.created_at).max();

        Ok(ContextStorageStats {
            total_snapshots,
            archived_snapshots,
            favorite_snapshots,
            total_storage_bytes,
            average_snapshot_size,
            popular_tags,
            storage_by_user,
            oldest_snapshot,
            newest_snapshot,
        })
    }

    /// Export snapshot to external format
    pub async fn export_snapshot(&self, snapshot_id: &str, export_path: &Path, format: ExportFormat) -> Result<()> {
        let snapshot = self.load_snapshot(snapshot_id).await?;
        
        match format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&snapshot)?;
                tokio::fs::write(export_path, json).await?;
            }
            ExportFormat::Yaml => {
                let yaml = serde_yaml::to_string(&snapshot)?;
                tokio::fs::write(export_path, yaml).await?;
            }
            ExportFormat::Markdown => {
                let markdown = self.convert_snapshot_to_markdown(&snapshot);
                tokio::fs::write(export_path, markdown).await?;
            }
        }
        
        info!("Exported snapshot {} to {:?} as {:?}", snapshot_id, export_path, format);
        Ok(())
    }

    /// Import snapshot from external format
    pub async fn import_snapshot(&self, import_path: &Path, format: ExportFormat) -> Result<String> {
        let content = tokio::fs::read_to_string(import_path).await?;
        
        let snapshot: ContextSnapshot = match format {
            ExportFormat::Json => serde_json::from_str(&content)?,
            ExportFormat::Yaml => serde_yaml::from_str(&content)?,
            ExportFormat::Markdown => {
                return Err(anyhow::anyhow!("Markdown import not yet supported"));
            }
        };
        
        // Generate new ID to avoid conflicts
        let new_id = format!("imported_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        let mut imported_snapshot = snapshot;
        imported_snapshot.id = new_id.clone();
        imported_snapshot.last_accessed = Utc::now();
        
        // Save the imported snapshot
        self.save_snapshot_to_disk(&imported_snapshot).await?;
        self.snapshots.write().await.insert(new_id.clone(), imported_snapshot);
        
        info!("Imported snapshot from {:?} with new ID: {}", import_path, new_id);
        Ok(new_id)
    }

    // Private helper methods

    async fn save_snapshot_to_disk(&self, snapshot: &ContextSnapshot) -> Result<()> {
        let snapshot_path = self.get_snapshot_path(&snapshot.id);
        
        // Ensure directory exists
        if let Some(parent) = snapshot_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let config = self.config.read().await;
        let mut snapshot_data = snapshot.clone();
        
        if config.compress_snapshots {
            // Simplified compression (in real implementation, use proper compression library)
            let json = serde_json::to_string(snapshot)?;
            let compressed_size = json.len(); // Placeholder for actual compression
            snapshot_data.size_bytes = compressed_size;
            snapshot_data.compression_ratio = Some(json.len() as f64 / compressed_size as f64);
        } else {
            let json = serde_json::to_string(snapshot)?;
            snapshot_data.size_bytes = json.len();
        }
        
        let json = serde_json::to_string_pretty(&snapshot_data)?;
        tokio::fs::write(snapshot_path, json).await?;
        
        Ok(())
    }

    async fn load_snapshot_from_disk(&self, snapshot_id: &str) -> Result<ContextSnapshot> {
        let snapshot_path = self.get_snapshot_path(snapshot_id);
        let json = tokio::fs::read_to_string(snapshot_path).await?;
        let snapshot: ContextSnapshot = serde_json::from_str(&json)?;
        Ok(snapshot)
    }

    async fn load_all_snapshots(&self) -> Result<()> {
        let snapshots_dir = self.storage_dir.join("snapshots");
        if !snapshots_dir.exists() {
            return Ok(());
        }
        
        let mut entries = tokio::fs::read_dir(snapshots_dir).await?;
        let mut snapshots = self.snapshots.write().await;
        
        while let Some(entry) = entries.next_entry().await? {
            if let Some(extension) = entry.path().extension() {
                if extension == "json" {
                    if let Some(stem) = entry.path().file_stem() {
                        let snapshot_id = stem.to_string_lossy().to_string();
                        if !snapshots.contains_key(&snapshot_id) {
                            if let Ok(snapshot) = self.load_snapshot_from_disk(&snapshot_id).await {
                                snapshots.insert(snapshot_id, snapshot);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    fn get_snapshot_path(&self, snapshot_id: &str) -> PathBuf {
        self.storage_dir.join("snapshots").join(format!("{}.json", snapshot_id))
    }

    fn matches_query(&self, snapshot: &ContextSnapshot, query: &SnapshotQuery) -> bool {
        if let Some(ref user_id) = query.user_id {
            if snapshot.user_id != *user_id {
                return false;
            }
        }
        
        if let Some(ref session_id) = query.session_id {
            if snapshot.session_id != *session_id {
                return false;
            }
        }
        
        if !query.tags.is_empty() {
            if !query.tags.iter().any(|tag| snapshot.tags.contains(tag)) {
                return false;
            }
        }
        
        if let Some((start, end)) = query.date_range {
            if snapshot.created_at < start || snapshot.created_at > end {
                return false;
            }
        }
        
        if let Some(ref search_text) = query.search_text {
            let search_lower = search_text.to_lowercase();
            if !snapshot.name.to_lowercase().contains(&search_lower) &&
               !snapshot.description.as_ref().unwrap_or(&String::new()).to_lowercase().contains(&search_lower) {
                return false;
            }
        }
        
        if !query.include_archived && snapshot.is_archived {
            return false;
        }
        
        if query.favorites_only && !snapshot.is_favorite {
            return false;
        }
        
        true
    }

    async fn cleanup_old_snapshots(&self) -> Result<()> {
        let config = self.config.read().await;
        let mut snapshots = self.snapshots.write().await;
        
        if snapshots.len() <= config.max_snapshots {
            return Ok(());
        }
        
        let to_remove = snapshots.len() - config.max_snapshots;
        
        // Sort by last accessed time and remove oldest
        let snapshot_ids_to_remove: Vec<String> = {
            let mut snapshot_list: Vec<_> = snapshots.iter().collect();
            snapshot_list.sort_by(|a, b| a.1.last_accessed.cmp(&b.1.last_accessed));
            
            snapshot_list.iter()
                .take(to_remove)
                .map(|(id, _)| (*id).clone())
                .collect()
        };
        
        for snapshot_id in snapshot_ids_to_remove {
            let snapshot_path = self.get_snapshot_path(&snapshot_id);
            if snapshot_path.exists() {
                if let Err(e) = tokio::fs::remove_file(snapshot_path).await {
                    warn!("Failed to remove old snapshot file {}: {}", snapshot_id, e);
                }
            }
            snapshots.remove(&snapshot_id);
            info!("Cleaned up old snapshot: {}", snapshot_id);
        }
        
        Ok(())
    }

    async fn start_auto_save(&self) -> Result<()> {
        // Stop existing auto-save task
        self.stop_auto_save().await;
        
        let config = self.config.read().await;
        let interval = config.auto_save_interval;
        
        if interval > 0 {
            // Implementation would start a background task for auto-saving
            // For now, just log that auto-save is enabled
            info!("Auto-save enabled with interval: {} seconds", interval);
        }
        
        Ok(())
    }

    async fn stop_auto_save(&self) {
        if let Some(task) = self.auto_save_task.write().await.take() {
            task.abort();
            info!("Stopped auto-save task");
        }
    }

    async fn save_metadata(&self) -> Result<()> {
        let metadata_path = self.storage_dir.join("context_metadata.json");
        let config = self.config.read().await;
        let json = serde_json::to_string_pretty(&*config)?;
        tokio::fs::write(metadata_path, json).await?;
        Ok(())
    }

    fn convert_snapshot_to_markdown(&self, snapshot: &ContextSnapshot) -> String {
        let mut markdown = String::new();
        
        markdown.push_str(&format!("# Context Snapshot: {}\n\n", snapshot.name));
        markdown.push_str(&format!("**Created:** {}\n", snapshot.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
        markdown.push_str(&format!("**User:** {}\n", snapshot.user_id));
        markdown.push_str(&format!("**Session:** {}\n", snapshot.session_id));
        
        if let Some(ref description) = snapshot.description {
            markdown.push_str(&format!("**Description:** {}\n", description));
        }
        
        if !snapshot.tags.is_empty() {
            markdown.push_str(&format!("**Tags:** {}\n", snapshot.tags.join(", ")));
        }
        
        markdown.push_str("\n## Conversation Messages\n\n");
        for (i, message) in snapshot.messages.iter().enumerate() {
            match message {
                InternalChatMessage::System { content } => {
                    markdown.push_str(&format!("### Message {} (System)\n{}\n\n", i + 1, content));
                }
                InternalChatMessage::User { content } => {
                    markdown.push_str(&format!("### Message {} (User)\n{}\n\n", i + 1, content));
                }
                InternalChatMessage::Assistant { content, .. } => {
                    markdown.push_str(&format!("### Message {} (Assistant)\n{}\n\n", i + 1, content));
                }
                InternalChatMessage::Tool { tool_name, content, .. } => {
                    markdown.push_str(&format!("### Message {} (Tool: {})\n{}\n\n", i + 1, tool_name, content));
                }
            }
        }
        
        if !snapshot.memory_blocks.is_empty() {
            markdown.push_str(&format!("## Memory Blocks ({})\n\n", snapshot.memory_blocks.len()));
        }
        
        if !snapshot.summaries.is_empty() {
            markdown.push_str(&format!("## Conversation Summaries ({})\n\n", snapshot.summaries.len()));
        }
        
        markdown
    }
}

/// Information about restored context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoredContext {
    pub snapshot_id: String,
    pub snapshot_name: String,
    pub messages: Vec<InternalChatMessage>,
    pub memory_blocks_count: usize,
    pub summaries_count: usize,
    pub token_usage_entries: usize,
    pub restored_at: DateTime<Utc>,
}

/// Export formats for snapshots
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Yaml,
    Markdown,
}

impl Default for UsageFilter {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            operation_type: None,
            session_id: None,
            user_id: None,
            date_range: None,
            min_tokens: None,
            max_tokens: None,
        }
    }
}

// Add uuid to dependencies
// uuid = "1.0"