//! Conversation segment editing and deletion system
//!
//! This module provides comprehensive segment-level editing capabilities for conversations,
//! including message editing, deletion, reordering, and batch operations with undo/redo support.

use crate::llm::InternalChatMessage;
use luts_memory::MemoryManager;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Represents an editable conversation segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSegment {
    /// Unique segment ID
    pub id: String,
    /// Segment type
    pub segment_type: SegmentType,
    /// Segment content
    pub content: String,
    /// Original content (for undo)
    pub original_content: String,
    /// Message metadata
    pub metadata: SegmentMetadata,
    /// When this segment was created
    pub created_at: DateTime<Utc>,
    /// When this segment was last modified
    pub modified_at: Option<DateTime<Utc>>,
    /// Who created this segment
    pub author: String,
    /// Segment position in conversation
    pub position: usize,
    /// Whether this segment is read-only
    pub read_only: bool,
    /// Edit history
    pub edit_history: Vec<SegmentEdit>,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Custom properties
    pub properties: HashMap<String, String>,
}

/// Types of conversation segments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SegmentType {
    /// User message
    UserMessage,
    /// Assistant response
    AssistantMessage,
    /// System message
    SystemMessage,
    /// Tool call/response
    ToolMessage,
    /// Note/annotation
    Note,
    /// Code block
    CodeBlock,
    /// Image/media
    Media,
    /// Custom segment type
    Custom(String),
}

/// Segment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMetadata {
    /// Token count
    pub token_count: Option<u32>,
    /// Processing time (for AI responses)
    pub processing_time_ms: Option<u64>,
    /// Model used (for AI responses)
    pub model: Option<String>,
    /// Temperature setting
    pub temperature: Option<f32>,
    /// Confidence score
    pub confidence: Option<f64>,
    /// Whether segment is bookmarked
    pub is_bookmarked: bool,
    /// Whether segment is highlighted
    pub is_highlighted: bool,
    /// Importance level
    pub importance: ImportanceLevel,
    /// Custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Importance levels for segments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImportanceLevel {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for ImportanceLevel {
    fn default() -> Self {
        Self::Normal
    }
}

/// Represents an edit operation on a segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentEdit {
    /// Edit ID
    pub id: String,
    /// Type of edit
    pub edit_type: EditType,
    /// Content before edit
    pub before_content: String,
    /// Content after edit
    pub after_content: String,
    /// Who made the edit
    pub editor: String,
    /// When the edit was made
    pub timestamp: DateTime<Utc>,
    /// Edit reason/description
    pub reason: Option<String>,
    /// Whether this edit can be undone
    pub can_undo: bool,
}

/// Types of edit operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditType {
    /// Content modification
    ContentEdit,
    /// Metadata change
    MetadataEdit,
    /// Position change
    Reorder,
    /// Segment deletion
    Delete,
    /// Segment creation
    Create,
    /// Segment merge
    Merge,
    /// Segment split
    Split,
    /// Batch operation
    BatchEdit,
}

/// Batch edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEditOperation {
    /// Operation ID
    pub id: String,
    /// Individual edits in this batch
    pub edits: Vec<SegmentEdit>,
    /// Batch description
    pub description: String,
    /// When the batch was executed
    pub executed_at: DateTime<Utc>,
    /// Who executed the batch
    pub executor: String,
}

/// Edit validation result
#[derive(Debug, Clone)]
pub struct EditValidation {
    /// Whether the edit is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Suggested changes
    pub suggestions: Vec<String>,
}

/// Segment selection for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentSelection {
    /// Selected segment IDs
    pub segment_ids: Vec<String>,
    /// Selection criteria
    pub criteria: SelectionCriteria,
    /// Selection metadata
    pub metadata: SelectionMetadata,
}

/// Criteria for selecting segments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionCriteria {
    /// Filter by segment type
    pub segment_types: Option<Vec<SegmentType>>,
    /// Filter by author
    pub authors: Option<Vec<String>>,
    /// Filter by date range
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Filter by content pattern
    pub content_pattern: Option<String>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Filter by importance
    pub importance: Option<Vec<ImportanceLevel>>,
    /// Position range
    pub position_range: Option<(usize, usize)>,
}

