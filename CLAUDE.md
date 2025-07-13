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

#### Completed Tasks
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
- [x] **Make 5 built-in agents less built-in with database seeding** (HIGH)
  _Converted hardcoded built-in agents to database-seeded records with full CRUD operations and automatic seeding on startup_
- [x] **Build visual memory blocks management interface** (HIGH)
  _Created comprehensive web interface with variable block names, smart tag suggestions, content-based auto-generation, grid/list views, filtering, search, and statistics_

#### Architecture Refactoring (COMPLETED ✅)
- [x] **Create LUTS layered architecture plan document** (HIGH)
  _Detailed plan for splitting luts-core into layered crates - COMPLETED: Created comprehensive migration plan in LAYERED_ARCHITECTURE_PLAN.md_
- [x] **Phase 1: Create new crate structure and Cargo.toml files** (HIGH)
  _Set up crate directories and dependency structure - COMPLETED: Created 6 new crates (luts-common, luts-memory, luts-llm, luts-tools, luts-agents, luts-framework) with proper dependencies and verified compilation_
- [x] **Phase 2: Create luts-common crate with shared utilities** (HIGH)
  _Extract shared types, errors, and utilities - COMPLETED: Created comprehensive luts-common crate with config types, pricing logic, constants, common data types, and utility functions. All tests pass._
- [x] **Phase 3: Extract luts-memory from luts-core** (HIGH)
  _Memory architecture, embeddings, context management - COMPLETED: Created comprehensive luts-memory crate with memory blocks, storage traits, embedding services, and SurrealDB integration. All tests pass._
- [x] **Phase 4: Extract luts-llm (LLM + streaming + conversation)** (HIGH)
  _LLM integration, streaming infrastructure, conversation management - COMPLETED: Successfully extracted and migrated all LLM-related functionality_
- [x] **Phase 5: Extract luts-tools (pure tools)** (HIGH)
  _Tools that don't require agent functionality - COMPLETED: Pure tools extracted and working with new architecture_
- [x] **Phase 6: Extract luts-agents (agents + agent-specific tools)** (HIGH)
  _Agent system and tools that need agent context - COMPLETED: All agent functionality successfully migrated_
- [x] **Phase 7: Create luts-framework meta-crate** (HIGH)
  _Convenience crate that re-exports everything - COMPLETED: Framework crate provides unified API surface_
- [x] **Phase 8: Update applications to use luts-framework** (HIGH)
  _Update CLI, TUI, API to use new structure - COMPLETED: All applications updated and compiling successfully_
- [x] **Fix all compilation warnings and dead code** (HIGH)
  _Clean up unused imports, variables, and dead code - COMPLETED: Zero warnings across entire workspace_

#### SurrealDB Enhancements (COMPLETED ✅)
- [x] **Implement automatic embedding generation for memory blocks** (HIGH)
  _Add vector embeddings using SurrealDB vector functions - COMPLETED: Auto-embedding generation implemented with text content extraction_
- [x] **Add vector similarity search with SurrealDB** (HIGH)
  _Implement semantic search using SurrealDB vector capabilities - COMPLETED: MTREE indexing and vector search implemented_
- [x] **Create semantic search tools for agents** (HIGH)
  _Build agent tools that leverage vector similarity search - COMPLETED: Agent memory search tool with natural language interface_

#### Web Frontend Development (CURRENT FOCUS)
- [x] **TanStack Start project setup** (HIGH)
  _Modern React + TypeScript + TanStack Start foundation established in js/ folder_
- [x] **Create TanStack Router API routes to proxy to luts-api** (HIGH)
  _Successfully created /api/luts-health and /api/luts-chat endpoints_
- [x] **Connect web frontend to luts-api backend** (HIGH)
  _Backend connectivity established - health checks working, ready for chat integration_
- [ ] **Configure API keys and test chat completions** (HIGH)
  _Set up environment variables for LLM providers and test end-to-end chat flow_
- [ ] **Update demo chat to use luts-api instead of Claude API** (HIGH)
  _Replace direct Anthropic API calls with luts-api proxy endpoints_
- [ ] **Implement streaming responses via Server-Sent Events** (HIGH)
  _Add real-time streaming chat responses through luts-api_
- [ ] **Create LUTS agent selection interface** (HIGH)
  _Replace demo with LUTS agent selection and personality profiles_
- [x] **Build visual memory blocks management interface** (HIGH)
  _Created comprehensive web interface with variable block names, smart tag suggestions, content-based auto-generation, grid/list views, filtering, search, and statistics_
- [ ] **Add context window viewer with token usage visualization** (HIGH)
  _Visual context composition and token usage tracking_
- [ ] **Create tool activity dashboard with real-time monitoring** (MEDIUM)
  _Charts and metrics for tool execution and performance_


## Web Frontend Development (TanStack Start)

### Technology Stack & Setup
The web frontend is located in the `js/` folder and provides a modern web interface that surpasses the TUI application capabilities.

**Technology Stack:**
- **Framework**: TanStack Start (full-stack React framework)
- **Routing**: TanStack Router (file-based routing)
- **State Management**: TanStack Store (reactive state management)
- **Styling**: Tailwind CSS + shadcn/ui components
- **Icons**: Lucide React
- **Package Manager**: PNPM (as requested)
- **TypeScript**: Full type safety throughout
- **AI Integration**: Anthropic Claude API (to be replaced with luts-api)

