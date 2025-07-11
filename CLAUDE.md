# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## TODO Management

**IMPORTANT**: Always keep TODOs synchronized between:
1. The TodoWrite/TodoRead tools during development
2. This CLAUDE.md file for persistence across sessions

### TODO Synchronization Instructions

**When starting a session:**
1. Check the TODO list below in this CLAUDE.md file
2. Load the current state into TodoWrite tool with exactly the same content
3. Use TodoRead to verify the sync was successful

**During development:**
1. Use TodoWrite to update task status as you work
2. IMMEDIATELY after updating TodoWrite, also update the TODO list in this CLAUDE.md file
3. Keep both lists identical at all times

**When finishing work:**
1. Ensure both the TodoWrite tool and this CLAUDE.md file have the same final state
2. This ensures the next session can start with the correct TODO status
3. Lint and check the files you've worked on.

### Current TODO List

- [x] **Token Usage Tracking & Management** (HIGH)
  _Comprehensive token tracking, budgeting, analytics_
- [x] **Conversation Summarization** (HIGH)
  _AI-powered conversation condensing with multiple strategies_
- [x] **Context Saving and Loading** (HIGH)
  _Complete context snapshots with save/load/restore_
- [x] **Conversation Export/Import** (MEDIUM)
  _Multi-format export/import with metadata preservation_
- [x] **Conversation Search and Filtering** (MEDIUM)
  _Advanced search with powerful filtering and analytics_
- [x] **Conversation Bookmarks/Favorites** (MEDIUM)
  _Complete bookmark system with categorization_
- [x] **Auto-Save Functionality** (MEDIUM)
  _Intelligent auto-save with conflict resolution_
- [x] **Segment Editing/Deleting** (HIGH)
  _Comprehensive conversation editing with undo/redo_
- [x] **Response Streaming & Typing Indicators** (HIGH)
  _Real-time streaming responses with progress tracking_
- [x] **TUI Text Wrapping Fix** (HIGH)
  _Proper text wrapping in scrollable chat areas_
- [x] **Tool Explanation System** (HIGH)
  _Models explain tool usage with reasoning and final responses_
- [x] **Ensure Final Responses** (HIGH)
  _System prompts and mechanisms to guarantee responses after tool use_
- [x] **Fix Tool Calling in TUI Streaming** (HIGH)
  _Fixed tool execution in streaming responses - tools now actually run and return results_
- [x] **Create Streaming Test Mode** (HIGH)
  _Comprehensive CLI test suite for streaming, tool calling, and error handling_
- [x] **Clean Up and Simplify Core Module** (HIGH)
  _Reorganized luts-core into logical modules: conversation/, streaming/, utils/, context/_
- [x] **Design Essential 'Core' Context Blocks** (HIGH)
  _Implemented 8 core block types: SystemPrompt, UserPersona, TaskContext, KeyFacts, etc._
- [x] **Context Window Manager (Letta-style)** (HIGH)
  _Complete context window management with dynamic memory selection and token budgeting_
- [x] **Fix Agent Tool Availability** (HIGH)
  _Agents can now access real tools including modify_core_block_
- [x] **Fix TUI Message Display** (HIGH)
  _Messages now properly scroll and wrap without disappearing_
- [x] **SurrealDB Foundation - Add dependencies and config** (HIGH)
  _Added SurrealDB to Cargo.toml with embedded kv-surrealkv feature_
- [x] **SurrealDB Data Model & Schema Design** (HIGH)
  _Created enhanced MemoryStore trait and SurrealDB schema with relationships_
- [x] **SurrealDB Core Implementation - Basic CRUD** (HIGH)
  _Implemented store/retrieve/update/delete with proper JSON serialization_
- [x] **Fix SurrealDB Thing Type Serialization Issues** (HIGH)
  _Fixed enum serialization issues following GitHub issue #4921 - using string-based approach instead of direct struct serialization to avoid SurrealDB 2.x enum bugs_
- [x] **Update Memory Manager to Use SurrealDB** (HIGH)
  _Switched all applications (API, TUI, Agents) from FjallMemoryStore to SurrealMemoryStore - complete migration to SurrealDB backend_
- [ ] **Implement automatic embedding generation for memory blocks** (HIGH)
  _Add vector embeddings using SurrealDB vector functions_
- [ ] **Add vector similarity search with SurrealDB** (HIGH)
  _Implement semantic search using SurrealDB vector capabilities_
- [ ] **Create semantic search tools for agents** (HIGH)
  _Build agent tools that leverage vector similarity search_
- [ ] **Update memory manager to use SurrealDB** (HIGH)
  _Switch MemoryManager from Fjall to SurrealDB backend_
- [ ] **Update context viewer for SurrealDB** (HIGH)
  _Modify TUI memory blocks viewer to work with SurrealDB_
- [ ] **Update all agent tools to work with SurrealDB** (HIGH)
  _Ensure retrieve_context and other tools work with new backend_
