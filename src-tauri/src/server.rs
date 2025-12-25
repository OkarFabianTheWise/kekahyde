use axum::{
    Router,
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tokio_util::sync::CancellationToken;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use uuid::Uuid;

use crate::hybrid::HybridExecutor;
use crate::model::Model;
use crate::monitor::{Monitor, StatusResponse};

#[derive(Deserialize, Debug, Clone)]
pub struct Policy {
    pub allow_networking: bool,
    pub allow_hybrid_compute: bool,
    pub allow_telemetry: bool,
}

#[derive(Deserialize)]
struct RunPromptRequest {
    prompt: String,
    policy: Policy,
}

#[derive(Deserialize)]
struct StartExecutionRequest {
    prompt: String,
    policy: Policy,
}

#[derive(Serialize)]
struct StartExecutionResponse {
    id: String,
}

#[derive(Serialize, Clone)]
pub(crate) struct ExecutionStatus {
    id: String,
    state: String,
    result: Option<String>,
    error: Option<String>,
    start_time: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ExecutionState {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone)]
struct Execution {
    id: String,
    prompt: String,
    _policy: Policy,
    state: ExecutionState,
    start_time: DateTime<Utc>,
    result: Option<String>,
    error: Option<String>,
    cancel_token: Option<CancellationToken>,
}

pub struct ExecutionManager {
    executions: HashMap<String, Execution>,
    current: Option<String>,
    status_tx: broadcast::Sender<ExecutionStatus>,
}

impl ExecutionManager {
    pub fn new() -> Self {
        let (status_tx, _) = broadcast::channel(100);
        Self {
            executions: HashMap::new(),
            current: None,
            status_tx,
        }
    }

    fn start_execution(&mut self, prompt: String, policy: Policy) -> Result<String, String> {
        if self.current.is_some() {
            return Err("Another execution is already running".to_string());
        }
        let id = Uuid::new_v4().to_string();
        let cancel_token = CancellationToken::new();
        let execution = Execution {
            id: id.clone(),
            prompt,
            _policy: policy,
            state: ExecutionState::Queued,
            start_time: Utc::now(),
            result: None,
            error: None,
            cancel_token: Some(cancel_token),
        };
        self.executions.insert(id.clone(), execution);
        self.current = Some(id.clone());
        Ok(id)
    }

    fn get_execution(&self, id: &str) -> Option<&Execution> {
        self.executions.get(id)
    }

    fn cancel_execution(&mut self, id: &str) -> Result<(), String> {
        if let Some(execution) = self.executions.get_mut(id) {
            if execution.state == ExecutionState::Running {
                if let Some(token) = &execution.cancel_token {
                    token.cancel();
                }
                execution.state = ExecutionState::Cancelled;
                self.current = None;
                Ok(())
            } else {
                Err("Execution is not running".to_string())
            }
        } else {
            Err("Execution not found".to_string())
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ExecutionStatus> {
        self.status_tx.subscribe()
    }

    fn update_execution(
        &mut self,
        id: &str,
        state: ExecutionState,
        result: Option<String>,
        error: Option<String>,
    ) {
        if let Some(execution) = self.executions.get_mut(id) {
            execution.state = state.clone();
            execution.result = result.clone();
            execution.error = error.clone();
            if !matches!(state, ExecutionState::Running) {
                self.current = None;
            }
            // Send status update
            let status = ExecutionStatus {
                id: execution.id.clone(),
                state: format!("{:?}", execution.state),
                result: execution.result.clone(),
                error: execution.error.clone(),
                start_time: execution.start_time.to_rfc3339(),
            };
            let _ = self.status_tx.send(status);
        }
    }
}

pub struct AppState {
    pub model: Arc<Mutex<Model>>,
    pub monitor: Arc<Mutex<Monitor>>,
    pub state: Arc<Mutex<String>>,
    pub execution_manager: Arc<Mutex<ExecutionManager>>,
    pub hybrid_executor: Arc<Mutex<HybridExecutor>>,
}

fn enforce_policy(policy: Policy) -> Result<Policy, String> {
    // Reject privilege escalation - networking and telemetry are disabled by build configuration
    if policy.allow_networking {
        return Err("Networking is disabled by build configuration".into());
    }
    if policy.allow_telemetry {
        return Err("Telemetry is disabled by build configuration".into());
    }
    // Allow hybrid compute as per policy
    Ok(Policy {
        allow_networking: false,
        allow_hybrid_compute: policy.allow_hybrid_compute,
        allow_telemetry: false,
    })
}

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/run_prompt", post(run_prompt))
        .route("/stop", post(stop))
        .route("/status", get(status))
        .route("/execution/start", post(start_execution))
        .route("/execution/cancel/:id", post(cancel_execution))
        .route("/execution/status/:id", get(execution_status))
        .route("/ws/execution/:id", get(execution_ws))
        .fallback_service(ServeDir::new("frontend/out"))
        .layer(cors)
        .with_state(state)
}

async fn run_prompt(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RunPromptRequest>,
) -> Result<axum::response::Response<String>, StatusCode> {
    let _enforced_policy = enforce_policy(req.policy).map_err(|e| {
        tracing::error!("Policy enforcement failed: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    tracing::info!("Running prompt: {}", req.prompt);
    let model = state.model.lock().await;
    *state.state.lock().await = "running".to_string();
    match model.run_prompt(&req.prompt).await {
        Ok(response) => {
            *state.state.lock().await = "idle".to_string();
            tracing::info!("Prompt executed successfully");
            Ok(axum::response::Response::builder()
                .header("content-type", "text/plain")
                .body(response)
                .unwrap())
        }
        Err(e) => {
            *state.state.lock().await = "idle".to_string();
            tracing::error!("Failed to run prompt: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn stop(State(state): State<Arc<AppState>>) -> StatusCode {
    // For cancellation, need to implement
    *state.state.lock().await = "idle".to_string();
    StatusCode::OK
}

async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    tracing::debug!("Status requested");
    let mut monitor = state.monitor.lock().await;
    let model_loaded = state.model.lock().await.is_loaded();
    let current_state = state.state.lock().await.clone();
    let status = monitor.get_status(model_loaded, &current_state);
    Json(status)
}

async fn start_execution(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartExecutionRequest>,
) -> Result<Json<StartExecutionResponse>, StatusCode> {
    let _enforced_policy =
        enforce_policy(req.policy.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut manager = state.execution_manager.lock().await;
    match manager.start_execution(req.prompt, _enforced_policy) {
        Ok(id) => {
            // Spawn the execution task
            let execution_manager_clone = Arc::clone(&state.execution_manager);
            let model_clone = Arc::clone(&state.model);
            let hybrid_clone = Arc::clone(&state.hybrid_executor);
            let execution = manager.executions.get(&id).unwrap().clone();
            let id_clone = id.clone();
            drop(manager); // release lock

            tokio::spawn(async move {
                let mut mgr = execution_manager_clone.lock().await;
                mgr.update_execution(&id_clone, ExecutionState::Running, None, None);
                drop(mgr);

                let enforced_policy = execution._policy.clone();
                let use_hybrid = {
                    let hybrid = hybrid_clone.lock().await;
                    hybrid.should_use_hybrid(enforced_policy.allow_hybrid_compute)
                        && !hybrid.peers.is_empty()
                };

                if use_hybrid {
                    // Distributed execution: offload to peer
                    let peer = {
                        let hybrid = hybrid_clone.lock().await;
                        hybrid.peers.first().cloned().unwrap() // Safe since we checked !is_empty
                    };

                    let result = {
                        let hybrid = hybrid_clone.lock().await;
                        hybrid
                            .run_distributed_inference(
                                &*model_clone.lock().await,
                                &execution.prompt,
                                &peer,
                            )
                            .await
                    };

                    let result = match result {
                        Ok(output) => Ok(output),
                        Err(e) => {
                            tracing::warn!(
                                "Distributed execution failed, falling back to local: {}",
                                e
                            );
                            // Fallback to local on failure
                            let model = model_clone.lock().await;
                            let cancel_token = execution.cancel_token.as_ref().unwrap().clone();
                            tokio::select! {
                                res = model.run_prompt(&execution.prompt) => res,
                                _ = cancel_token.cancelled() => {
                                    let mut mgr = execution_manager_clone.lock().await;
                                    mgr.update_execution(&id_clone, ExecutionState::Cancelled, None, None);
                                    return;
                                }
                            }
                        }
                    };

                    let mut mgr = execution_manager_clone.lock().await;
                    match result {
                        Ok(r) => mgr.update_execution(
                            &id_clone,
                            ExecutionState::Completed,
                            Some(r),
                            None,
                        ),
                        Err(e) => {
                            mgr.update_execution(&id_clone, ExecutionState::Failed, None, Some(e))
                        }
                    }
                } else {
                    // Local execution
                    let model = model_clone.lock().await;
                    let cancel_token = execution.cancel_token.as_ref().unwrap().clone();

                    let result = tokio::select! {
                        res = model.run_prompt(&execution.prompt) => res,
                        _ = cancel_token.cancelled() => {
                            let mut mgr = execution_manager_clone.lock().await;
                            mgr.update_execution(&id_clone, ExecutionState::Cancelled, None, None);
                            return;
                        }
                    };

                    let mut mgr = execution_manager_clone.lock().await;
                    match result {
                        Ok(r) => mgr.update_execution(
                            &id_clone,
                            ExecutionState::Completed,
                            Some(r),
                            None,
                        ),
                        Err(e) => {
                            mgr.update_execution(&id_clone, ExecutionState::Failed, None, Some(e))
                        }
                    }
                }
            });

            Ok(Json(StartExecutionResponse { id }))
        }
        Err(_) => Err(StatusCode::CONFLICT), // Another execution running
    }
}

async fn cancel_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> StatusCode {
    let mut manager = state.execution_manager.lock().await;
    match manager.cancel_execution(&id) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

async fn execution_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Value> {
    let manager = state.execution_manager.lock().await;
    if let Some(execution) = manager.get_execution(&id) {
        Json(
            serde_json::to_value(ExecutionStatus {
                id: execution.id.clone(),
                state: format!("{:?}", execution.state),
                result: execution.result.clone(),
                error: execution.error.clone(),
                start_time: execution.start_time.to_rfc3339(),
            })
            .unwrap(),
        )
    } else {
        Json(serde_json::json!({"error": "Execution not found"}))
    }
}

async fn execution_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, id))
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>, id: String) {
    let (mut sender, mut receiver) = socket.split();

    // Send initial status
    let manager = state.execution_manager.lock().await;
    if let Some(execution) = manager.get_execution(&id) {
        let status = ExecutionStatus {
            id: execution.id.clone(),
            state: format!("{:?}", execution.state),
            result: execution.result.clone(),
            error: execution.error.clone(),
            start_time: execution.start_time.to_rfc3339(),
        };
        if let Ok(msg) = serde_json::to_string(&status) {
            let _ = sender
                .send(axum::extract::ws::Message::Text(msg.into()))
                .await;
        }
    }
    drop(manager);

    // Subscribe to updates
    let mut rx = state.execution_manager.lock().await.subscribe();

    tokio::spawn(async move {
        while let Ok(status) = rx.recv().await {
            if status.id == id {
                if let Ok(msg) = serde_json::to_string(&status) {
                    if sender
                        .send(axum::extract::ws::Message::Text(msg.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    // Handle incoming messages (for ping/pong, etc.)
    while let Some(msg) = receiver.next().await {
        if let Ok(axum::extract::ws::Message::Close(_)) = msg {
            break;
        }
    }
}