/// Selection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionMetadata {
    /// Total segments selected
    pub count: usize,
    /// Selection timestamp
    pub selected_at: DateTime<Utc>,
    /// Who made the selection
    pub selector: String,
    /// Selection purpose
    pub purpose: Option<String>,
}

/// Undo/redo operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoRedoOperation {
    /// Operation ID
    pub id: String,
    /// Type of operation
    pub operation_type: UndoRedoType,
    /// Affected segment IDs
    pub affected_segments: Vec<String>,
    /// State before operation
    pub before_state: Vec<ConversationSegment>,
    /// State after operation
    pub after_state: Vec<ConversationSegment>,
    /// Operation timestamp
    pub timestamp: DateTime<Utc>,
    /// Description
    pub description: String,
}

/// Types of undo/redo operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UndoRedoType {
    Undo,
    Redo,
}

/// Configuration for segment editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentEditConfig {
    /// Maximum undo history size
    pub max_undo_history: usize,
    /// Enable automatic backup before edits
    pub auto_backup: bool,
    /// Backup retention days
    pub backup_retention_days: u32,
    /// Validate edits before applying
    pub validate_edits: bool,
    /// Enable edit notifications
    pub enable_notifications: bool,
    /// Maximum segment content length
    pub max_segment_length: usize,
    /// Allowed segment types for editing
    pub editable_segment_types: Vec<SegmentType>,
    /// Require edit reasons for certain operations
    pub require_edit_reasons: bool,
    /// Enable collaborative editing
    pub enable_collaboration: bool,
}

impl Default for SegmentEditConfig {
    fn default() -> Self {
        Self {
            max_undo_history: 50,
            auto_backup: true,
            backup_retention_days: 30,
            validate_edits: true,
            enable_notifications: true,
            max_segment_length: 50000, // 50KB max
            editable_segment_types: vec![
                SegmentType::UserMessage,
                SegmentType::AssistantMessage,
                SegmentType::Note,
                SegmentType::CodeBlock,
            ],
            require_edit_reasons: false,
            enable_collaboration: false,
        }
    }
}

/// Conversation segment editor
pub struct ConversationSegmentEditor {
    /// Current segments
    segments: RwLock<Vec<ConversationSegment>>,
    /// Undo history stack
    undo_stack: RwLock<VecDeque<UndoRedoOperation>>,
    /// Redo history stack
    redo_stack: RwLock<VecDeque<UndoRedoOperation>>,
    /// Configuration
    config: RwLock<SegmentEditConfig>,
    /// Memory manager for related blocks
    memory_manager: Option<Arc<MemoryManager>>,
    /// Edit listeners for notifications
    edit_listeners: RwLock<Vec<Box<dyn EditListener + Send + Sync>>>,
    /// Active selections
    #[allow(dead_code)]
    active_selections: RwLock<HashMap<String, SegmentSelection>>,
    /// Batch operations in progress
    #[allow(dead_code)]
    batch_operations: RwLock<HashMap<String, BatchEditOperation>>,
}

/// Trait for listening to edit events
pub trait EditListener {
    fn on_segment_edited(&self, segment_id: &str, edit: &SegmentEdit);
    fn on_segment_deleted(&self, segment_id: &str);
    fn on_segment_created(&self, segment: &ConversationSegment);
    fn on_batch_operation(&self, operation: &BatchEditOperation);
}

impl ConversationSegmentEditor {
    /// Create a new segment editor
    pub fn new() -> Self {
        Self {
            segments: RwLock::new(Vec::new()),
            undo_stack: RwLock::new(VecDeque::new()),
            redo_stack: RwLock::new(VecDeque::new()),
            config: RwLock::new(SegmentEditConfig::default()),
            memory_manager: None,
            edit_listeners: RwLock::new(Vec::new()),
            active_selections: RwLock::new(HashMap::new()),
            batch_operations: RwLock::new(HashMap::new()),
        }
    }

    /// Create editor with memory manager
    pub fn new_with_memory_manager(memory_manager: Arc<MemoryManager>) -> Self {
        let mut editor = Self::new();
        editor.memory_manager = Some(memory_manager);
        editor
    }

