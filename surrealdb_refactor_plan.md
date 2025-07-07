# SurrealDB Migration & Refactor Plan

## Overview

This document outlines the comprehensive plan to migrate LUTS from FjallMemoryStore to SurrealDB for enhanced memory block management, advanced querying, and real-time features. The migration will provide both local embedded and remote database options while maintaining backward compatibility.

## Goals

1. **Enhanced Query Capabilities**: Replace simple key-value storage with rich relational queries, semantic search, and graph relationships
2. **Real-time Features**: Enable live subscriptions for context window updates and collaborative editing
3. **Scalability**: Support larger datasets and more complex memory relationships
4. **Flexibility**: Provide both local embedded (SurrealDB file) and remote server options
5. **Data Integrity**: Maintain ACID compliance and robust data validation
6. **Performance**: Optimize for the specific access patterns of AI context management

## Current State Analysis

### Existing FjallMemoryStore Limitations
- **Simple Storage**: Key-value store with limited querying
- **No Relationships**: Memory blocks exist in isolation
- **No Real-time**: Changes require polling or manual refresh
- **Limited Search**: Basic text matching only
- **No Analytics**: Difficult to analyze memory usage patterns

### Current Data Model
```rust
// Memory blocks stored as individual CBOR-encoded entries
pub struct MemoryBlock {
    pub id: BlockId,
    pub user_id: String,
    pub session_id: Option<String>,
    pub block_type: BlockType,
    pub content: MemoryContent,
    pub metadata: BlockMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

## Phase 1: Foundation & Setup

### 1.1 SurrealDB Configuration Options

**Local Embedded Mode (Default)**
```rust
// For single-user, local development
SurrealConfig::File {
    path: "./data/memory.db",
    namespace: "luts",
    database: "memory",
}
```

**Local Server Mode**
```rust
// For development with SurrealDB server
SurrealConfig::Local {
    host: "127.0.0.1",
    port: 8000,
    namespace: "luts",
    database: "memory",
}
```

**Remote Server Mode**
```rust
// For production deployments
SurrealConfig::Remote {
    url: "wss://db.example.com",
    namespace: "luts_prod",
    database: "memory",
    auth: AuthConfig::RootAuth { username, password },
}
```

### 1.2 Enhanced MemoryStore Trait
```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    // Existing methods
    async fn store(&self, block: MemoryBlock) -> Result<BlockId>;
    async fn retrieve(&self, id: &BlockId) -> Result<Option<MemoryBlock>>;
    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryBlock>>;
    
    // New advanced methods
    async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<(MemoryBlock, f32)>>;
    async fn find_related(&self, block_id: &BlockId, relation_type: RelationType) -> Result<Vec<MemoryBlock>>;
    async fn aggregate_stats(&self, user_id: &str) -> Result<MemoryStats>;
    async fn subscribe_changes(&self, filter: ChangeFilter) -> Result<ChangeStream>;
    async fn create_relationship(&self, from: &BlockId, to: &BlockId, rel_type: RelationType) -> Result<()>;
    async fn batch_operation(&self, ops: Vec<BatchOperation>) -> Result<Vec<BatchResult>>;
}
```

### 1.3 Data Migration Strategy
- **Backward Compatibility**: Maintain FjallMemoryStore as fallback
- **Feature Flags**: Allow runtime switching between storage backends
- **Migration Tool**: CLI command to migrate existing data
- **Validation**: Ensure data integrity during migration

## Phase 2: Enhanced Data Model

### 2.1 SurrealDB Schema Design

**Core Tables**

```sql
-- Memory blocks with enhanced metadata
DEFINE TABLE memory_blocks SCHEMAFULL;
DEFINE FIELD id ON memory_blocks TYPE string;
DEFINE FIELD user_id ON memory_blocks TYPE string;
DEFINE FIELD session_id ON memory_blocks TYPE option<string>;
DEFINE FIELD block_type ON memory_blocks TYPE string;
DEFINE FIELD content ON memory_blocks TYPE object;
DEFINE FIELD metadata ON memory_blocks TYPE object;
DEFINE FIELD tags ON memory_blocks TYPE array<string>;
DEFINE FIELD embedding ON memory_blocks TYPE option<array<float>>;
DEFINE FIELD relevance_score ON memory_blocks TYPE option<float>;
DEFINE FIELD access_count ON memory_blocks TYPE number DEFAULT 0;
DEFINE FIELD last_accessed ON memory_blocks TYPE datetime;
DEFINE FIELD created_at ON memory_blocks TYPE datetime;
DEFINE FIELD updated_at ON memory_blocks TYPE datetime;

