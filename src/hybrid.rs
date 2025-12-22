use crate::server::Policy;
use llama_cpp_2::token::LlamaToken;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// Define types for hybrid compute

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub data: Vec<f32>, // Numeric representation, e.g., token embeddings or layer outputs
    pub metadata: HashMap<String, String>, // Additional info like layer index, etc.
}

#[derive(Clone, Debug)]
pub struct Peer {
    pub id: String,
    pub address: String, // e.g., IP:port for encrypted channel
                         // In real impl, would have encryption keys, etc.
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialResult {
    pub chunk_id: String,
    pub data: Vec<f32>,
    pub hash: String, // SHA256 of data
}

#[derive(Clone, Debug)]
pub struct FullResult {
    pub result: String,
    pub partials: Vec<PartialResult>,
}

// Placeholder for Prompt, assuming it's the tokenized prompt
pub type Prompt = Vec<LlamaToken>; // Token IDs

pub struct HybridExecutor {
    pub peers: Vec<Peer>,
    // In real impl, connection pools, etc.
}

impl HybridExecutor {
    pub fn new() -> Self {
        let mut executor = Self { peers: vec![] };
        // Add dummy peers for testing
        executor.add_peer(Peer {
            id: "peer1".to_string(),
            address: "http://dummy1:8080".to_string(),
        });
        executor.add_peer(Peer {
            id: "peer2".to_string(),
            address: "http://dummy2:8080".to_string(),
        });
        executor
    }

    pub fn add_peer(&mut self, peer: Peer) {
        self.peers.push(peer);
    }

    // Decide if to use hybrid based on policy and availability
    pub fn should_use_hybrid(&self, allow_hybrid: bool) -> bool {
        allow_hybrid && !self.peers.is_empty()
    }

    // Split prompt into chunks for remote execution
    pub fn split_prompt_for_remote(&self, prompt: &Prompt) -> Vec<Chunk> {
        // Simple split: divide tokens into chunks
        let chunk_size = 10; // Example
        let mut chunks = vec![];
        for (i, chunk_tokens) in prompt.chunks(chunk_size).enumerate() {
            let data = chunk_tokens.iter().map(|&t| t.0 as f32).collect(); // Placeholder numeric rep
            let mut metadata = HashMap::new();
            metadata.insert("index".to_string(), i.to_string());
            chunks.push(Chunk {
                id: format!("chunk_{}", i),
                data,
                metadata,
            });
        }
        chunks
    }

    // Send chunk to peer (placeholder, simulate encrypted channel)
    pub async fn send_chunk_to_peer(chunk: &Chunk, _peer: &Peer) -> Result<PartialResult, String> {
        // Simulate network delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Simulate processing: reverse the data or something
        let processed_data = chunk.data.iter().rev().cloned().collect::<Vec<_>>();

        // Compute hash
        let mut hasher = Sha256::new();
        for &val in &processed_data {
            hasher.update(val.to_le_bytes());
        }
        let hash = format!("{:x}", hasher.finalize());

        Ok(PartialResult {
            chunk_id: chunk.id.clone(),
            data: processed_data,
            hash,
        })
    }

    // Merge partial results into full result
    pub fn merge_results(&self, partials: &[PartialResult]) -> FullResult {
        // Sort partials by chunk_id (assuming ids are chunk_0, chunk_1, etc.)
        let mut sorted_partials = partials.to_vec();
        sorted_partials.sort_by_key(|p| {
            p.chunk_id
                .strip_prefix("chunk_")
                .unwrap()
                .parse::<usize>()
                .unwrap()
        });

        // Concatenate data
        let mut merged_data = Vec::new();
        for partial in &sorted_partials {
            merged_data.extend(&partial.data);
        }

        // For result, convert to string (placeholder)
        let result = merged_data
            .iter()
            .map(|f: &f32| f.to_string())
            .collect::<Vec<String>>()
            .join(" ");

        FullResult {
            result,
            partials: sorted_partials,
        }
    }

    // Verify result
    pub fn verify_result(&self, result: &FullResult) -> bool {
        // Check that all partials have correct hashes
        result.partials.iter().all(|p| {
            let mut hasher = Sha256::new();
            for &val in &p.data {
                hasher.update(val.to_le_bytes());
            }
            let computed_hash = format!("{:x}", hasher.finalize());
            computed_hash == p.hash
        })
    }

    // Execute distributed prompt
    pub async fn execute_distributed(
        &self,
        prompt: &Prompt,
        policy: &Policy,
    ) -> Result<FullResult, String> {
        if !policy.allow_hybrid_compute || self.peers.is_empty() {
            return Err("Hybrid compute not allowed or no peers available".to_string());
        }

        let chunks = self.split_prompt_for_remote(prompt);
        let mut partials = Vec::new();
        let mut tasks = Vec::new();

        // Assign chunks to peers (simple round-robin)
        for (i, chunk) in chunks.into_iter().enumerate() {
            let peer = &self.peers[i % self.peers.len()];
            let chunk_clone = chunk.clone();
            let peer_clone = peer.clone();
            let task =
                tokio::spawn(
                    async move { Self::send_chunk_to_peer(&chunk_clone, &peer_clone).await },
                );
            tasks.push(task);
        }

        // Wait for all tasks, with timeout
        let timeout_duration = std::time::Duration::from_secs(30);
        let mut results = Vec::new();
        for task in tasks {
            let res = tokio::time::timeout(timeout_duration, task).await;
            results.push(res);
        }

        for result in results {
            match result {
                Ok(Ok(Ok(partial))) => partials.push(partial),
                Ok(Ok(Err(e))) => return Err(format!("Chunk execution failed: {}", e)),
                Ok(Err(e)) => return Err(format!("Task panicked: {:?}", e)),
                Err(_) => return Err("Chunk execution timed out".to_string()),
            }
        }

        let full_result = self.merge_results(&partials);
        if self.verify_result(&full_result) {
            Ok(full_result)
        } else {
            Err("Result verification failed".to_string())
        }
    }
}
