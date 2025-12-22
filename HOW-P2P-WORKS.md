# How Decentralized Compute Works in Kekahyde

Kekahyde's decentralized compute feature enables distributed AI prompt execution across peer devices while maintaining strict privacy guarantees. Here's how it operates:

## Core Workflow

1. **Prompt Splitting**: When a user submits a prompt with decentralized compute enabled, the system tokenizes the prompt and splits it into safe, numeric-only chunks. These chunks contain only mathematical representations (tensors/floats) of the prompt data, never readable text.

2. **Peer Discovery & Assignment**: The system discovers available peers through a P2P handshake mechanism. Chunks are assigned to different peers based on availability and load balancing.

3. **Distributed Execution**: Each peer processes its assigned chunk locally using their own compute resources. Only intermediate numeric states and results are transmitted between devices.

4. **Result Collection & Verification**: Partial results are received asynchronously from peers. Each result includes a cryptographic hash (SHA256) for integrity verification. The system detects and rejects any incorrect or malicious results.

5. **Merging & Streaming**: Verified partial results are merged locally into the final output. Tokens are streamed in real-time to the UI via WebSocket connections.

## Security & Privacy

- **Zero Text Exposure**: No readable prompt text ever leaves the local device. Only numeric tensors are shared.
- **Cryptographic Verification**: All results are hashed and verified to ensure authenticity.
- **Encryption**: All peer communications use mandatory encryption (framework ready for TLS/WebSocket secure channels).
- **Timeout & Fallback**: If a peer fails or times out (30 seconds), the system automatically falls back to local execution.

## Architecture Components

- **ExecutionManager**: Coordinates chunk assignment, peer selection, timeouts, and retries.
- **HybridExecutor**: Handles both local and distributed execution paths.
- **WebSocket Streaming**: Provides real-time status updates to the frontend UI.

## Key Functions

- `split_prompt_for_remote()`: Divides tokenized prompts into chunks
- `send_chunk_to_peer()`: Securely transmits chunks to peers (placeholder for encrypted channels)
- `merge_results()`: Combines verified partial results
- `verify_result()`: Validates results using cryptographic hashes
- `execute_distributed()`: Orchestrates the full distributed workflow

## Current Limitations

- No token economy or rewards system
- Single-layer peer network (no multi-hop routing)
- No cloud fallback mechanisms
- Dummy peer implementation for testing

The system ensures that AI computation can be distributed across devices without compromising user privacy or data security.