- [ ] **Remove Fjall dependencies and clean up old code** (MEDIUM)
  _Phase out Fjall completely in favor of SurrealDB_
- [ ] **Implement LIVE SELECT subscriptions for real-time updates** (MEDIUM)
  _Add real-time context updates using SurrealDB live queries_
- [ ] **Add full-text search with SurrealDB search indexes** (MEDIUM)
  _Implement advanced text search capabilities_
- [ ] **Create Conversation Templates** (MEDIUM)
  _Reusable conversation starters and formats_
- [ ] **Implement Conversation Analytics Dashboard** (LOW)
  _Usage metrics and insights_
- [ ] **Add Keyboard Shortcuts Customization** (LOW)
  _User-configurable hotkeys_
- [ ] **Create Conversation Themes and Styling** (LOW)
  _Visual customization options_


## Git Workflow - Feature Branches

**IMPORTANT**: Once the project is stable, we use feature branches for all development:

1. **Before starting any work**, create a feature branch:
   ```bash
   git checkout -b feature/descriptive-name
   # Examples: feature/add-default-impls, fix/batch-api-errors, docs/improve-examples
   ```

2. **Commit regularly** as you work:
   - After each logical change or set of related edits
   - Use clear, descriptive commit messages
   - Example: `git commit -m "Add Default impl for UpdateMemoryBlockRequest"`

3. **When feature is complete**, create a pull request to main
   - This keeps main stable and CI runs only on complete changes
   - Allows for code review and discussion

4. **Branch naming conventions**:
   - `claude/feature-` - New features or enhancements
   - `claude/fix-` - Bug fixes
   - `claude/docs-` - Documentation improvements
   - `claude/refactor-` - Code refactoring
   - `claude/test-` - Test additions or improvements

## Development Principles

- **ALWAYS check if files/scripts/functions exist before creating new ones** - Use `ls`, `find`, `grep`, or read existing code first
- Run `cargo check` frequently when producing code. This will help you catch errors early.
- NEVER use `unsafe{}`. If you feel you need to, stop, think about other ways, and ask the user for help if needed.
- NEVER ignore a failing test or change a test to make your code pass
- NEVER ignore a test
- ALWAYS fix compile errors before moving on.
- **ALWAYS ENSURE that tests will fail (via assert or panic with descriptive message) on any error condition**
- Use the web or context7 to help find docs, in addition to any other reference material

## Testing Strategy

All tests should validate actual behavior and be able to fail:
- **Unit tests**: Test individual functions with edge cases
- **Integration tests**: Test module interactions
- **Database tests**: Use in-memory SQLite for speed
- **No mock-heavy tests**: Prefer testing real behavior
- **Meaningful assertions**: Tests should catch actual bugs

Run tests with:
```bash
cargo test --lib           # Run all library tests
cargo test --lib -- db::   # Run specific module tests
just pre-commit-all        # Run all checks before committing
```


## Development Commands

### Build and Test
```bash
# Build the entire workspace
cargo build

# Build specific crate
cargo build -p luts-core
cargo build -p luts-cli
cargo build -p luts-api

# Run tests
cargo test

# Run tests for specific crate
cargo test -p luts-core
```

### Install and Run
```bash
# Install CLI from local path
cargo install --path crates/luts-cli

# Install TUI from local path
cargo install --path crates/luts-tui

# Install API server from local path
cargo install --path crates/luts-api

# Run CLI with personality agents
luts --list-agents                    # List all available personality agents
luts --agent researcher               # Start with specific agent
luts --data-dir ./data --provider gemini-2.5-pro  # With custom settings

# Run TUI application
luts-tui --list-agents               # List available personality agents
luts-tui --agent researcher          # Start TUI with specific agent
luts-tui --data-dir ./data --provider gemini-2.5-pro  # With custom settings

# Run TUI streaming test mode (for testing streaming, tool calls, etc.)
luts-tui --test-streaming            # Interactive test mode
luts-tui --list-test-scenarios       # List available test scenarios
luts-tui --test-streaming --test-scenario calculator  # Run specific test

# Run API server
luts-api --host 127.0.0.1 --port 3000 --data-dir ./data --provider DeepSeek-R1-0528
```

### Personality Agents (CLI)
The CLI now uses a multiagent system with personality-based agents:

- **Dr. Research** (`researcher`) - Thorough analyst with web search and scraping tools
- **Logic** (`calculator`) - Precise mathematical problem-solver with calculator
- **Spark** (`creative`) - Imaginative thinker using pure reasoning (no tools)
- **Maestro** (`coordinator`) - Strategic organizer with all tools available
- **Practical** (`pragmatic`) - Efficient problem-solver with essential tools

```bash
# Interactive agent selection (default behavior)
luts

# Direct agent selection
luts --agent creative
luts -a researcher

# Switch agents during conversation with /switch command
```

