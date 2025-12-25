use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// Define types for hybrid compute

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HiddenState {
    pub data: Vec<u8>, // Serialized hidden state
    pub hash: String,  // SHA256 of data
}

#[derive(Clone, Debug)]
pub struct Peer {
    pub id: String,
    pub address: String, // e.g., "127.0.0.1:8081"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerResponse {
    pub hidden_state: HiddenState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceResult {
    pub output: String,
    pub hash: String, // SHA256 of output
}

// Tokenized prompt
// pub type Tokens = Vec<llama_cpp_2::token::LlamaToken>;

pub struct HybridExecutor {
    pub peers: Vec<Peer>,
}

impl HybridExecutor {
    pub fn new() -> Self {
        let mut executor = Self { peers: vec![] };
        // Add dummy peers for testing
        executor.add_peer(Peer {
            id: "peer1".to_string(),
            address: "127.0.0.1:8081".to_string(),
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

    // Send prompt to peer and receive result
    pub async fn send_prompt_to_peer(prompt: &str, peer: &Peer) -> Result<InferenceResult, String> {
        let mut stream = TcpStream::connect(&peer.address)
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        // Send message: type 2 for prompt execution, length, prompt
        let mut message = vec![2u8]; // type
        let prompt_bytes = prompt.as_bytes();
        message.extend(&(prompt_bytes.len() as u32).to_le_bytes());
        message.extend(prompt_bytes);
        stream
            .write_all(&message)
            .await
            .map_err(|e| format!("Send failed: {}", e))?;

        // Receive response: type 3, length, data (JSON with output and hash)
        let mut type_buf = [0u8; 1];
        stream
            .read_exact(&mut type_buf)
            .await
            .map_err(|e| format!("Read type failed: {}", e))?;
        if type_buf[0] != 3 {
            return Err("Invalid response type".to_string());
        }
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| format!("Read length failed: {}", e))?;
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut data = vec![0u8; len];
        stream
            .read_exact(&mut data)
            .await
            .map_err(|e| format!("Read data failed: {}", e))?;

        let result: InferenceResult =
            serde_json::from_slice(&data).map_err(|e| format!("Deserialize failed: {}", e))?;

        // Verify hash
        let mut hasher = Sha256::new();
        hasher.update(&result.output);
        let computed_hash = format!("{:x}", hasher.finalize());
        if computed_hash != result.hash {
            return Err("Result hash mismatch".to_string());
        }

        Ok(result)
    }

    // Run distributed inference by offloading to a peer
    pub async fn run_distributed_inference(
        &self,
        _model: &crate::model::Model,
        prompt: &str,
        peer: &Peer,
    ) -> Result<String, String> {
        // Send the full prompt to peer for remote execution
        let result = Self::send_prompt_to_peer(prompt, peer).await?;
        // Result is already verified
        Ok(result.output)
    }
}
