# Kekahyde

A decentralized, local-first AI runtime with P2P distributed computing. Runs LLMs locally with optional peer-to-peer inference offloading for enhanced performance and scalability. Includes a web frontend for easy interaction.

## Installation

Download the latest release from [GitHub Releases](https://github.com/OkarFabianTheWise/kekahyde/releases).

### System Requirements
- **OS**: Windows 10+, macOS 10.15+, or Linux (Ubuntu 18.04+)
- **RAM**: 4GB minimum (8GB+ recommended for larger models)
- **Storage**: 2GB free space (plus model files)
- **Dependencies**: None (bundled in installer)

### Installing on Windows
1. Download `Kekahyde_0.1.0_x64.msi` or `Kekahyde_0.1.0_x64.exe`.
2. Run the installer and follow the prompts.
3. Launch Kekahyde from the Start menu.

### Installing on macOS
1. Download `Kekahyde_0.1.0_x64.dmg`.
2. Open the DMG and drag Kekahyde to Applications.
3. Launch from Applications (you may need to allow it in Security settings).

### Installing on Linux
1. Download `kekahyde_0.1.0_amd64.deb` (Debian/Ubuntu) or `Kekahyde.AppImage` (universal).
2. For `.deb`: `sudo dpkg -i Kekahyde_0.1.0_amd64.deb`
3. For `.AppImage`: `chmod +x Kekahyde.AppImage && ./Kekahyde.AppImage`
4. Launch Kekahyde from your app menu.

### First Run
- On first launch, the app will check for the model file.
- If missing, download it manually:
  ```bash
  mkdir -p ~/.local/share/com.kekahyde.dev/models
  wget https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf -O ~/.local/share/com.kekahyde.dev/models/qwen2.5-0.5b-instruct-q4_k_m.gguf
  ```
- Then relaunch the app.

## Features

- **Local-first AI**: No internet required, runs entirely offline
- **P2P Distributed Computing**: Offload inference to peer devices for parallel processing
- **Real-time Streaming**: WebSocket-based status updates and result streaming
- **Cryptographic Verification**: Ensures integrity of distributed computations
- **Web Frontend**: Modern Next.js interface for prompt submission and monitoring
- **Async Architecture**: Built with Tokio for high concurrency
- **Resource Monitoring**: Tracks system usage and model status
- **Cancellation Support**: Graceful stopping of ongoing executions
- **Minimal API**: Clean HTTP/WebSocket endpoints

## Architecture

- **Model Engine**: Loads and runs GGUF models using llama.cpp bindings
- **HTTP Server**: Axum-based API server for inference and management
- **P2P Coordinator**: Manages peer connections and distributed execution
- **WebSocket Streaming**: Real-time updates for frontend
- **Frontend**: Next.js app with React components
- **State Management**: Shared async state for models, executions, and monitoring

## Quick Start

### Prerequisites
- Rust 1.70+ (2024 edition)
- Node.js 18+ (for frontend)
- Local GGUF model file (e.g., Qwen2.5 or Llama models)

### Build and Run

1. **Clone and build:**
   ```bash
   git clone <repo>
   cd kekahyde
   cargo build --release
   ```

2. **Set model path:**
   ```bash
   export MODEL_PATH=./models/models/qwen2.5-0.5b-instruct-q4_k_m.gguf
   ```

3. **Run the server:**
   ```bash
   ./target/release/kekahyde
   ```
   Server starts on `http://127.0.0.1:3000`

4. **(Optional) Run a peer for P2P:**
   ```bash
   ./target/release/kekahyde peer
   ```
   Peer listens on `127.0.0.1:8081`

5. **Start the frontend:**
   ```bash
   cd frontend
   pnpm install
   pnpm run dev
   ```
   Frontend available at `http://localhost:3000`

## API Endpoints

Server runs on `http://127.0.0.1:3000` by default.

### POST /run_prompt
Run a simple prompt (legacy endpoint).

**Request:**
```json
{
  "prompt": "Hello, world!",
  "policy": {
    "allow_networking": false,
    "allow_hybrid_compute": false,
    "allow_telemetry": false
  }
}
```

### POST /execution/start
Start an async execution with P2P support.

**Request:**
```json
{
  "prompt": "Explain quantum computing",
  "policy": {
    "allow_networking": false,
    "allow_hybrid_compute": true,
    "allow_telemetry": false
  }
}
```

**Response:**
```json
{
  "id": "uuid-here"
}
```

### GET /execution/status/:id
Get execution status.

**Response:**
```json
{
  "id": "uuid",
  "state": "Running",
  "result": null,
  "error": null,
  "start_time": "2025-12-23T12:00:00Z"
}
```

### POST /execution/cancel/:id
Cancel an execution.

### GET /status
Get system status.

**Response:**
```json
{
  "model_loaded": true,
  "cpu_usage": 25.5,
  "memory_usage": 1073741824,
  "state": "idle"
}
```

### WebSocket /ws/execution/:id
Subscribe to real-time execution updates.

## P2P Distributed Computing

Kekahyde supports offloading inference to peer devices for distributed processing:

- **Peer Discovery**: Manual configuration (expandable to mDNS/DHT)
- **Load Balancing**: Automatic distribution when peers available
- **Verification**: SHA256 hashes ensure result integrity
- **Fallback**: Seamless fallback to local execution if peers fail
- **Security**: No data leakage - only computed results are shared

### Running with P2P

1. Start one or more peers: `./kekahyde peer`
2. Start server: `./kekahyde`
3. Submit prompts with `"allow_hybrid_compute": true`

Peers will automatically handle inference requests.

## Configuration

- **Model Path**: Set `MODEL_PATH` environment variable
- **Host/Port**: Modify `main.rs` for custom binding
- **Peer Addresses**: Currently hardcoded; can be made configurable

## Dependencies

- `tokio`: Async runtime
- `axum`: HTTP/WebSocket server
- `llama-cpp-2`: LLM inference bindings
- `serde`: JSON handling
- `sha2`: Cryptographic hashing
- `sysinfo`: System monitoring
- `next.js`: Frontend framework
- `react`: UI components

## Current Implementation Status

✅ **Completed:**
- Full llama.cpp integration with Qwen2.5 support
- Async execution with cancellation
- P2P remote execution with verification
- WebSocket streaming
- Next.js frontend with prompt panels
- Resource monitoring
- Execution management

🔄 **In Progress:**
- Peer discovery mechanisms
- Multi-peer load balancing
- Advanced P2P protocols

📋 **Planned:**
- Model marketplace/download
- GPU acceleration detection
- Advanced security features
- Mobile peer support

## Limitations

- Single model per instance
- Manual peer configuration
- No streaming token-by-token (returns full response)
- Basic error handling

## Contributing

[Add contribution guidelines]

## License

[Add license]</content>
<parameter name="filePath">/home/orkarfabianthewise/code/kekahyde/README.md