mod hybrid;
mod model;
mod monitor;
mod server;

use axum::serve;
use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use hybrid::HybridExecutor;
use model::Model;
use monitor::Monitor;
use server::{AppState, ExecutionManager, create_router};

#[cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            tauri::async_runtime::spawn(async move {
                run_server().await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
    let model_path = env::var("MODEL_PATH")
        .unwrap_or_else(|_| "./models/models/qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string());
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

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("Daemon running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn run_as_peer() {
    println!("Running as peer server on 127.0.0.1:8081");

    let mut model = Model::new().expect("Failed to create model");
    let model_path = env::var("MODEL_PATH")
        .unwrap_or_else(|_| "./models/models/qwen2.5-0.5b-instruct-q4_k_m.gguf".to_string());
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
