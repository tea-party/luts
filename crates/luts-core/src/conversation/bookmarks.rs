//! Conversation bookmarks and favorites system
//!
//! This module provides comprehensive bookmark and favorites management for conversations,
//! including categorization, tagging, notes, and quick access functionality.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

/// A bookmark for a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationBookmark {
    /// Unique bookmark ID
    pub id: String,
    /// Conversation ID being bookmarked
    pub conversation_id: String,
    /// User who created the bookmark
    pub user_id: String,
    /// Bookmark title (optional, defaults to conversation title)
    pub title: Option<String>,
    /// User's note about this bookmark
    pub note: Option<String>,
    /// Bookmark category
    pub category: Option<String>,
    /// Tags for organization
    pub tags: Vec<String>,
    /// When this bookmark was created
    pub created_at: DateTime<Utc>,
    /// When this bookmark was last accessed
    pub last_accessed: Option<DateTime<Utc>>,
    /// How many times this bookmark has been accessed
    pub access_count: usize,
    /// Whether this is a favorite bookmark
    pub is_favorite: bool,
    /// Custom properties
    pub properties: HashMap<String, String>,
    /// Bookmark color/theme
    pub color: Option<BookmarkColor>,
    /// Priority level
    pub priority: BookmarkPriority,
    /// Whether to show in quick access
    pub quick_access: bool,
    /// Reminder settings
    pub reminder: Option<BookmarkReminder>,
}

/// Bookmark colors for visual organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BookmarkColor {
    Red,
    Blue,
    Green,
    Yellow,
    Purple,
    Orange,
    Pink,
    Gray,
    Custom(String), // Hex color code
}

/// Bookmark priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BookmarkPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for BookmarkPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Reminder settings for bookmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkReminder {
    /// Reminder date/time
    pub remind_at: DateTime<Utc>,
    /// Reminder message
    pub message: Option<String>,
    /// Whether the reminder is recurring
    pub recurring: Option<RecurringPattern>,
    /// Whether the reminder has been triggered
    pub triggered: bool,
}

/// Recurring reminder patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecurringPattern {
    Daily,
    Weekly,
    Monthly,
    Custom(chrono::Duration),
}

/// Bookmark collection/folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkCollection {
    /// Collection ID
    pub id: String,
    /// Collection name
    pub name: String,
    /// Collection description
    pub description: Option<String>,
    /// Collection color/theme
    pub color: Option<BookmarkColor>,
    /// Collection icon
    pub icon: Option<String>,
    /// Owner user ID
    pub owner_id: String,
    /// Whether this collection is shared
    pub is_shared: bool,
    /// Collection tags
    pub tags: Vec<String>,
    /// When this collection was created
    pub created_at: DateTime<Utc>,
    /// When this collection was last modified
    pub modified_at: DateTime<Utc>,
    /// Custom properties
    pub properties: HashMap<String, String>,
    /// Sort order for bookmarks in this collection
    pub sort_order: CollectionSortOrder,
}

/// Sort order options for collections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollectionSortOrder {
    CreatedAt,
    LastAccessed,
    AccessCount,
    Priority,
    Title,
    Custom,
}

impl Default for CollectionSortOrder {
    fn default() -> Self {
        Self::LastAccessed
    }
}

/// Bookmark search and filter criteria
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookmarkQuery {
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by conversation ID
    pub conversation_id: Option<String>,
    /// Filter by category
    pub category: Option<String>,
    /// Filter by tags (any of these tags)
    pub tags: Option<Vec<String>>,
    /// Filter by required tags (all of these tags)
    pub required_tags: Option<Vec<String>>,
    /// Text search in title and notes
    pub text_search: Option<String>,
    /// Filter by priority
    pub priority: Option<BookmarkPriority>,
    /// Filter by color
    pub color: Option<BookmarkColor>,
    /// Date range filter
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Only favorites
    pub favorites_only: bool,
    /// Only quick access bookmarks
    pub quick_access_only: bool,
    /// Only bookmarks with reminders
    pub with_reminders_only: bool,
    /// Collection ID filter
    pub collection_id: Option<String>,
    /// Sort order
    pub sort: BookmarkSortOrder,
    /// Maximum results
    pub limit: Option<usize>,
    /// Result offset
    pub offset: Option<usize>,
}

