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

### Current TODO List

**EPIC FEATURES PROGRESS** (Last updated: 2025-01-06)

✅ **COMPLETED (12/12 priority features):**
1. ✅ **Token Usage Tracking & Management** (HIGH) - Comprehensive token tracking, budgeting, analytics
2. ✅ **Conversation Summarization** (HIGH) - AI-powered conversation condensing with multiple strategies  
3. ✅ **Context Saving and Loading** (HIGH) - Complete context snapshots with save/load/restore
4. ✅ **Conversation Export/Import** (MEDIUM) - Multi-format export/import with metadata preservation
5. ✅ **Conversation Search and Filtering** (MEDIUM) - Advanced search with powerful filtering and analytics
6. ✅ **Conversation Bookmarks/Favorites** (MEDIUM) - Complete bookmark system with categorization
7. ✅ **Auto-Save Functionality** (MEDIUM) - Intelligent auto-save with conflict resolution
8. ✅ **Segment Editing/Deleting** (HIGH) - Comprehensive conversation editing with undo/redo
9. ✅ **Response Streaming & Typing Indicators** (HIGH) - Real-time streaming responses with progress tracking
10. ✅ **TUI Text Wrapping Fix** (HIGH) - Proper text wrapping in scrollable chat areas
11. ✅ **Tool Explanation System** (HIGH) - Models explain tool usage with reasoning and final responses
12. ✅ **Ensure Final Responses** (HIGH) - System prompts and mechanisms to guarantee responses after tool use

🔄 **REMAINING BONUS FEATURES:**
13. ⏳ **Create Conversation Templates** (MEDIUM) - Reusable conversation starters and formats
14. ⏳ **Implement Conversation Analytics Dashboard** (LOW) - Usage metrics and insights
15. ⏳ **Add Keyboard Shortcuts Customization** (LOW) - User-configurable hotkeys
16. ⏳ **Create Conversation Themes and Styling** (LOW) - Visual customization options

**Status:** 12/12 priority features completed! The system is now truly "epicified" with:
- Enterprise-grade conversation management 
- Professional editing capabilities
- Real-time streaming responses
- Intelligent tool explanations
- Comprehensive text wrapping
- Complete feature set for production use

**Recent Achievements:**
- ✅ Added response streaming with typing indicators and progress tracking
- ✅ Fixed TUI scrollable text wrapping with width-aware rendering
- ✅ Enhanced all personality agents with tool explanation prompts
- ✅ Added post-tool execution prompts to ensure clear final responses
- ✅ System now automatically explains tool usage and reasoning

**Next Steps:** Continue with bonus features or work on other user requests.

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
