# LUTS API

An OpenAI-compatible API server for the Layered Universal Tiered Storage for AI, providing a drop-in replacement for OpenAI services.

## Features

- **OpenAI API Compatibility**: Compatible with OpenAI's chat completions API
- **Streaming Support**: Server-sent events (SSE) for streaming responses
- **Tool Calling**: Support for function calling compatible with OpenAI's format
- **CORS Support**: Built-in CORS configuration for web applications
- **Health Endpoint**: Simple health check endpoint for monitoring
- **Customizable**: Configure the server with command-line options

## Installation

Install from the repository:

```bash
cargo install --path crates/luts-api
```

Or from crates.io:

```bash
cargo install luts-api
```

## Usage

Start the API server with default settings:

```bash
luts-api
```

With custom settings:

```bash
luts-api --host 0.0.0.0 --port 8080 --data-dir /path/to/data --provider DeepSeek-R1-0528 --prompt /path/to/prompt.txt
```

### Command-line Options

- `--host, -h`: Host to bind to (default: "127.0.0.1")
- `--port, -p`: Port to listen on (default: 3000)
- `--data-dir, -d`: Path to the data directory (default: "./data")
- `--provider, -p`: LLM provider to use (default: "DeepSeek-R1-0528")
- `--prompt, -p`: Path to the prompt file (optional)

## API Endpoints

### `POST /v1/chat/completions`

Creates a completion for the chat message.

**Request:**

```json
{
  "model": "DeepSeek-R1-0528",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello, how are you?"}
  ],
  "stream": false,
  "temperature": 0.7,
  "max_tokens": 1000,
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "calculator",
        "description": "Evaluates mathematical expressions",
        "parameters": {
          "type": "object",
          "properties": {
            "expression": {
              "type": "string",
              "description": "The mathematical expression to evaluate"
            }
          },
          "required": ["expression"]
        }
      }
    }
  ]
}
```

**Response:**

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1677858242,
  "model": "DeepSeek-R1-0528",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! I'm doing well, thank you for asking. How can I assist you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 25,
    "completion_tokens": 16,
    "total_tokens": 41
  }
}
```

### `GET /v1/models`

Returns a list of available models.

**Response:**

```json
{
  "object": "list",
  "data": [
    {
      "id": "DeepSeek-R1-0528",
      "object": "model",
      "created": 1716508800,
      "owned_by": "luts"
    }
  ]
}
```

### `GET /health`

Health check endpoint.

**Response:**

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

## Using with OpenAI Clients

Since the API is compatible with OpenAI's format, you can use it with any OpenAI client library by just changing the base URL:

### Python (with OpenAI SDK)

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:3000/v1",
    api_key="not-needed"  # API key is ignored but required by the client
)

completion = client.chat.completions.create(
    model="DeepSeek-R1-0528",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello, how are you?"}
    ]
)

print(completion.choices[0].message.content)
```

### JavaScript/TypeScript

```javascript
import OpenAI from 'openai';

const openai = new OpenAI({
  baseURL: 'http://localhost:3000/v1',
  apiKey: 'not-needed', // API key is ignored but required by the client
});

const completion = await openai.chat.completions.create({
  model: 'DeepSeek-R1-0528',
  messages: [
    { role: 'system', content: 'You are a helpful assistant.' },
    { role: 'user', content: 'Hello, how are you?' }
  ],
});

console.log(completion.choices[0].message.content);
```

## Configuration

The API server can be configured using a custom prompt file (`prompt_api.txt` by default). This file should contain the system prompt that defines the assistant's behavior.

Example prompt file:
```
You are a helpful AI assistant focused on providing accurate information. You can use tools to help you answer questions. Always maintain a professional tone and prioritize clarity in your responses.
```

## License

MIT License