/// Sort order for bookmark queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BookmarkSortOrder {
    CreatedAt,
    LastAccessed,
    AccessCount,
    Priority,
    Title,
    Category,
}

impl Default for BookmarkSortOrder {
    fn default() -> Self {
        Self::LastAccessed
    }
}

/// Bookmark statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkStats {
    /// Total bookmarks
    pub total_bookmarks: usize,
    /// Total favorites
    pub total_favorites: usize,
    /// Total collections
    pub total_collections: usize,
    /// Bookmarks by category
    pub by_category: HashMap<String, usize>,
    /// Bookmarks by priority
    pub by_priority: HashMap<BookmarkPriority, usize>,
    /// Most used tags
    pub popular_tags: Vec<(String, usize)>,
    /// Most accessed bookmarks
    pub most_accessed: Vec<(String, usize)>,
    /// Recent bookmarks
    pub recent_bookmarks: Vec<String>,
    /// Upcoming reminders
    pub upcoming_reminders: Vec<(String, DateTime<Utc>)>,
}

/// Quick access bookmark entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAccessBookmark {
    /// Bookmark ID
    pub bookmark_id: String,
    /// Display title
    pub title: String,
    /// Conversation ID
    pub conversation_id: String,
    /// Category
    pub category: Option<String>,
    /// Color
    pub color: Option<BookmarkColor>,
    /// Priority
    pub priority: BookmarkPriority,
    /// Last access time
    pub last_accessed: Option<DateTime<Utc>>,
    /// Access count
    pub access_count: usize,
}

/// Bookmark import/export data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkExportData {
    /// Export metadata
    pub metadata: BookmarkExportMetadata,
    /// Bookmarks to export
    pub bookmarks: Vec<ConversationBookmark>,
    /// Collections to export
    pub collections: Vec<BookmarkCollection>,
    /// Collection membership data
    pub collection_memberships: HashMap<String, Vec<String>>, // collection_id -> bookmark_ids
}

/// Export metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkExportMetadata {
    /// Export version
    pub version: String,
    /// Export timestamp
    pub exported_at: DateTime<Utc>,
    /// Exporter information
    pub exporter: String,
    /// Total items exported
    pub total_items: usize,
}

/// Bookmark and favorites manager
pub struct BookmarkManager {
    /// Storage for bookmarks
    bookmarks: RwLock<HashMap<String, ConversationBookmark>>,
    /// Storage for collections
    collections: RwLock<HashMap<String, BookmarkCollection>>,
    /// Collection membership (collection_id -> bookmark_ids)
    collection_memberships: RwLock<HashMap<String, Vec<String>>>,
    /// Bookmark to collections mapping (bookmark_id -> collection_ids)
    bookmark_collections: RwLock<HashMap<String, Vec<String>>>,
    /// Configuration
    config: RwLock<BookmarkConfig>,
    /// Storage path for persistence
    storage_path: std::path::PathBuf,
}

/// Bookmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkConfig {
    /// Maximum bookmarks per user
    pub max_bookmarks_per_user: Option<usize>,
    /// Maximum collections per user
    pub max_collections_per_user: Option<usize>,
    /// Auto-cleanup old bookmarks after days
    pub auto_cleanup_after_days: Option<u32>,
    /// Default bookmark category
    pub default_category: Option<String>,
    /// Enable reminder notifications
    pub enable_reminders: bool,
    /// Quick access limit
    pub quick_access_limit: usize,
    /// Enable bookmark sharing
    pub enable_sharing: bool,
    /// Auto-add to quick access for high priority
    pub auto_quick_access_high_priority: bool,
}