    /// Load conversation from messages
    pub async fn load_conversation(&self, messages: Vec<InternalChatMessage>) -> Result<()> {
        let mut segments = Vec::new();
        
        for (index, message) in messages.into_iter().enumerate() {
            let segment = self.message_to_segment(message, index).await?;
            segments.push(segment);
        }

        *self.segments.write().await = segments;
        
        // Clear undo/redo history when loading new conversation
        self.undo_stack.write().await.clear();
        self.redo_stack.write().await.clear();

        info!("Loaded conversation with {} segments", self.segments.read().await.len());
        Ok(())
    }

    /// Edit a segment's content
    pub async fn edit_segment_content(
        &self,
        segment_id: &str,
        new_content: String,
        editor: String,
        reason: Option<String>,
    ) -> Result<()> {
        let config = self.config.read().await;
        
        // Validate edit
        if config.validate_edits {
            let validation = self.validate_content_edit(segment_id, &new_content).await?;
            if !validation.is_valid {
                return Err(anyhow::anyhow!("Edit validation failed: {:?}", validation.errors));
            }
        }

        // Check edit permissions
        if !self.can_edit_segment(segment_id, &editor).await? {
            return Err(anyhow::anyhow!("User {} cannot edit segment {}", editor, segment_id));
        }

        drop(config);

        // Create backup if auto-backup is enabled
        let backup_state = if self.config.read().await.auto_backup {
            Some(self.segments.read().await.clone())
        } else {
            None
        };

        // Perform the edit
        let mut segments = self.segments.write().await;
        if let Some(segment) = segments.iter_mut().find(|s| s.id == segment_id) {
            let old_content = segment.content.clone();
            
            // Create edit record
            let edit = SegmentEdit {
                id: format!("edit_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string()),
                edit_type: EditType::ContentEdit,
                before_content: old_content,
                after_content: new_content.clone(),
                editor: editor.clone(),
                timestamp: Utc::now(),
                reason,
                can_undo: true,
            };

            // Apply the edit
            segment.content = new_content;
            segment.modified_at = Some(Utc::now());
            segment.edit_history.push(edit.clone());

            // Update metadata
            segment.metadata.token_count = Some(self.calculate_token_count(&segment.content));

            drop(segments);

            // Add to undo stack
            if let Some(backup) = backup_state {
                self.add_to_undo_stack(UndoRedoOperation {
                    id: edit.id.clone(),
                    operation_type: UndoRedoType::Undo,
                    affected_segments: vec![segment_id.to_string()],
                    before_state: backup,
                    after_state: self.segments.read().await.clone(),
                    timestamp: Utc::now(),
                    description: format!("Edit content of segment {}", segment_id),
                }).await;
            }

            // Notify listeners
            self.notify_segment_edited(segment_id, &edit).await;

            info!("Edited segment {} by {}", segment_id, editor);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Segment not found: {}", segment_id))
        }
    }

    /// Delete a segment
    pub async fn delete_segment(
        &self,
        segment_id: &str,
        editor: String,
        reason: Option<String>,
    ) -> Result<()> {
        // Check delete permissions
        if !self.can_delete_segment(segment_id, &editor).await? {
            return Err(anyhow::anyhow!("User {} cannot delete segment {}", editor, segment_id));
        }

        // Create backup
        let backup_state = self.segments.read().await.clone();

        // Perform deletion
        let mut segments = self.segments.write().await;
        if let Some(index) = segments.iter().position(|s| s.id == segment_id) {
            let deleted_segment = segments.remove(index);
            
            // Update positions of remaining segments
            for (i, segment) in segments.iter_mut().enumerate() {
                segment.position = i;
            }

            drop(segments);

            // Create edit record
            let edit = SegmentEdit {
                id: format!("delete_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string()),
                edit_type: EditType::Delete,
                before_content: deleted_segment.content.clone(),
                after_content: String::new(),
                editor: editor.clone(),
                timestamp: Utc::now(),
                reason,
                can_undo: true,
            };

            // Add to undo stack
            self.add_to_undo_stack(UndoRedoOperation {
                id: edit.id.clone(),
                operation_type: UndoRedoType::Undo,
                affected_segments: vec![segment_id.to_string()],
                before_state: backup_state,
                after_state: self.segments.read().await.clone(),
                timestamp: Utc::now(),
                description: format!("Delete segment {}", segment_id),
            }).await;

            // Notify listeners
            self.notify_segment_deleted(segment_id).await;

            info!("Deleted segment {} by {}", segment_id, editor);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Segment not found: {}", segment_id))
        }
    }

    /// Reorder segments
    pub async fn reorder_segments(
        &self,
        segment_ids: Vec<String>,
        editor: String,
    ) -> Result<()> {
        let backup_state = self.segments.read().await.clone();
        
        let mut segments = self.segments.write().await;
        let mut new_segments = Vec::with_capacity(segments.len());

        // Add segments in the new order
        for segment_id in &segment_ids {
            if let Some(index) = segments.iter().position(|s| s.id == *segment_id) {
                let mut segment = segments.remove(index);
                segment.position = new_segments.len();
                new_segments.push(segment);
            }
        }

        // Add any remaining segments that weren't in the reorder list
        for mut segment in segments.drain(..) {
            segment.position = new_segments.len();
            new_segments.push(segment);
        }

        *segments = new_segments;
        drop(segments);

        // Add to undo stack
        let edit_id = format!("reorder_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        self.add_to_undo_stack(UndoRedoOperation {
            id: edit_id,
            operation_type: UndoRedoType::Undo,
            affected_segments: segment_ids.clone(),
            before_state: backup_state,
            after_state: self.segments.read().await.clone(),
            timestamp: Utc::now(),
            description: "Reorder segments".to_string(),
        }).await;

        info!("Reordered {} segments by {}", segment_ids.len(), editor);
        Ok(())
    }

    /// Create a new segment
    pub async fn create_segment(
        &self,
        segment_type: SegmentType,
        content: String,
        author: String,
        position: Option<usize>,
    ) -> Result<String> {
        let segment_id = format!("segment_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        
        let mut segments = self.segments.write().await;
        let insert_position = position.unwrap_or(segments.len());

        let segment = ConversationSegment {
            id: segment_id.clone(),
            segment_type,
            content: content.clone(),
            original_content: content.clone(),
            metadata: SegmentMetadata {
                token_count: Some(self.calculate_token_count(&content)),
                processing_time_ms: None,
                model: None,
                temperature: None,
                confidence: None,
                is_bookmarked: false,
                is_highlighted: false,
                importance: ImportanceLevel::default(),
                custom: HashMap::new(),
            },
            created_at: Utc::now(),
            modified_at: None,
            author: author.clone(),
            position: insert_position,
            read_only: false,
            edit_history: Vec::new(),
            tags: Vec::new(),
            properties: HashMap::new(),
        };

        // Insert at the specified position
        if insert_position <= segments.len() {
            segments.insert(insert_position, segment.clone());
            
            // Update positions of segments after the insertion point
            for (i, seg) in segments.iter_mut().enumerate() {
                seg.position = i;
            }
        } else {
            return Err(anyhow::anyhow!("Invalid position: {}", insert_position));
        }

        drop(segments);

        // Notify listeners
        self.notify_segment_created(&segment).await;

        info!("Created new segment {} by {}", segment_id, author);
        Ok(segment_id)
    }

    /// Merge multiple segments into one
    pub async fn merge_segments(
        &self,
        segment_ids: Vec<String>,
        separator: Option<String>,
        editor: String,
    ) -> Result<String> {
        if segment_ids.len() < 2 {
            return Err(anyhow::anyhow!("Need at least 2 segments to merge"));
        }

        let backup_state = self.segments.read().await.clone();
        let separator = separator.unwrap_or_else(|| "\n\n".to_string());

        let mut segments = self.segments.write().await;
        let mut segments_to_merge = Vec::new();
        let mut indices_to_remove = Vec::new();

        // Collect segments to merge and their indices
        for segment_id in &segment_ids {
            if let Some(index) = segments.iter().position(|s| s.id == *segment_id) {
                segments_to_merge.push(segments[index].clone());
                indices_to_remove.push(index);
            }
        }

        if segments_to_merge.is_empty() {
            return Err(anyhow::anyhow!("No valid segments found to merge"));
        }

        // Sort by position to maintain order
        segments_to_merge.sort_by_key(|s| s.position);
        indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse for removal

        // Create merged content
        let merged_content = segments_to_merge
            .iter()
            .map(|s| s.content.clone())
            .collect::<Vec<_>>()
            .join(&separator);

        // Create the merged segment
        let merged_segment_id = format!("merged_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        let first_segment = &segments_to_merge[0];
        
        let merged_segment = ConversationSegment {
            id: merged_segment_id.clone(),
            segment_type: first_segment.segment_type.clone(),
            content: merged_content.clone(),
            original_content: merged_content.clone(),
            metadata: SegmentMetadata {
                token_count: Some(self.calculate_token_count(&merged_content)),
                processing_time_ms: None,
                model: first_segment.metadata.model.clone(),
                temperature: first_segment.metadata.temperature,
                confidence: None,
                is_bookmarked: false,
                is_highlighted: false,
                importance: ImportanceLevel::default(),
                custom: HashMap::new(),
            },
            created_at: Utc::now(),
            modified_at: None,
            author: editor.clone(),
            position: first_segment.position,
            read_only: false,
            edit_history: Vec::new(),
            tags: Vec::new(),
            properties: HashMap::new(),
        };

        // Remove the original segments (in reverse order)
        for index in indices_to_remove {
            segments.remove(index);
        }

        // Insert the merged segment at the position of the first original segment
        segments.insert(first_segment.position, merged_segment.clone());

        // Update positions
        for (i, segment) in segments.iter_mut().enumerate() {
            segment.position = i;
        }

        drop(segments);

        // Add to undo stack
        let edit_id = format!("merge_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        self.add_to_undo_stack(UndoRedoOperation {
            id: edit_id,
            operation_type: UndoRedoType::Undo,
            affected_segments: segment_ids,
            before_state: backup_state,
            after_state: self.segments.read().await.clone(),
            timestamp: Utc::now(),
            description: format!("Merge segments into {}", merged_segment_id),
        }).await;

        info!("Merged segments into {} by {}", merged_segment_id, editor);
        Ok(merged_segment_id)
    }

    /// Split a segment into multiple segments
    pub async fn split_segment(
        &self,
        segment_id: &str,
        split_points: Vec<usize>,
        editor: String,
    ) -> Result<Vec<String>> {
        let backup_state = self.segments.read().await.clone();

        let mut segments = self.segments.write().await;
        if let Some(index) = segments.iter().position(|s| s.id == segment_id) {
            let original_segment = segments.remove(index);
            let content = &original_segment.content;
            
            // Validate split points
            for &point in &split_points {
                if point >= content.len() {
                    return Err(anyhow::anyhow!("Split point {} is beyond content length {}", point, content.len()));
                }
            }

            // Sort split points
            let mut sorted_points = split_points;
            sorted_points.sort_unstable();
            sorted_points.dedup();

            // Create split segments
            let mut new_segment_ids = Vec::new();
            let mut last_point = 0;
            
            for (i, &split_point) in sorted_points.iter().enumerate() {
                let segment_content = content[last_point..split_point].to_string();
                if !segment_content.trim().is_empty() {
                    let new_segment_id = format!("split_{}_{}_{}", segment_id, i, uuid::Uuid::new_v4().to_string()[..8].to_string());
                    
                    let new_segment = ConversationSegment {
                        id: new_segment_id.clone(),
                        segment_type: original_segment.segment_type.clone(),
                        content: segment_content.clone(),
                        original_content: segment_content.clone(),
                        metadata: SegmentMetadata {
                            token_count: Some(self.calculate_token_count(&segment_content)),
                            processing_time_ms: None,
                            model: original_segment.metadata.model.clone(),
                            temperature: original_segment.metadata.temperature,
                            confidence: None,
                            is_bookmarked: false,
                            is_highlighted: false,
                            importance: original_segment.metadata.importance.clone(),
                            custom: HashMap::new(),
                        },
                        created_at: Utc::now(),
                        modified_at: None,
                        author: editor.clone(),
                        position: index + new_segment_ids.len(),
                        read_only: false,
                        edit_history: Vec::new(),
                        tags: original_segment.tags.clone(),
                        properties: HashMap::new(),
                    };

                    segments.insert(index + new_segment_ids.len(), new_segment);
                    new_segment_ids.push(new_segment_id);
                }
                last_point = split_point;
            }

            // Handle the last segment
            if last_point < content.len() {
                let segment_content = content[last_point..].to_string();
                if !segment_content.trim().is_empty() {
                    let new_segment_id = format!("split_{}_last_{}", segment_id, uuid::Uuid::new_v4().to_string()[..8].to_string());
                    
                    let new_segment = ConversationSegment {
                        id: new_segment_id.clone(),
                        segment_type: original_segment.segment_type.clone(),
                        content: segment_content.clone(),
                        original_content: segment_content.clone(),
                        metadata: SegmentMetadata {
                            token_count: Some(self.calculate_token_count(&segment_content)),
                            processing_time_ms: None,
                            model: original_segment.metadata.model.clone(),
                            temperature: original_segment.metadata.temperature,
                            confidence: None,
                            is_bookmarked: false,
                            is_highlighted: false,
                            importance: original_segment.metadata.importance.clone(),
                            custom: HashMap::new(),
                        },
                        created_at: Utc::now(),
                        modified_at: None,
                        author: editor.clone(),
                        position: index + new_segment_ids.len(),
                        read_only: false,
                        edit_history: Vec::new(),
                        tags: original_segment.tags.clone(),
                        properties: HashMap::new(),
                    };

                    segments.insert(index + new_segment_ids.len(), new_segment);
                    new_segment_ids.push(new_segment_id);
                }
            }

            // Update positions
            for (i, segment) in segments.iter_mut().enumerate() {
                segment.position = i;
            }

            drop(segments);

            // Add to undo stack
            let edit_id = format!("split_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
            self.add_to_undo_stack(UndoRedoOperation {
                id: edit_id,
                operation_type: UndoRedoType::Undo,
                affected_segments: vec![segment_id.to_string()],
                before_state: backup_state,
                after_state: self.segments.read().await.clone(),
                timestamp: Utc::now(),
                description: format!("Split segment {} into {} parts", segment_id, new_segment_ids.len()),
            }).await;

            info!("Split segment {} into {} parts by {}", segment_id, new_segment_ids.len(), editor);
            Ok(new_segment_ids)
        } else {
            Err(anyhow::anyhow!("Segment not found: {}", segment_id))
        }
    }

    /// Undo the last operation
    pub async fn undo(&self) -> Result<Option<UndoRedoOperation>> {
        let mut undo_stack = self.undo_stack.write().await;
        if let Some(operation) = undo_stack.pop_back() {
            // Restore the before state
            *self.segments.write().await = operation.before_state.clone();
            
            // Add to redo stack
            let mut redo_stack = self.redo_stack.write().await;
            let redo_operation = UndoRedoOperation {
                id: format!("redo_{}", operation.id),
                operation_type: UndoRedoType::Redo,
                affected_segments: operation.affected_segments.clone(),
                before_state: operation.after_state.clone(),
                after_state: operation.before_state.clone(),
                timestamp: Utc::now(),
                description: format!("Redo: {}", operation.description),
            };
            redo_stack.push_back(redo_operation);

            drop(undo_stack);
            drop(redo_stack);

            info!("Undid operation: {}", operation.description);
            Ok(Some(operation))
        } else {
            Ok(None)
        }
    }

    /// Redo the last undone operation
    pub async fn redo(&self) -> Result<Option<UndoRedoOperation>> {
        let mut redo_stack = self.redo_stack.write().await;
        if let Some(operation) = redo_stack.pop_back() {
            // Restore the before state (which is the "after" state of redo)
            *self.segments.write().await = operation.before_state.clone();
            
            // Add back to undo stack
            let mut undo_stack = self.undo_stack.write().await;
            let undo_operation = UndoRedoOperation {
                id: format!("undo_{}", operation.id),
                operation_type: UndoRedoType::Undo,
                affected_segments: operation.affected_segments.clone(),
                before_state: operation.after_state.clone(),
                after_state: operation.before_state.clone(),
                timestamp: Utc::now(),
                description: operation.description.replace("Redo: ", ""),
            };
            undo_stack.push_back(undo_operation);

            drop(undo_stack);
            drop(redo_stack);

            info!("Redid operation: {}", operation.description);
            Ok(Some(operation))
        } else {
            Ok(None)
        }
    }

    /// Get all segments
    pub async fn get_segments(&self) -> Vec<ConversationSegment> {
        self.segments.read().await.clone()
    }

    /// Get a specific segment
    pub async fn get_segment(&self, segment_id: &str) -> Option<ConversationSegment> {
        self.segments.read().await
            .iter()
            .find(|s| s.id == segment_id)
            .cloned()
    }

    /// Get undo history
    pub async fn get_undo_history(&self) -> Vec<UndoRedoOperation> {
        self.undo_stack.read().await.iter().cloned().collect()
    }

    /// Get redo history
    pub async fn get_redo_history(&self) -> Vec<UndoRedoOperation> {
        self.redo_stack.read().await.iter().cloned().collect()
    }

    // Private helper methods

    async fn message_to_segment(&self, message: InternalChatMessage, position: usize) -> Result<ConversationSegment> {
        let (segment_type, content, author) = match message {
            InternalChatMessage::User { content } => (SegmentType::UserMessage, content, "User".to_string()),
            InternalChatMessage::Assistant { content, .. } => (SegmentType::AssistantMessage, content, "Assistant".to_string()),
            InternalChatMessage::System { content } => (SegmentType::SystemMessage, content, "System".to_string()),
            InternalChatMessage::Tool { tool_name, content, .. } => (SegmentType::ToolMessage, content, format!("Tool({})", tool_name)),
        };

        Ok(ConversationSegment {
            id: format!("segment_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string()),
            segment_type,
            content: content.clone(),
            original_content: content.clone(),
            metadata: SegmentMetadata {
                token_count: Some(self.calculate_token_count(&content)),
                processing_time_ms: None,
                model: None,
                temperature: None,
                confidence: None,
                is_bookmarked: false,
                is_highlighted: false,
                importance: ImportanceLevel::default(),
                custom: HashMap::new(),
            },
            created_at: Utc::now(),
            modified_at: None,
            author,
            position,
            read_only: false,
            edit_history: Vec::new(),
            tags: Vec::new(),
            properties: HashMap::new(),
        })
    }

    async fn validate_content_edit(&self, segment_id: &str, new_content: &str) -> Result<EditValidation> {
        let mut validation = EditValidation {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        };

        let config = self.config.read().await;

        // Check content length
        if new_content.len() > config.max_segment_length {
            validation.is_valid = false;
            validation.errors.push(format!("Content exceeds maximum length of {} characters", config.max_segment_length));
        }

        // Check if segment exists and is editable
        if let Some(segment) = self.get_segment(segment_id).await {
            if segment.read_only {
                validation.is_valid = false;
                validation.errors.push("Segment is read-only".to_string());
            }

            if !config.editable_segment_types.contains(&segment.segment_type) {
                validation.is_valid = false;
                validation.errors.push(format!("Segment type {:?} is not editable", segment.segment_type));
            }
        } else {
            validation.is_valid = false;
            validation.errors.push("Segment not found".to_string());
        }

        // Check for empty content
        if new_content.trim().is_empty() {
            validation.warnings.push("Content is empty".to_string());
            validation.suggestions.push("Consider deleting the segment instead of leaving it empty".to_string());
        }

        Ok(validation)
    }

    async fn can_edit_segment(&self, _segment_id: &str, _editor: &str) -> Result<bool> {
        // Simplified permission check - in a real implementation, this would check user permissions
        Ok(true)
    }

    async fn can_delete_segment(&self, _segment_id: &str, _editor: &str) -> Result<bool> {
        // Simplified permission check - in a real implementation, this would check user permissions
        Ok(true)
    }

    fn calculate_token_count(&self, content: &str) -> u32 {
        // Simplified token counting - in a real implementation, use proper tokenization
        (content.split_whitespace().count() as f32 * 1.3) as u32
    }

    async fn add_to_undo_stack(&self, operation: UndoRedoOperation) {
        let mut undo_stack = self.undo_stack.write().await;
        let config = self.config.read().await;
        
        undo_stack.push_back(operation);
        
        // Limit undo history size
        while undo_stack.len() > config.max_undo_history {
            undo_stack.pop_front();
        }
        
        // Clear redo stack when new operation is added
        self.redo_stack.write().await.clear();
    }

    async fn notify_segment_edited(&self, segment_id: &str, edit: &SegmentEdit) {
        let listeners = self.edit_listeners.read().await;
        for listener in listeners.iter() {
            listener.on_segment_edited(segment_id, edit);
        }
    }

    async fn notify_segment_deleted(&self, segment_id: &str) {
        let listeners = self.edit_listeners.read().await;
        for listener in listeners.iter() {
            listener.on_segment_deleted(segment_id);
        }
    }

    async fn notify_segment_created(&self, segment: &ConversationSegment) {
        let listeners = self.edit_listeners.read().await;
        for listener in listeners.iter() {
            listener.on_segment_created(segment);
        }
    }
}