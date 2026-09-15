use crate::domain::cache::CacheValue;
use crate::engine::cache_engine::{
    create_entry_call, delete_entry_call, get_entry_call, update_entry_call,
};
use crate::state::AppState;
use axum::response::{IntoResponse, Response};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::{error, info};

pub async fn create_cache_entry(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<CacheValue>,
) -> Response {
    info!("create_cache_entry called for key: {}", key);
    match create_entry_call(State(state), key.clone(), payload.value).await {
        Ok(true) => (StatusCode::CREATED, "CREATED").into_response(),
        Ok(false) => (
            StatusCode::CONFLICT,
            format!("key '{}' already exists", key),
        )
            .into_response(),
        Err(err) => {
            error!("Error occurred: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to create cache entry: {}", err),
            )
                .into_response()
        }
    }
}

pub async fn get_cache_entry(State(state): State<AppState>, Path(key): Path<String>) -> Response {
    info!("get_cache_entry called for key: {}", key);
    match get_entry_call(State(state), key.clone()).await {
        Ok(Some(value)) => (StatusCode::OK, Json(CacheValue { value })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, format!("key '{}' not found", key)).into_response(),
        Err(err) => {
            error!("Error occurred: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to read cache entry: {}", err),
            )
                .into_response()
        }
    }
}

pub async fn update_cache_entry(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<CacheValue>,
) -> Response {
    info!("update_cache_entry called for key: {}", key);
    match update_entry_call(State(state), key.clone(), payload.value).await {
        Ok(true) => (StatusCode::OK, "UPDATED").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, format!("key '{}' not found", key)).into_response(),
        Err(err) => {
            error!("Error occurred: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to update cache entry: {}", err),
            )
                .into_response()
        }
    }
}

pub async fn delete_cache_entry(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    info!("delete_cache_entry called for key: {}", key);
    match delete_entry_call(State(state), key.clone()).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, format!("key '{}' not found", key)).into_response(),
        Err(err) => {
            error!("Error occurred: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to delete cache entry: {}", err),
            )
                .into_response()
        }
    }
}