**Project Structure:**
```
js/
├── src/
│   ├── routes/           # TanStack Router routes (file-based)
│   │   ├── __root.tsx   # Root layout with navigation
│   │   ├── index.tsx    # Home/landing page
│   │   ├── example.chat.tsx  # Current demo chat (to be LUTS-ified)
│   │   └── api/         # API routes for backend integration
│   ├── components/      # Reusable UI components (shadcn/ui)
│   ├── lib/            # Utilities and helpers
│   ├── store/          # TanStack Store state definitions
│   └── integrations/   # External service integrations
├── package.json        # Dependencies and scripts
├── components.json     # shadcn/ui configuration
└── README.md          # Project documentation
```

### Development Commands
```bash
cd js/

# Install dependencies (first time)
pnpm install

# Start development server
pnpm run dev                 # Runs on http://localhost:3000

# Production build
pnpm run build

# Run tests
pnpm run test

# Add shadcn/ui components
pnpx shadcn@latest add button card input textarea dialog
```

### Current Features (Demo Chat)
- ✅ Modern React + TypeScript setup
- ✅ TanStack Router with file-based routing  
- ✅ TanStack Store for state management
- ✅ Tailwind CSS + shadcn/ui components
- ✅ Lucide icons integration
- ✅ Chat interface foundations
- ✅ Markdown rendering with syntax highlighting
- ✅ Real-time messaging architecture
- ✅ Claude API integration (demo)

### Planned LUTS Integration
**Phase 1: Agent Selection Interface**
- Replace demo chat home page with LUTS agent selection
- Personality agent cards with descriptions and tool lists
- Agent configuration (tools, memory settings, etc.)
- Match TUI agent selection functionality but with better UX

**Phase 2: Streaming Conversation Interface**
- Real-time chat with streaming responses
- Tool execution visualization with status indicators
- Message history with proper formatting
- Better than TUI: rich formatting, tool activity sidebar

**Phase 3: Memory Blocks Management (Letta-style)**
- Visual memory block interface with drag-and-drop
- Block editing with rich text editor
- Block relationships and context visualization
- Far superior to TUI text-based interface

**Phase 4: Context Window Viewer**
- Visual representation of current context window
- Token usage bars and memory selection
- Interactive context block management
- Better than TUI: graphical representation

**Phase 5: Tool Activity Dashboard**
- Real-time tool execution monitoring
- Performance metrics and charts
- Error tracking and debugging interface
- Better than TUI: charts, graphs, real-time updates

**Phase 6: Backend Integration**
- Replace Claude API with luts-api backend
- Server-Sent Events (SSE) for streaming
- WebSocket for real-time tool monitoring
- Full OpenAI-compatible API support

### API Integration Strategy
- **Current**: Direct Claude API calls in demo
- **Target**: Proxy through TanStack Router API routes to luts-api
- **Streaming**: Server-Sent Events (SSE) for response streaming
- **Real-time**: WebSocket for tool activity monitoring
- **Compatibility**: OpenAI-compatible endpoints via luts-api

### Key Benefits Over TUI
1. **Visual Interface**: Drag-drop memory blocks, charts, rich formatting
2. **Real-time Updates**: Live tool monitoring, streaming responses
3. **Better UX**: Mouse interaction, modern UI components
4. **Rich Media**: Images, charts, better markdown rendering
5. **Multi-panel**: Side-by-side views, context + chat + tools
6. **Mobile Support**: Responsive design for tablets/phones


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

### Current: Monolithic Structure
This is a Rust workspace with three main components and a **multiagent system**:

### PLANNED: Layered Architecture (IN PROGRESS)

#### Target Structure
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

#### Dependency Flow
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

#### Crate Responsibilities

**luts-common**: Shared types, errors, tracing setup, basic utilities
- `LutsError`, `Result<T>` types
- Configuration structs 
- Logging/tracing setup
- Common constants

**luts-memory**: Reusable memory architecture library (core innovation)
- Memory blocks, embeddings, SurrealDB integration
- Context window management, core blocks
- Token management, block utilities
- **Dependencies**: Only luts-common + external crates

**luts-llm**: LLM integration and response processing
- LLM service, model abstraction
- Response streaming infrastructure  
- Conversation management, message handling
- **Dependencies**: luts-common, luts-memory

**luts-tools**: Pure AI tools (agent-independent)
- Calculator, web search, website scraping
- Semantic search (uses luts-memory)
- **Dependencies**: luts-common, luts-memory

**luts-agents**: Agent system and agent-specific functionality
- Base agent, personality agents, registry
- Agent-specific tools (modify_core_block, interactive_tester)
- **Dependencies**: luts-common, luts-memory, luts-llm, luts-tools

**luts-framework**: Convenience meta-crate
- Re-exports from all other luts-* crates
- Unified API surface
- **Dependencies**: All luts-* crates

#### Migration Benefits
1. **Modularity**: Users can pick and choose components
2. **Reusability**: luts-memory can be used independently
3. **Clear Dependencies**: Each layer only depends on layers below
4. **Testing**: Each layer can be tested independently
5. **Maintenance**: Focused scope per crate

### Current Core Library (`luts-core`)
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