### Streaming Test Mode (TUI)
The TUI includes a comprehensive streaming test mode for testing streaming responses, tool calling, and error handling:

**Available Test Scenarios:**
- **Basic Streaming** - Test text streaming without tools
- **Calculator Tool** - Test mathematical tool calls
- **Web Search Tool** - Test web search functionality
- **Multiple Tools** - Test sequential tool usage
- **Error Handling** - Test tool error recovery
- **Stress Test** - High-volume tool calling

**Usage:**
```bash
# Interactive test mode (recommended)
luts-tui --test-streaming

# List all available test scenarios
luts-tui --list-test-scenarios

# Run specific test scenario
luts-tui --test-streaming --test-scenario calculator
luts-tui --test-streaming --test-scenario web-search

# Run all tests in sequence
luts-tui --test-streaming  # then select 'a' for all
```

**Features:**
- 🎯 **Targeted Testing** - Each scenario tests specific functionality
- 📊 **Real-time Metrics** - Duration, chunk count, tool usage stats
- 🎨 **Color-coded Output** - Visual indicators for success/failure/progress
- 🔧 **Tool Execution** - Actually runs tools and displays results
- ⚡ **Streaming Display** - Shows real-time response streaming
- 🚨 **Error Testing** - Validates error handling and recovery

### Development Tools
```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check code without building
cargo check
```

## Architecture

This is a Rust workspace with three main components and a **multiagent system**:

### Core Library (`luts-core`)
- **Context Management**: `context/` module with pluggable storage providers (Fjall, Redis)
- **Memory Blocks**: `memory/` module implementing structured AI context similar to Letta
- **Tools**: `tools/` module with AI assistant tools (math, search, website scraping)
- **LLM Service**: `llm.rs` for AI model interaction using the `genai` crate
- **Multiagent System**: `agents/` module with agent abstractions and personality-based agents

### CLI Application (`luts-cli`)
- **Personality-based agents** with different reasoning styles and tool sets
- Interactive agent selection menu
- Agent switching during conversations (`/switch`)
- Built with `clap` for command-line parsing
- Uses `termimad` for terminal markdown rendering

### TUI Application (`luts-tui`)
- **Interactive terminal interface** built with `ratatui`
- **Multi-mode interface**: Agent selection, conversation, memory blocks management, configuration
- **Memory Blocks Mode**: Letta-style memory management interface
  - Create, edit, and manage memory blocks (Message, Summary, Fact, Preference, etc.)
  - Interactive memory block editing and management
- **Navigation**: Switch between modes with `Ctrl+B` (memory blocks), `Ctrl+C` (config), `Ctrl+Q`/`Esc` (back)
- **Mouse and keyboard support** with vim-style navigation
- **Help system** with F1 key in each mode

### API Server (`luts-api`)
- OpenAI-compatible REST API using `axum`
- Implements chat completions endpoint
- Supports streaming responses
- Uses `tower-http` for middleware (CORS, tracing)

## Key Design Patterns

### Multiagent Architecture (NEW!)
- `Agent` trait for personality-based reasoning entities
- `PersonalityAgentBuilder` for creating specialized agents with different tool sets
- Agent-to-agent communication via `AgentMessage` and `MessageResponse`
- `AgentRegistry` for managing multiple agents (used in future multiagent scenarios)
- Agents vs Tools: Agents think and reason, Tools are shared utilities

### Storage Abstraction
- `ContextProvider` trait for pluggable context storage
- `MemoryStore` trait for memory block persistence
- Primary backend: Fjall (LSM-tree based storage)

### Memory Block System
- Structured blocks with metadata (user_id, session_id, timestamps)
- Block types: Message, Summary, Fact, Preference, PersonalInfo, Goal, Task
- Query system with filtering, sorting, and time ranges
- Content types: Text, JSON, Binary

### Tool Integration
- `AiTool` trait for extending AI capabilities
- Built-in tools: calculator, web search, website scraping
- JSON schema validation for tool parameters
- **Shared across agents**: Tools are utilities, not agents themselves

## Important Dependencies

- **genai**: LLM interaction and tool calling
- **fjall**: Primary storage backend (LSM-tree)
- **ratatui**: Terminal UI framework for `luts-tui`
- **axum**: Web framework for API server
- **clap**: Command-line argument parsing
- **serde**: JSON serialization throughout
- **tokio**: Async runtime
- **tracing**: Structured logging

## Data Storage

- Default data directory: `./data`
- Fjall stores create subdirectories for different data types
- Memory blocks stored with CBOR encoding for efficiency
- Context data stored as JSON values

## Configuration

- CLI accepts `--data-dir` and `--provider` arguments
- TUI accepts `--data-dir`, `--provider`, and `--agent` arguments for direct agent selection
- API server accepts `--host`, `--port`, `--data-dir`, `--provider`
- No config files currently - all configuration via command-line arguments
- Supports environment variables through `dotenvy` in CLI and TUI
