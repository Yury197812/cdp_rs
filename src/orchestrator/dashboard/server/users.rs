// orchestrator/dashboard/server/users.rs - Users API
use axum::{
    routing::{get, post, put, delete},
    Json,
    extract::State,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::orchestrator::dashboard::api::DashboardApi;
use crate::database::models::User;

pub fn users_routes() -> axum::Router<Arc<Mutex<crate::orchestrator::dashboard::server::AppState>>> {
    axum::Router::new()
        .route("/api/users", get(get_users).post(create_user))
        .route("/api/users/:id", get(get_user).put(update_user).delete(delete_user))
}

async fn get_users(State(state): State<Arc<Mutex<crate::orchestrator::dashboard::server::AppState>>>) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.get_all_users() {
        Ok(users) => Json(serde_json::to_value(users).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn get_user(
    State(state): State<Arc<Mutex<crate::orchestrator::dashboard::server::AppState>>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.get_user(id) {
        Ok(user) => Json(serde_json::to_value(user).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn create_user(
    State(state): State<Arc<Mutex<crate::orchestrator::dashboard::server::AppState>>>,
    Json(payload): Json<CreateUser>,
) -> Json<serde_json::Value> {
    let state = state.lock().await;
    match state.api.create_user(&payload.name, &payload.email, &payload.category) {
        Ok(user) => Json(serde_json::to_value(user).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn update_user(
    State(state): State<Arc<Mutex<crate::orchestrator::dashboard::server::AppState>>>,
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
    State(state): State<Arc<Mutex<crate::orchestrator::dashboard::server::AppState>>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> axum::http::StatusCode {
    let state = state.lock().await;
    match state.api.delete_user(id) {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
