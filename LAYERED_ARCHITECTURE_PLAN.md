# LUTS Layered Architecture Migration Plan

## Overview
Transform the current monolithic `luts-core` into a layered architecture with clear separation of concerns and dependencies flowing upward.

## Target Structure

```
luts/
├── crates/
│   ├── luts-common/       # Shared utilities and types
│   ├── luts-memory/       # Memory architecture (core innovation)
│   ├── luts-llm/          # LLM integration + streaming + conversation
│   ├── luts-agents/       # Agent system + agent-specific tools
│   ├── luts-tools/        # Pure AI tools (no agent dependencies)
│   ├── luts-framework/    # Meta-crate (convenience re-exports)
│   ├── luts-cli/          # CLI app (uses framework)
│   ├── luts-tui/          # TUI app (uses framework) 
│   └── luts-api/          # API server (uses framework)
```

## Dependency Flow

```
                    ┌─────────────────┐
                    │ luts-framework  │
                    └─────────┬───────┘
                             │
                    ┌────────▼────────┐
                    │  luts-agents    │
                    └─────────┬───────┘
                             │
            ┌────────────────┼────────────────┐
            │                │                │
    ┌───────▼───────┐ ┌──────▼──────┐ ┌──────▼──────┐
    │  luts-tools   │ │  luts-llm   │ │    apps     │
    └───────┬───────┘ └──────┬──────┘ └─────────────┘
            │                │
            │        ┌───────▼───────┐
            │        │ luts-memory   │
            │        └───────┬───────┘
            │                │
            │        ┌───────▼───────┐
            │        │ luts-common   │
            └────────┴───────────────┘
```

## Detailed Crate Breakdown

### 1. `luts-common` (Foundation Layer)
**Purpose**: Shared types, utilities, and error handling
**Contents**:
- Common error types (`LutsError`, `Result<T>`)
- Shared configuration types
- Basic utilities used across multiple crates
- Logging/tracing setup helpers
- Common constants and defaults
**Dependencies**: Only external crates (anyhow, serde, tracing, etc.)
**Public API**: Error types, config structs, utility functions

### 2. `luts-memory` (Memory Layer)
**Purpose**: Reusable memory architecture library - the core innovation
**Contents**:
- `memory/` - Memory blocks, embeddings, SurrealDB integration
- `context/` - Context window management, core blocks  
- `utils/blocks/` - Block-specific utilities
- `utils/tokens/` - Token management utilities
**Dependencies**: `luts-common` + external crates
**Public API**: Memory management, context windows, embeddings

### 3. `luts-llm` (LLM Integration Layer)  
**Purpose**: LLM service integration and response streaming
**Contents**:
- `llm/` - LLM service, model abstraction
- `streaming/` - Response streaming infrastructure
- `conversation/` - Conversation management, message handling
**Dependencies**: `luts-common`, `luts-memory`
**Public API**: LLM services, streaming responses, conversation management

### 4. `luts-tools` (Pure Tools Layer)
**Purpose**: Agent-independent AI tools
**Contents**:
- `calc/` - Calculator tool
- `search/` - Web search tool  
- `website/` - Website scraping tool
- `semantic_search/` - Memory search tool (uses luts-memory)
**Dependencies**: `luts-common`, `luts-memory`
**Public API**: Tool trait, individual tools

### 5. `luts-agents` (Agent Layer)
**Purpose**: Agent system and agent-specific functionality
**Contents**:
- `agents/` - Base agent, personality agents, registry, communication
- `tools/` - Agent-specific tools (modify_core_block, interactive_tester)
**Dependencies**: `luts-common`, `luts-memory`, `luts-llm`, `luts-tools`
**Public API**: Agent traits, personality builders, agent tools

### 6. `luts-framework` (Integration Layer)
**Purpose**: Convenience meta-crate
**Contents**:
- Re-exports from all other luts-* crates
- High-level convenience APIs
**Dependencies**: All luts-* crates
**Public API**: Everything, unified namespace

## Migration Strategy

### Phase 1: Create New Crate Structure
1. Create new `Cargo.toml` files for each crate
2. Set up proper dependency relationships in workspace `Cargo.toml`
3. Create empty `lib.rs` files with module declarations
4. Test that workspace builds with empty crates

**Files to create:**
- `crates/luts-common/Cargo.toml`
- `crates/luts-common/src/lib.rs`
- `crates/luts-memory/Cargo.toml`
- `crates/luts-memory/src/lib.rs`
- `crates/luts-llm/Cargo.toml`
- `crates/luts-llm/src/lib.rs`
- `crates/luts-tools/Cargo.toml`
- `crates/luts-tools/src/lib.rs`
- `crates/luts-agents/Cargo.toml`
- `crates/luts-agents/src/lib.rs`
- `crates/luts-framework/Cargo.toml`
- `crates/luts-framework/src/lib.rs`

### Phase 2: Create luts-common
1. Extract shared error types from luts-core
2. Move common configuration structs
3. Set up tracing/logging utilities
4. Ensure luts-common has no dependencies on other luts crates

**Files to move/create:**
- Common error types (`LutsError`, `Result<T>`)
- Configuration utilities
- Tracing setup helpers

