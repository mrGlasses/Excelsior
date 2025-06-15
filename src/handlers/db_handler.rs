use crate::domain::database::NewUser;
use crate::engine::db_engine::{create_user_db_call, get_users_db_call};
use crate::state::AppState;
use axum::response::{IntoResponse, Response};
use axum::{extract::State, http::StatusCode, Json};
use tracing::{error, info, warn};

pub async fn get_users(State(state): State<AppState>) -> Response {
    info!("get_users called");
    let vec_users = get_users_db_call(State(state)).await;
    match vec_users {
        Ok(users) => {
            warn!("Users returned");
            (StatusCode::OK, Json(users)).into_response()
        }
        Err(err) => {
            error!("Error occurred: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get users: {}", err).into_response(),
            )
                .into_response()
        }
    }
}

pub async fn create_user(State(state): State<AppState>, Json(payload): Json<NewUser>) -> Response {
    info!("create_user called with params: {:?}", payload);
    match create_user_db_call(State(state), payload.name.clone()).await {
        Ok(_) => {
            warn!("New user created");
            (StatusCode::CREATED, "CREATED").into_response()
        }
        Err(err) => {
            error!("Error occurred: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to insert user: {}", err).into_response(),
            )
                .into_response()
        }
    }
}
