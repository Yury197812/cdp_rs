// orchestrator/dashboard/server/mod.rs - API server with axum
use axum::{
    routing::get,
    Router, Json,
    extract::State,
    http::StatusCode,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::orchestrator::dashboard::api::DashboardApi;

pub struct AppState {
    pub api: DashboardApi,
}

pub fn create_router(db_path: &str) -> Router {
    let api = DashboardApi::new(db_path).expect("Failed to create API");
    let state = Arc::new(Mutex::new(AppState { api }));
    
    Router::new()
        .route("/health", get(health))
        .route("/api/overview", get(get_overview))
        .route("/api/projects/:id", get(get_project))
        .route("/api/projects/:id/branches", get(get_branches))
        .route("/api/projects/:id/tasks", get(get_tasks))
        .route("/api/users", get(get_users).post(create_user))
        .route("/api/users/:id", get(get_user).put(update_user).delete(delete_user))
        .with_state(state)
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn get_overview(State(state): State<Arc<Mutex<AppState>>>) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.get_overview() {
        Ok(overview) => Json(serde_json::to_value(overview).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn get_project(
    State(state): State<Arc<Mutex<AppState>>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.get_project(id) {
        Ok(project) => Json(project),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn get_branches(
    State(state): State<Arc<Mutex<AppState>>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.get_branches(id) {
        Ok(branches) => Json(branches),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn get_tasks(
    State(state): State<Arc<Mutex<AppState>>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.get_tasks(id) {
        Ok(tasks) => Json(tasks),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn get_users(State(state): State<Arc<Mutex<AppState>>>) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.get_all_users() {
        Ok(users) => Json(serde_json::to_value(users).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn get_user(
    State(state): State<Arc<Mutex<AppState>>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.get_user(id) {
        Ok(user) => Json(serde_json::to_value(user).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn create_user(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<CreateUser>,
) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.create_user(&payload.name, &payload.email, &payload.category) {
        Ok(user) => Json(serde_json::to_value(user).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn update_user(
    State(state): State<Arc<Mutex<AppState>>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<UpdateUser>,
) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.update_user(id, &payload.name, &payload.email) {
        Ok(_) => Json(serde_json::json!({"success": true})),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn delete_user(
    State(state): State<Arc<Mutex<AppState>>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> StatusCode {
    let state = state.lock().await;
    match state.api.delete_user(id) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[derive(serde::Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub category: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateUser {
    pub name: String,
    pub email: String,
}