-- Core context blocks (system prompt, persona, etc.)
DEFINE TABLE core_blocks SCHEMAFULL;
DEFINE FIELD id ON core_blocks TYPE string;
DEFINE FIELD user_id ON core_blocks TYPE string;
DEFINE FIELD block_type ON core_blocks TYPE string;
DEFINE FIELD content ON core_blocks TYPE string;
DEFINE FIELD is_active ON core_blocks TYPE bool DEFAULT true;
DEFINE FIELD token_count ON core_blocks TYPE number DEFAULT 0;
DEFINE FIELD priority ON core_blocks TYPE number DEFAULT 5;
DEFINE FIELD created_at ON core_blocks TYPE datetime;
DEFINE FIELD updated_at ON core_blocks TYPE datetime;

-- Relationships between memory blocks
DEFINE TABLE block_relations SCHEMAFULL;
DEFINE FIELD in ON block_relations TYPE record<memory_blocks>;
DEFINE FIELD out ON block_relations TYPE record<memory_blocks>;
DEFINE FIELD relation_type ON block_relations TYPE string;
DEFINE FIELD strength ON block_relations TYPE float DEFAULT 1.0;
DEFINE FIELD created_at ON block_relations TYPE datetime;

-- User sessions and conversation context
DEFINE TABLE sessions SCHEMAFULL;
DEFINE FIELD id ON sessions TYPE string;
DEFINE FIELD user_id ON sessions TYPE string;
DEFINE FIELD agent_id ON sessions TYPE string;
DEFINE FIELD context_window_config ON sessions TYPE object;
DEFINE FIELD created_at ON sessions TYPE datetime;
DEFINE FIELD last_activity ON sessions TYPE datetime;
```

**Indexes for Performance**
```sql
-- Search and filtering indexes
DEFINE INDEX user_blocks ON memory_blocks FIELDS user_id, block_type;
DEFINE INDEX session_blocks ON memory_blocks FIELDS session_id, created_at;
DEFINE INDEX content_search ON memory_blocks FIELDS content SEARCH ANALYZER ascii BM25;
DEFINE INDEX tag_search ON memory_blocks FIELDS tags;
DEFINE INDEX relevance_ranking ON memory_blocks FIELDS relevance_score DESC;

-- Core blocks indexes
DEFINE INDEX user_core_blocks ON core_blocks FIELDS user_id, block_type;
DEFINE INDEX active_blocks ON core_blocks FIELDS user_id, is_active, priority DESC;
```

### 2.2 Enhanced Block Types
```rust
pub enum RelationType {
    References,      // Block A references information in Block B
    Contradicts,     // Block A contradicts Block B
    Supports,        // Block A supports/confirms Block B
    FollowsFrom,     // Block A is a logical consequence of Block B
    Generalizes,     // Block A is a generalization of Block B
    Specializes,     // Block A is a specialization of Block B
    Temporal,        // Block A happens before/after Block B
    Similarity,      // Block A is semantically similar to Block B
}