### Phase 3: Extract luts-memory
1. Move `memory/`, `context/`, `utils/` to `luts-memory`
2. Update imports within moved modules
3. Ensure `luts-memory` depends only on `luts-common` + external crates
4. Update all internal imports to use the new structure

**Files to move:**
- `luts-core/src/memory/` → `luts-memory/src/memory/`
- `luts-core/src/context/` → `luts-memory/src/context/`
- `luts-core/src/utils/` → `luts-memory/src/utils/`

### Phase 4: Extract luts-llm
1. Move `llm/`, `streaming/`, `conversation/` to `luts-llm`
2. Update imports and ensure dependencies on `luts-common` and `luts-memory`
3. Ensure conversation management is properly integrated

**Files to move:**
- `luts-core/src/llm/` → `luts-llm/src/llm/`
- `luts-core/src/streaming/` → `luts-llm/src/streaming/`
- `luts-core/src/conversation/` → `luts-llm/src/conversation/`

### Phase 5: Extract luts-tools
1. Move pure tools (calc, search, website, semantic_search) to `luts-tools`
2. Update tool dependencies (most use `luts-memory`)
3. Ensure tools work independently of agents

**Files to move:**
- `luts-core/src/tools/calc/` → `luts-tools/src/calc/`
- `luts-core/src/tools/search/` → `luts-tools/src/search/`
- `luts-core/src/tools/website/` → `luts-tools/src/website/`
- `luts-core/src/tools/semantic_search/` → `luts-tools/src/semantic_search/`

### Phase 6: Extract luts-agents
1. Move `agents/` to `luts-agents`
2. Move agent-specific tools (modify_core_block, interactive_tester) to `luts-agents/src/tools/`
3. Update to use `luts-memory`, `luts-llm`, and `luts-tools` as dependencies
4. Ensure all agent functionality is properly integrated

**Files to move:**
- `luts-core/src/agents/` → `luts-agents/src/agents/`
- `luts-core/src/tools/modify_core_block/` → `luts-agents/src/tools/modify_core_block/`
- `luts-core/src/tools/interactive_tester/` → `luts-agents/src/tools/interactive_tester/`

### Phase 7: Create luts-framework
1. Set up `luts-framework` with re-exports from all other crates
2. Create unified API surface
3. Add any high-level convenience functions
4. Test that framework provides complete functionality

**Implementation:**
```rust
// luts-framework/src/lib.rs
pub use luts_common::*;
pub use luts_memory::*;
pub use luts_llm::*;
pub use luts_tools::*;
pub use luts_agents::*;

// High-level convenience APIs
pub mod prelude {
    pub use luts_memory::{MemoryManager, MemoryBlock};
    pub use luts_agents::{Agent, PersonalityAgentBuilder};
    pub use luts_tools::AiTool;
    // ... other commonly used items
}
```

### Phase 8: Update Applications
1. Update CLI, TUI, API to use `luts-framework` instead of `luts-core`
2. Test that all applications work correctly
3. Update documentation and examples

**Files to update:**
- `luts-cli/Cargo.toml` - change luts-core to luts-framework
- `luts-tui/Cargo.toml` - change luts-core to luts-framework  
- `luts-api/Cargo.toml` - change luts-core to luts-framework
- Update all `use luts_core::` to `use luts_framework::`

## Module Mapping

### What Goes Where:

**luts-common:**
- Error types and result aliases
- Configuration structs
- Logging setup
- Basic utilities (file paths, etc.)

**luts-memory:**
- Memory blocks and metadata
- Embeddings and vector search
- Context window management
- Core blocks system
- Token management
- SurrealDB integration

**luts-llm:**
- LLM service abstraction
- Response streaming
- Conversation management
- Message handling

**luts-tools:**
- `calc` - Mathematical calculations
- `search` - Web search
- `website` - Website scraping
- `semantic_search` - Memory-based search
- Base `AiTool` trait

**luts-agents:**
- Base `Agent` trait
- Personality agent implementations
- Agent registry and communication
- Agent-specific tools:
  - `modify_core_block` - Needs agent context
  - `interactive_tester` - Creates and manages agents

## Benefits of This Architecture

1. **Modularity**: Users can pick specific components (e.g., just luts-memory)
2. **Reusability**: luts-memory becomes a standalone library for memory management
3. **Clear Dependencies**: No circular dependencies, clean separation of concerns
4. **Testing**: Each layer can be tested independently
5. **Documentation**: Each crate has focused purpose and clear docs
6. **Maintenance**: Smaller, focused codebases are easier to maintain
7. **Performance**: Applications can include only what they need

## Potential Challenges

1. **API Surface**: Need to maintain ease of use while adding layers
2. **Version Management**: Multiple crates to version and release
3. **Integration Testing**: Need to test cross-crate functionality
4. **Documentation**: Need to document both individual crates and the framework

## Post-Migration Cleanup

1. Remove the old `luts-core` crate
2. Update all documentation to reflect new structure
3. Create examples showing how to use individual crates
4. Update CI/CD to handle multi-crate workspace
5. Consider publishing individual crates to crates.io

## Success Criteria

- [ ] All existing functionality works after migration
- [ ] All tests pass
- [ ] Applications (CLI, TUI, API) work correctly  
- [ ] `luts-memory` can be used independently
- [ ] Clean dependency graph with no cycles
- [ ] Clear documentation for each crate
- [ ] Performance is maintained or improved