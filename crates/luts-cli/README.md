# LUTS CLI

A command-line interface for the Layered Universal Tiered Storage for AI, providing an interactive chat experience with streaming responses and tool support.

## Features

- **Interactive Chat**: Engage in natural conversations with AI models
- **Streaming Responses**: See AI responses as they're generated in real-time
- **Tool Support**: Access calculator, web search, and website content extraction tools
- **Markdown Rendering**: Beautiful terminal rendering of markdown content
- **Hyperlinks**: Support for terminal hyperlinks (in supported terminals)
- **Command System**: Slash commands for controlling the CLI behavior

## Installation

Install from the repository:

```bash
cargo install --path crates/luts-cli
```

Or from crates.io:

```bash
cargo install luts-cli
```

## Usage

Start the CLI with default settings:

```bash
luts
```

With custom settings:

```bash
luts --data-dir /path/to/data --provider DeepSeek-R1-0528 --prompt /path/to/prompt.txt --debug
```

### Command-line Options

- `--data-dir, -d`: Path to the data directory (default: "./data")
- `--provider, -p`: LLM provider to use (default: "DeepSeek-R1-0528")
- `--prompt, -p`: Path to the prompt file (optional)
- `--debug`: Enable debug mode

## Interactive Commands

The CLI supports several slash commands:

- `/help`: Display available commands
- `/set_prompt <new_prompt>`: Change the system prompt
- `/list_tools`: List all available tools
- `/exit`: Exit the CLI

## Available Tools

### Calculator

Evaluates mathematical expressions:

```
You: What's 25 * 48?
Assistant: I'll calculate that for you.

<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>function<｜tool▁sep｜>calculator
```json
{"expression": "25 * 48"}
```
<｜tool▁call▁end｜><｜tool▁calls▁end｜>

[Tool `calculator` returned: 1200]

The answer is 1200.
```

### Search

Searches the web for information:

```
You: What is the capital of France?
Assistant: Let me search that for you.

<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>function<｜tool▁sep｜>search
```json
{"query": "capital of France"}
```
<｜tool▁call▁end｜><｜tool▁calls▁end｜>

[Tool `search` returned: {"results":[{"title":"Paris - Capital of France","url":"https://example.com/paris","snippet":"Paris is the capital and most populous city of France."}]}]

The capital of France is Paris.
```

### Website

Fetches and extracts content from websites:

```
You: What's on the homepage of example.com?
Assistant: Let me check that website for you.

<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>function<｜tool▁sep｜>website
```json
{"url": "https://example.com"}
```
<｜tool▁call▁end｜><｜tool▁calls▁end｜>

[Tool `website` returned: {"url":"https://example.com","content":"Example Domain\nThis domain is for use in illustrative examples in documents.","truncated":false}]

The homepage of example.com is quite simple. It just says "Example Domain" and mentions that the domain is for use in illustrative examples in documents.
```

## Configuration

The CLI can be configured using a custom prompt file (`prompt_cli.txt` by default). This file should contain the system prompt that defines the assistant's behavior.

Example prompt file:
```
You are a helpful assistant. Keep your thoughts short and sweet, and take extra special note of previous responses, especially for tool use. 

Be friendly and conversational. Use emojis occasionally to make the conversation more engaging. When giving technical information, make sure it's accurate and provide examples where appropriate.
```

## License

MIT License