pub struct EnhancedMemoryBlock {
    pub id: BlockId,
    pub user_id: String,
    pub session_id: Option<String>,
    pub block_type: BlockType,
    pub content: MemoryContent,
    pub metadata: BlockMetadata,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,           // For semantic search
    pub relevance_score: Option<f32>,          // Dynamic relevance
    pub access_count: u64,                     // Usage tracking
    pub last_accessed: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

## Phase 3: Core Implementation

### 3.1 SurrealMemoryStore Implementation
```rust
pub struct SurrealMemoryStore {
    db: Surreal<Client>,
    config: SurrealConfig,
    connection_pool: Arc<Pool<Connection>>,
    change_listeners: Arc<RwLock<HashMap<String, ChangeListener>>>,
}

impl SurrealMemoryStore {
    pub async fn new(config: SurrealConfig) -> Result<Self> {
        let db = match config {
            SurrealConfig::File { path, namespace, database } => {
                let db = Surreal::new::<File>(path).await?;
                db.use_ns(namespace).use_db(database).await?;
                db
            }
            SurrealConfig::Local { host, port, namespace, database } => {
                let addr = format!("{}:{}", host, port);
                let db = Surreal::new::<Ws>(addr).await?;
                db.use_ns(namespace).use_db(database).await?;
                db
            }
            SurrealConfig::Remote { url, namespace, database, auth } => {
                let db = Surreal::new::<Wss>(url).await?;
                auth.authenticate(&db).await?;
                db.use_ns(namespace).use_db(database).await?;
                db
            }
        };
        
        // Initialize schema
        Self::initialize_schema(&db).await?;
        
        Ok(Self {
            db,
            config,
            connection_pool: Arc::new(Pool::new()),
            change_listeners: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}
```

### 3.2 Advanced Query Capabilities
```rust
impl SurrealMemoryStore {
    // Semantic search with vector similarity
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<(MemoryBlock, f32)>> {
        let embedding = self.generate_embedding(query).await?;
        
        let sql = "
            SELECT *, vector::similarity::cosine(embedding, $embedding) AS score 
            FROM memory_blocks 
            WHERE embedding IS NOT NONE 
            ORDER BY score DESC 
            LIMIT $limit
        ";
        
        let results: Vec<QueryResult> = self.db.query(sql)
            .bind(("embedding", embedding))
            .bind(("limit", limit))
            .await?;
            
        // Convert results to (MemoryBlock, score) tuples
        Ok(results.into_iter().map(|r| (r.into_memory_block(), r.score)).collect())
    }
    
    // Find related blocks through relationships
    pub async fn find_related(&self, block_id: &BlockId, relation_type: RelationType) -> Result<Vec<MemoryBlock>> {
        let sql = "
            SELECT out.* FROM block_relations 
            WHERE in = $block_id AND relation_type = $rel_type
        ";
        
        let blocks: Vec<MemoryBlock> = self.db.query(sql)
            .bind(("block_id", block_id.as_str()))
            .bind(("rel_type", relation_type.to_string()))
            .await?;
            
        Ok(blocks)
    }
    
    // Real-time change subscriptions
    pub async fn subscribe_changes(&self, filter: ChangeFilter) -> Result<ChangeStream> {
        let subscription_id = Uuid::new_v4().to_string();
        
        // Use SurrealDB's LIVE SELECT for real-time updates
        let live_query = format!("
            LIVE SELECT * FROM memory_blocks 
            WHERE user_id = '{}' AND block_type IN {}
        ", filter.user_id, filter.block_types_sql());
        
        let (tx, rx) = mpsc::channel(100);
        let listener = ChangeListener { sender: tx };
        
        self.change_listeners.write().await.insert(subscription_id.clone(), listener);
        
        Ok(ChangeStream {
            subscription_id,
            receiver: rx,
        })
    }
}
```

## Phase 4: Advanced Features

### 4.1 Knowledge Graph Construction
- **Automatic Relationship Detection**: Analyze content to suggest relationships
- **Graph Visualization**: Export relationship data for visualization tools
- **Path Finding**: Find connections between distant memory blocks
- **Cluster Analysis**: Group related memories automatically

### 4.2 Intelligent Context Management
```rust
pub struct IntelligentContextManager {
    store: Arc<dyn MemoryStore>,
    embedding_service: Arc<EmbeddingService>,
    relevance_scorer: Arc<RelevanceScorer>,
}

impl IntelligentContextManager {
    // Dynamic context window optimization
    pub async fn optimize_context_window(&self, user_id: &str, current_query: &str) -> Result<ContextWindow> {
        // 1. Get query embedding
        let query_embedding = self.embedding_service.embed(current_query).await?;
        
        // 2. Find semantically similar blocks
        let similar_blocks = self.store.semantic_search(current_query, 50).await?;
        
        // 3. Get related blocks through graph relationships
        let mut context_blocks = HashSet::new();
        for (block, _score) in &similar_blocks {
            let related = self.store.find_related(&block.id, RelationType::References).await?;
            context_blocks.extend(related);
        }
        
        // 4. Apply relevance scoring and token budgeting
        let scored_blocks = self.relevance_scorer.score_blocks(&context_blocks, &query_embedding).await?;
        let optimized = self.select_optimal_blocks(scored_blocks, 2000).await?;
        
        Ok(ContextWindow {
            core_blocks: self.get_active_core_blocks(user_id).await?,
            dynamic_blocks: optimized,
            total_tokens: self.calculate_total_tokens(&optimized).await?,
        })
    }
}
```

### 4.3 Real-time Context Viewer Updates
- **Live Subscriptions**: Context viewer updates automatically when memory changes
- **Collaborative Editing**: Multiple users can see real-time updates
- **Change Notifications**: Visual indicators for modified blocks
- **Conflict Resolution**: Handle concurrent edits gracefully

## Phase 5: Migration & Compatibility

### 5.1 Data Migration Tool
```bash
# CLI command for migrating from Fjall to SurrealDB
luts migrate --from fjall --to surrealdb --config ./surreal_config.toml --verify

# Options:
# --dry-run: Preview migration without making changes
# --batch-size: Number of blocks to migrate per batch
# --parallel: Number of parallel migration workers
# --verify: Validate data integrity after migration
```

### 5.2 Backward Compatibility
- **Feature Flags**: Runtime switching between backends
- **Graceful Degradation**: Fall back to Fjall if SurrealDB unavailable
- **Configuration Detection**: Auto-detect available storage backends

### 5.3 Configuration Management
```toml
# luts_config.toml
[storage]
# Primary storage backend
backend = "surrealdb"  # or "fjall"

# SurrealDB configuration
[storage.surrealdb]
mode = "file"  # "file", "local", "remote"

# File mode (embedded database)
[storage.surrealdb.file]
path = "./data/memory.db"
namespace = "luts"
database = "memory"

# Local server mode
[storage.surrealdb.local]
host = "127.0.0.1"
port = 8000
namespace = "luts"
database = "memory"

# Remote server mode  
[storage.surrealdb.remote]
url = "wss://db.example.com"
namespace = "luts_prod"
database = "memory"
username = "admin"
password = "${SURREALDB_PASSWORD}"

# Fallback configuration
[storage.fallback]
backend = "fjall"
data_dir = "./data"
```

## Phase 6: Performance & Optimization

### 6.1 Connection Pooling & Caching
- **Connection Pool**: Reuse database connections efficiently
- **Query Cache**: Cache frequently accessed blocks in memory
- **Result Cache**: Cache complex query results with TTL
- **Lazy Loading**: Load block content on-demand

### 6.2 Performance Monitoring
```rust
pub struct PerformanceMetrics {
    pub query_latency: Histogram,
    pub cache_hit_rate: Gauge,
    pub connection_pool_size: Gauge,
    pub active_subscriptions: Counter,
    pub memory_usage: Gauge,
}

// Export metrics for monitoring tools
impl PerformanceMetrics {
    pub fn export_prometheus(&self) -> String {
        // Export metrics in Prometheus format
    }
    
    pub fn export_json(&self) -> Value {
        // Export metrics as JSON for custom monitoring
    }
}
```

## Benefits of SurrealDB Migration

### 1. Enhanced Query Capabilities
- **Rich Queries**: Complex filtering, sorting, and aggregation
- **Full-text Search**: Built-in search with relevance ranking
- **Semantic Search**: Vector similarity for AI-powered search
- **Graph Traversal**: Follow relationships between memory blocks

### 2. Real-time Features
- **Live Updates**: Context viewer updates automatically
- **Collaborative Editing**: Multiple users editing shared memories
- **Change Streams**: React to memory changes in real-time
- **Conflict Resolution**: Handle concurrent modifications

### 3. Scalability & Performance
- **Optimized Storage**: Efficient storage of large memory datasets
- **Indexing**: Fast lookups and complex queries
- **Connection Pooling**: Handle high concurrency
- **Caching**: Intelligent caching for frequently accessed data

### 4. Developer Experience
- **SQL-like Queries**: Familiar query syntax
- **Type Safety**: Strong typing with Rust integration
- **Developer Tools**: Built-in administration and monitoring
- **Backup & Recovery**: Enterprise-grade data protection

### 5. Future-Proofing
- **Extensibility**: Easy to add new features and relationships
- **Multi-model**: Support for document, graph, and relational patterns
- **Distributed**: Scale across multiple nodes when needed
- **Standards**: ACID compliance and SQL compatibility

## Implementation Timeline

### Week 1-2: Foundation
- [ ] SurrealDB integration setup
- [ ] Enhanced MemoryStore trait design
- [ ] Configuration management
- [ ] Basic SurrealMemoryStore implementation

### Week 3-4: Data Model & Migration
- [ ] SurrealDB schema definition
- [ ] Enhanced memory block types
- [ ] Data migration tool
- [ ] Backward compatibility layer

### Week 5-6: Core Features
- [ ] Advanced querying capabilities
- [ ] Semantic search implementation
- [ ] Relationship management
- [ ] Real-time subscriptions

### Week 7-8: Integration & Testing
- [ ] Context viewer integration
- [ ] Agent tool updates
- [ ] Performance optimization
- [ ] Comprehensive testing

### Week 9-10: Polish & Documentation
- [ ] Performance monitoring
- [ ] Configuration examples
- [ ] Documentation updates
- [ ] Migration guides

## Success Metrics

1. **Performance**: 10x improvement in complex query performance
2. **Features**: Real-time updates working across all components
3. **Scalability**: Support for 10,000+ memory blocks per user
4. **Reliability**: 99.9% uptime with data integrity guarantees
5. **Developer Experience**: Reduced complexity for adding new memory features

## Risk Mitigation

1. **Complexity**: Phased rollout with feature flags
2. **Performance**: Extensive benchmarking and optimization
3. **Data Loss**: Comprehensive backup and migration validation
4. **Learning Curve**: Detailed documentation and examples
5. **Compatibility**: Maintain Fjall fallback option

This migration will transform LUTS from a simple AI assistant into an intelligent, context-aware system with sophisticated memory management and real-time collaboration capabilities.