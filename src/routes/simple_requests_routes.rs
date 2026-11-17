// The simple-requests pack's routes (plugged in with one `.merge(...)` line in routes/mod.rs).
use crate::handlers::simple_requests_handler::{
    get_params, get_question, post_body_data, protected_route,
};
use crate::state::AppState;
use axum::Router;
use axum::routing::{get, post};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/protected-enter", get(protected_route))
        .route("/params/{param_1}/another_p/{param_2}", get(get_params)) // localhost/params/1/another_p/textTest
        .route("/question_separator", get(get_question)) // localhost/question_separator?name=Jack&age=25&active=true
        .route("/body-data", post(post_body_data))
}
