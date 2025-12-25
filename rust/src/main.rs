mod hybrid;
mod model;
mod monitor;
mod server;

use axum::serve;
use futures::TryStreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_util::io::StreamReader;

use hybrid::HybridExecutor;
use model::Model;
use monitor::Monitor;
use server::{AppState, ExecutionManager, create_router};

async fn download_model(url: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        println!("Model already exists at {:?}", path);
        return Ok(());
    }

    // Create directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!("Downloading model from {}...", url);
    let client = Client::new();
    let response = client.get(url).send().await?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    let mut file = File::create(path)?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.try_next().await? {
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download complete");
    Ok(())
}

#[tokio::main]
async fn main() {
    run_server().await;
}

async fn run_server() {
    tracing_subscriber::fmt().init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "peer" {
        run_as_peer().await;
        return;
    }

    let mut model = Model::new().expect("Failed to create model");

    // Load model at startup
    let model_path = env::var("MODEL_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME not set");
        format!(
            "{}/.local/share/com.kekahyde.dev/models/qwen2.5-0.5b-instruct-q4_k_m.gguf",
            home
        )
    });
    println!("Model path: {}", model_path);
    let model_path_path = Path::new(&model_path);
    if !model_path_path.exists() {
        let url = "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf";
        download_model(url, model_path_path)
            .await
            .expect("Failed to download model");
    }
    println!("Loading model from: {}", model_path);
    model
        .load_model(&model_path)
        .await
        .expect("Failed to load model");

    let app_state = Arc::new(AppState {
        model: Arc::new(Mutex::new(model)),
        monitor: Arc::new(Mutex::new(Monitor::new())),
        state: Arc::new(Mutex::new("idle".to_string())),
        execution_manager: Arc::new(Mutex::new(ExecutionManager::new())),
        hybrid_executor: Arc::new(Mutex::new(HybridExecutor::new())),
    });

    let app = create_router(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await;
    if let Err(e) = &listener {
        eprintln!(
            "Failed to bind to port 3000: {}. Please ensure no other process is using port 3000.",
            e
        );
        std::process::exit(1);
    }
    let listener = listener.unwrap();
    println!("Daemon running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn run_as_peer() {
    println!("Running as peer server on 127.0.0.1:8081");

    let mut model = Model::new().expect("Failed to create model");
    let model_path = env::var("MODEL_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME not set");
        format!(
            "{}/.local/share/com.kekahyde.dev/models/qwen2.5-0.5b-instruct-q4_k_m.gguf",
            home
        )
    });
    let model_path_path = Path::new(&model_path);
    if !model_path_path.exists() {
        let url = "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf";
        download_model(url, model_path_path)
            .await
            .expect("Failed to download model");
    }
    println!("Peer loading model from: {}", model_path);
    model
        .load_model(&model_path)
        .await
        .expect("Failed to load model");

    let model = Arc::new(Mutex::new(model));

    let listener = TcpListener::bind("127.0.0.1:8081").await.unwrap();

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let model = Arc::clone(&model);

        tokio::spawn(async move {
            use hybrid::InferenceResult;
            use serde_json;
            use sha2::{Digest, Sha256};
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            // Read type
            let mut type_buf = [0u8; 1];
            if socket.read_exact(&mut type_buf).await.is_err() {
                return;
            }
            if type_buf[0] != 2 {
                return; // Invalid type
            }

            // Read length
            let mut len_buf = [0u8; 4];
            if socket.read_exact(&mut len_buf).await.is_err() {
                return;
            }
            let len = u32::from_le_bytes(len_buf) as usize;

            // Read prompt
            let mut prompt_buf = vec![0u8; len];
            if socket.read_exact(&mut prompt_buf).await.is_err() {
                return;
            }
            let prompt = String::from_utf8(prompt_buf).unwrap();

            // Run inference
            let output = {
                let model = model.lock().await;
                model
                    .run_prompt(&prompt)
                    .await
                    .unwrap_or_else(|_| "Error".to_string())
            };

            // Compute hash
            let mut hasher = Sha256::new();
            hasher.update(&output);
            let hash = format!("{:x}", hasher.finalize());

            let result = InferenceResult { output, hash };
            let data = serde_json::to_vec(&result).unwrap();

            // Send response: type 3, length, data
            let mut message = vec![3u8];
            message.extend(&(data.len() as u32).to_le_bytes());
            message.extend(data);
            let _ = socket.write_all(&message).await;
        });
    }
}
