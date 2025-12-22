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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let mut model = Model::new().expect("Failed to create model");

    // Load model at startup
    let model_path = env::var("MODEL_PATH")
        // .unwrap_or_else(|_| "./models/models/Q2_K-00001-of-00001.gguf".to_string());
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

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Daemon running on http://127.0.0.1:3000");

    serve(listener, app).await.unwrap();
}