impl Default for BookmarkConfig {
    fn default() -> Self {
        Self {
            max_bookmarks_per_user: Some(1000),
            max_collections_per_user: Some(50),
            auto_cleanup_after_days: None, // Don't auto-cleanup by default
            default_category: Some("General".to_string()),
            enable_reminders: true,
            quick_access_limit: 20,
            enable_sharing: false,
            auto_quick_access_high_priority: true,
        }
    }
}

impl BookmarkManager {
    /// Create a new bookmark manager
    pub fn new(storage_path: std::path::PathBuf) -> Self {
        Self {
            bookmarks: RwLock::new(HashMap::new()),
            collections: RwLock::new(HashMap::new()),
            collection_memberships: RwLock::new(HashMap::new()),
            bookmark_collections: RwLock::new(HashMap::new()),
            config: RwLock::new(BookmarkConfig::default()),
            storage_path,
        }
    }

    /// Create a bookmark for a conversation
    pub async fn create_bookmark(
        &self,
        conversation_id: String,
        user_id: String,
        title: Option<String>,
        note: Option<String>,
        category: Option<String>,
        tags: Vec<String>,
        priority: Option<BookmarkPriority>,
    ) -> Result<String> {
        let config = self.config.read().await;
        
        // Check user bookmark limit
        if let Some(max_bookmarks) = config.max_bookmarks_per_user {
            let user_bookmarks = self.count_user_bookmarks(&user_id).await;
            if user_bookmarks >= max_bookmarks {
                return Err(anyhow::anyhow!("User has reached maximum bookmark limit"));
            }
        }

        let bookmark_id = format!("bookmark_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        let priority = priority.unwrap_or_default();
        
        let bookmark = ConversationBookmark {
            id: bookmark_id.clone(),
            conversation_id,
            user_id,
            title,
            note,
            category: category.or_else(|| config.default_category.clone()),
            tags,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            is_favorite: false,
            properties: HashMap::new(),
            color: None,
            priority: priority.clone(),
            quick_access: config.auto_quick_access_high_priority && priority >= BookmarkPriority::High,
            reminder: None,
        };

        self.bookmarks.write().await.insert(bookmark_id.clone(), bookmark);
        self.save_to_storage().await?;

        info!("Created bookmark: {}", bookmark_id);
        Ok(bookmark_id)
    }

    /// Update a bookmark
    pub async fn update_bookmark(
        &self,
        bookmark_id: &str,
        updates: BookmarkUpdates,
    ) -> Result<()> {
        let mut bookmarks = self.bookmarks.write().await;
        
        if let Some(bookmark) = bookmarks.get_mut(bookmark_id) {
            if let Some(title) = updates.title {
                bookmark.title = Some(title);
            }
            if let Some(note) = updates.note {
                bookmark.note = Some(note);
            }
            if let Some(category) = updates.category {
                bookmark.category = Some(category);
            }
            if let Some(tags) = updates.tags {
                bookmark.tags = tags;
            }
            if let Some(priority) = updates.priority {
                bookmark.priority = priority;
            }
            if let Some(color) = updates.color {
                bookmark.color = Some(color);
            }
            if let Some(quick_access) = updates.quick_access {
                bookmark.quick_access = quick_access;
            }
            if let Some(reminder) = updates.reminder {
                bookmark.reminder = Some(reminder);
            }
            if let Some(properties) = updates.properties {
                bookmark.properties.extend(properties);
            }

            drop(bookmarks);
            self.save_to_storage().await?;
            info!("Updated bookmark: {}", bookmark_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Bookmark not found: {}", bookmark_id))
        }
    }

    /// Toggle favorite status of a bookmark
    pub async fn toggle_favorite(&self, bookmark_id: &str) -> Result<bool> {
        let mut bookmarks = self.bookmarks.write().await;
        
        if let Some(bookmark) = bookmarks.get_mut(bookmark_id) {
            bookmark.is_favorite = !bookmark.is_favorite;
            let is_favorite = bookmark.is_favorite;
            
            drop(bookmarks);
            self.save_to_storage().await?;
            info!("Toggled favorite for bookmark {}: {}", bookmark_id, is_favorite);
            Ok(is_favorite)
        } else {
            Err(anyhow::anyhow!("Bookmark not found: {}", bookmark_id))
        }
    }

    /// Access a bookmark (updates access count and time)
    pub async fn access_bookmark(&self, bookmark_id: &str) -> Result<ConversationBookmark> {
        let mut bookmarks = self.bookmarks.write().await;
        
        if let Some(bookmark) = bookmarks.get_mut(bookmark_id) {
            bookmark.last_accessed = Some(Utc::now());
            bookmark.access_count += 1;
            let bookmark_clone = bookmark.clone();
            
            drop(bookmarks);
            self.save_to_storage().await?;
            Ok(bookmark_clone)
        } else {
            Err(anyhow::anyhow!("Bookmark not found: {}", bookmark_id))
        }
    }

    /// Delete a bookmark
    pub async fn delete_bookmark(&self, bookmark_id: &str) -> Result<()> {
        let mut bookmarks = self.bookmarks.write().await;
        let mut bookmark_collections = self.bookmark_collections.write().await;
        let mut collection_memberships = self.collection_memberships.write().await;

        if bookmarks.remove(bookmark_id).is_some() {
            // Remove from collections
            if let Some(collection_ids) = bookmark_collections.remove(bookmark_id) {
                for collection_id in collection_ids {
                    if let Some(bookmark_ids) = collection_memberships.get_mut(&collection_id) {
                        bookmark_ids.retain(|id| id != bookmark_id);
                    }
                }
            }

            drop(bookmarks);
            drop(bookmark_collections);
            drop(collection_memberships);
            self.save_to_storage().await?;
            info!("Deleted bookmark: {}", bookmark_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Bookmark not found: {}", bookmark_id))
        }
    }

    /// Search bookmarks
    pub async fn search_bookmarks(&self, query: BookmarkQuery) -> Result<Vec<ConversationBookmark>> {
        let bookmarks = self.bookmarks.read().await;
        let mut results: Vec<ConversationBookmark> = bookmarks
            .values()
            .filter(|bookmark| self.matches_query(bookmark, &query))
            .cloned()
            .collect();

        // Sort results
        results.sort_by(|a, b| {
            match query.sort {
                BookmarkSortOrder::CreatedAt => b.created_at.cmp(&a.created_at),
                BookmarkSortOrder::LastAccessed => {
                    match (&b.last_accessed, &a.last_accessed) {
                        (Some(b_time), Some(a_time)) => b_time.cmp(a_time),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => b.created_at.cmp(&a.created_at),
                    }
                }
                BookmarkSortOrder::AccessCount => b.access_count.cmp(&a.access_count),
                BookmarkSortOrder::Priority => b.priority.cmp(&a.priority),
                BookmarkSortOrder::Title => {
                    let a_title = a.title.as_deref().unwrap_or("");
                    let b_title = b.title.as_deref().unwrap_or("");
                    a_title.cmp(b_title)
                }
                BookmarkSortOrder::Category => {
                    let a_cat = a.category.as_deref().unwrap_or("");
                    let b_cat = b.category.as_deref().unwrap_or("");
                    a_cat.cmp(b_cat)
                }
            }
        });

        // Apply pagination
        if let Some(offset) = query.offset {
            if offset < results.len() {
                results.drain(0..offset);
            } else {
                results.clear();
            }
        }
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Get quick access bookmarks
    pub async fn get_quick_access_bookmarks(&self, user_id: &str) -> Result<Vec<QuickAccessBookmark>> {
        let config = self.config.read().await;
        let limit = config.quick_access_limit;
        drop(config);

        let query = BookmarkQuery {
            user_id: Some(user_id.to_string()),
            quick_access_only: true,
            sort: BookmarkSortOrder::LastAccessed,
            limit: Some(limit),
            ..Default::default()
        };

        let bookmarks = self.search_bookmarks(query).await?;
        let quick_access: Vec<QuickAccessBookmark> = bookmarks
            .into_iter()
            .map(|bookmark| QuickAccessBookmark {
                bookmark_id: bookmark.id,
                title: bookmark.title.unwrap_or_else(|| "Untitled".to_string()),
                conversation_id: bookmark.conversation_id,
                category: bookmark.category,
                color: bookmark.color,
                priority: bookmark.priority,
                last_accessed: bookmark.last_accessed,
                access_count: bookmark.access_count,
            })
            .collect();

        Ok(quick_access)
    }

    /// Create a collection
    pub async fn create_collection(
        &self,
        name: String,
        description: Option<String>,
        owner_id: String,
        color: Option<BookmarkColor>,
        tags: Vec<String>,
    ) -> Result<String> {
        let config = self.config.read().await;
        
        // Check user collection limit
        if let Some(max_collections) = config.max_collections_per_user {
            let user_collections = self.count_user_collections(&owner_id).await;
            if user_collections >= max_collections {
                return Err(anyhow::anyhow!("User has reached maximum collection limit"));
            }
        }

        let collection_id = format!("collection_{}_{}", Utc::now().timestamp(), uuid::Uuid::new_v4().to_string()[..8].to_string());
        
        let collection = BookmarkCollection {
            id: collection_id.clone(),
            name,
            description,
            color,
            icon: None,
            owner_id,
            is_shared: false,
            tags,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            properties: HashMap::new(),
            sort_order: CollectionSortOrder::default(),
        };

        self.collections.write().await.insert(collection_id.clone(), collection);
        self.save_to_storage().await?;

        info!("Created collection: {}", collection_id);
        Ok(collection_id)
    }

    /// Add bookmark to collection
    pub async fn add_to_collection(&self, bookmark_id: &str, collection_id: &str) -> Result<()> {
        // Verify bookmark and collection exist
        let bookmarks = self.bookmarks.read().await;
        let collections = self.collections.read().await;
        
        if !bookmarks.contains_key(bookmark_id) {
            return Err(anyhow::anyhow!("Bookmark not found: {}", bookmark_id));
        }
        if !collections.contains_key(collection_id) {
            return Err(anyhow::anyhow!("Collection not found: {}", collection_id));
        }
        
        drop(bookmarks);
        drop(collections);

        // Add to collection membership
        let mut collection_memberships = self.collection_memberships.write().await;
        let mut bookmark_collections = self.bookmark_collections.write().await;

        collection_memberships
            .entry(collection_id.to_string())
            .or_insert_with(Vec::new)
            .push(bookmark_id.to_string());

        bookmark_collections
            .entry(bookmark_id.to_string())
            .or_insert_with(Vec::new)
            .push(collection_id.to_string());

        drop(collection_memberships);
        drop(bookmark_collections);
        self.save_to_storage().await?;

        info!("Added bookmark {} to collection {}", bookmark_id, collection_id);
        Ok(())
    }

    /// Get bookmark statistics
    pub async fn get_stats(&self, user_id: Option<&str>) -> Result<BookmarkStats> {
        let bookmarks = self.bookmarks.read().await;
        let collections = self.collections.read().await;

        // Filter by user if specified
        let user_bookmarks: Vec<&ConversationBookmark> = if let Some(user_id) = user_id {
            bookmarks.values().filter(|b| b.user_id == user_id).collect()
        } else {
            bookmarks.values().collect()
        };

        let total_bookmarks = user_bookmarks.len();
        let total_favorites = user_bookmarks.iter().filter(|b| b.is_favorite).count();
        let total_collections = if let Some(user_id) = user_id {
            collections.values().filter(|c| c.owner_id == user_id).count()
        } else {
            collections.len()
        };

        // Category distribution
        let mut by_category = HashMap::new();
        for bookmark in &user_bookmarks {
            let category = bookmark.category.as_deref().unwrap_or("Uncategorized");
            *by_category.entry(category.to_string()).or_insert(0) += 1;
        }

        // Priority distribution
        let mut by_priority = HashMap::new();
        for bookmark in &user_bookmarks {
            *by_priority.entry(bookmark.priority.clone()).or_insert(0) += 1;
        }

        // Popular tags
        let mut tag_counts = HashMap::new();
        for bookmark in &user_bookmarks {
            for tag in &bookmark.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let mut popular_tags: Vec<_> = tag_counts.into_iter().collect();
        popular_tags.sort_by(|a, b| b.1.cmp(&a.1));
        popular_tags.truncate(10);

        // Most accessed
        let mut bookmark_access: Vec<_> = user_bookmarks
            .iter()
            .map(|b| (b.id.clone(), b.access_count))
            .collect();
        bookmark_access.sort_by(|a, b| b.1.cmp(&a.1));
        bookmark_access.truncate(10);

        // Recent bookmarks
        let mut recent: Vec<_> = user_bookmarks
            .iter()
            .map(|b| (b.id.clone(), b.created_at))
            .collect();
        recent.sort_by(|a, b| b.1.cmp(&a.1));
        let recent_bookmarks: Vec<String> = recent.into_iter().take(10).map(|(id, _)| id).collect();

        // Upcoming reminders
        let now = Utc::now();
        let mut upcoming_reminders: Vec<_> = user_bookmarks
            .iter()
            .filter_map(|b| {
                b.reminder.as_ref().and_then(|r| {
                    if !r.triggered && r.remind_at > now {
                        Some((b.id.clone(), r.remind_at))
                    } else {
                        None
                    }
                })
            })
            .collect();
        upcoming_reminders.sort_by(|a, b| a.1.cmp(&b.1));
        upcoming_reminders.truncate(10);

        Ok(BookmarkStats {
            total_bookmarks,
            total_favorites,
            total_collections,
            by_category,
            by_priority,
            popular_tags,
            most_accessed: bookmark_access,
            recent_bookmarks,
            upcoming_reminders,
        })
    }

    /// Export bookmarks
    pub async fn export_bookmarks(&self, user_id: Option<&str>) -> Result<BookmarkExportData> {
        let bookmarks = self.bookmarks.read().await;
        let collections = self.collections.read().await;
        let collection_memberships = self.collection_memberships.read().await;

        let exported_bookmarks: Vec<ConversationBookmark> = if let Some(user_id) = user_id {
            bookmarks.values().filter(|b| b.user_id == user_id).cloned().collect()
        } else {
            bookmarks.values().cloned().collect()
        };

        let exported_collections: Vec<BookmarkCollection> = if let Some(user_id) = user_id {
            collections.values().filter(|c| c.owner_id == user_id).cloned().collect()
        } else {
            collections.values().cloned().collect()
        };

        let total_items = exported_bookmarks.len() + exported_collections.len();

        let export_data = BookmarkExportData {
            metadata: BookmarkExportMetadata {
                version: "1.0".to_string(),
                exported_at: Utc::now(),
                exporter: "LUTS BookmarkManager".to_string(),
                total_items,
            },
            bookmarks: exported_bookmarks,
            collections: exported_collections,
            collection_memberships: collection_memberships.clone(),
        };

        info!("Exported {} bookmarks and {} collections", 
               export_data.bookmarks.len(), export_data.collections.len());
        
        Ok(export_data)
    }

    // Private helper methods

    fn matches_query(&self, bookmark: &ConversationBookmark, query: &BookmarkQuery) -> bool {
        if let Some(ref user_id) = query.user_id {
            if bookmark.user_id != *user_id {
                return false;
            }
        }

        if let Some(ref conversation_id) = query.conversation_id {
            if bookmark.conversation_id != *conversation_id {
                return false;
            }
        }

        if let Some(ref category) = query.category {
            if bookmark.category.as_ref() != Some(category) {
                return false;
            }
        }

        if let Some(ref tags) = query.tags {
            if !tags.iter().any(|tag| bookmark.tags.contains(tag)) {
                return false;
            }
        }

        if let Some(ref required_tags) = query.required_tags {
            if !required_tags.iter().all(|tag| bookmark.tags.contains(tag)) {
                return false;
            }
        }

        if let Some(ref text_search) = query.text_search {
            let search_lower = text_search.to_lowercase();
            let title_match = bookmark.title.as_ref()
                .map_or(false, |t| t.to_lowercase().contains(&search_lower));
            let note_match = bookmark.note.as_ref()
                .map_or(false, |n| n.to_lowercase().contains(&search_lower));
            
            if !title_match && !note_match {
                return false;
            }
        }

        if let Some(ref priority) = query.priority {
            if bookmark.priority != *priority {
                return false;
            }
        }

        if query.favorites_only && !bookmark.is_favorite {
            return false;
        }

        if query.quick_access_only && !bookmark.quick_access {
            return false;
        }

        if query.with_reminders_only && bookmark.reminder.is_none() {
            return false;
        }

        if let Some((start, end)) = query.date_range {
            if bookmark.created_at < start || bookmark.created_at > end {
                return false;
            }
        }

        true
    }

    async fn count_user_bookmarks(&self, user_id: &str) -> usize {
        let bookmarks = self.bookmarks.read().await;
        bookmarks.values().filter(|b| b.user_id == user_id).count()
    }

    async fn count_user_collections(&self, user_id: &str) -> usize {
        let collections = self.collections.read().await;
        collections.values().filter(|c| c.owner_id == user_id).count()
    }

    async fn save_to_storage(&self) -> Result<()> {
        // Create storage directory if it doesn't exist
        if let Some(parent) = self.storage_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let bookmarks = self.bookmarks.read().await;
        let collections = self.collections.read().await;
        let collection_memberships = self.collection_memberships.read().await;
        let bookmark_collections = self.bookmark_collections.read().await;
        let config = self.config.read().await;

        let storage_data = BookmarkStorageData {
            bookmarks: bookmarks.clone(),
            collections: collections.clone(),
            collection_memberships: collection_memberships.clone(),
            bookmark_collections: bookmark_collections.clone(),
            config: config.clone(),
        };

        let json = serde_json::to_string_pretty(&storage_data)?;
        tokio::fs::write(&self.storage_path, json).await?;
        
        Ok(())
    }

    /// Load bookmark manager from storage
    pub async fn load_from_storage(storage_path: std::path::PathBuf) -> Result<Self> {
        let manager = Self::new(storage_path.clone());

        if storage_path.exists() {
            let json = tokio::fs::read_to_string(&storage_path).await?;
            let storage_data: BookmarkStorageData = serde_json::from_str(&json)?;

            *manager.bookmarks.write().await = storage_data.bookmarks;
            *manager.collections.write().await = storage_data.collections;
            *manager.collection_memberships.write().await = storage_data.collection_memberships;
            *manager.bookmark_collections.write().await = storage_data.bookmark_collections;
            *manager.config.write().await = storage_data.config;

            info!("Loaded bookmark manager from storage");
        }

        Ok(manager)
    }
}

/// Bookmark update parameters
#[derive(Debug, Default)]
pub struct BookmarkUpdates {
    pub title: Option<String>,
    pub note: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub priority: Option<BookmarkPriority>,
    pub color: Option<BookmarkColor>,
    pub quick_access: Option<bool>,
    pub reminder: Option<BookmarkReminder>,
    pub properties: Option<HashMap<String, String>>,
}

/// Storage data structure
#[derive(Debug, Serialize, Deserialize)]
struct BookmarkStorageData {
    bookmarks: HashMap<String, ConversationBookmark>,
    collections: HashMap<String, BookmarkCollection>,
    collection_memberships: HashMap<String, Vec<String>>,
    bookmark_collections: HashMap<String, Vec<String>>,
    config: BookmarkConfig,
}