# Kekahyde

A local-first AI runtime daemon written in Rust. Runs as a background process, loads local LLM models (GGUF format), and provides a simple HTTP API for inference. Designed for local-only operation with no network dependencies.

## Features

- **Local-first**: No internet required, no telemetry
- **Deterministic**: Explicit permissions and behavior
- **Async**: Built with Tokio for concurrent operations
- **Resource monitoring**: Tracks CPU and memory usage
- **Cancellation support**: Stop ongoing generations
- **Minimal API**: Simple HTTP endpoints for model management and inference

## Architecture

- **Model Engine**: Handles loading GGUF models and running prompts
- **IPC Server**: HTTP server using Axum for API requests
- **Resource Monitor**: System resource tracking with sysinfo
- **State Management**: Shared state for model, monitor, and runtime status

## API Endpoints

The daemon runs on `http://127.0.0.1:3000` by default.

### POST /load_model

Load a local LLM model.

**Request Body:**
```json
{
  "path": "/path/to/model.gguf"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Model loaded"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:3000/load_model \
  -H "Content-Type: application/json" \
  -d '{"path": "/home/user/models/llama-7b.gguf"}'
```

### POST /run_prompt

Run inference on a prompt. Returns the generated text as plain text.

**Request Body:**
```json
{
  "prompt": "Hello, how are you?"
}
```

**Response:** Plain text string

**Example:**
```bash
curl -X POST http://127.0.0.1:3000/run_prompt \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Explain recursion"}'
```

*Note: Streaming is not implemented in this minimal version; returns full response synchronously.*

### POST /stop

Cancel the current generation (if running).

**Request Body:** None

**Response:** 200 OK

**Example:**
```bash
curl -X POST http://127.0.0.1:3000/stop
```

### GET /status

Get current runtime status.

**Response:**
```json
{
  "model_loaded": true,
  "cpu_usage": 15.2,
  "memory_usage": 2048576,
  "state": "idle"
}
```

**Example:**
```bash
curl http://127.0.0.1:3000/status
```

## Building and Running

### Prerequisites

- Rust 1.70+ (2024 edition)
- Local GGUF model file (not included)

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run
```

The daemon will start and listen on `127.0.0.1:3000`.

### Daemon Mode

For production, consider using systemd or similar to run as a background service.

## Dependencies

- `tokio`: Async runtime
- `axum`: HTTP server framework
- `serde`: JSON serialization
- `sysinfo`: System resource monitoring
- `tower-http`: HTTP utilities
- `futures`: Async utilities

## Configuration

Currently hardcoded to `127.0.0.1:3000`. Modify `main.rs` for different host/port.

## Limitations

- Model loading is stubbed; integrate with llama.cpp bindings for real inference
- No streaming responses; returns full text at once
- Single model at a time
- No authentication or rate limiting
- Basic cancellation (resets state only)

> RUN
MODEL_PATH=./models/models/Q2_K-00001-of-00001.gguf cargo run

## Future Enhancements

- Real llama.cpp integration
- Token streaming
- Multiple model support
- Proper cancellation with async tokens
- Configuration file
- Logging
- Error handling improvements

## License

[Add license here]</content>
<parameter name="filePath">/home/orkarfabianthewise/code/kekahyde/README.md