use axum::{Json, extract::Path, extract::State, http::StatusCode, response::IntoResponse};
use domain::Repository;
use domain::user::{User, UserRepoExt};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use super::AppState;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firstname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

pub fn router(state: AppState) -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .with_state(state)
        .routes(routes!(get_user, update_user))
        .routes(routes!(get_orders_from_user))
}

/// Get user by UUID
///
/// Get a specific user by their UUID
#[utoipa::path(
    get, 
    path = "/{user_id}", 
    params(
        ("user_id" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "User found", body = User),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid UUID format")
    ), 
    tag = super::USER_TAG
)]
async fn get_user(State(state): State<AppState>, Path(user_id): Path<Uuid>) -> impl IntoResponse {
    match state.broker().get_user_repo().get(&user_id) {
        Ok(Some(user)) => Json(user).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
/// Update user by UUID
///
/// Update a specific user by their UUID
#[utoipa::path(
    put, 
    path = "/{user_id}", 
    params(
        ("user_id" = Uuid, Path, description = "User UUID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = User),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Internal server error")
    ), 
    tag = super::USER_TAG
)]
async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    let broker = state.broker();
    let user_repo = broker.get_user_repo();

    if let Some(ref email) = payload.email {
        if !email.contains('@') {
            return (StatusCode::BAD_REQUEST, "Invalid email format").into_response();
        }
        if user_repo
            .get_user_by_email(email)
            .is_ok_and(|o| o.is_some())
        {
            return (StatusCode::BAD_REQUEST, "Email already in use").into_response();
        }
    }

    let mut user = match user_repo.get(&user_id) {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if let Some(firstname) = payload.firstname {
        user.firstname = firstname;
    }
    if let Some(surname) = payload.surname {
        user.surname = surname;
    }
    if let Some(email) = payload.email {
        user.email = email;
    }
    if let Some(password) = payload.password {
        if let Err(e) = user.update_password(&password) {
            return (StatusCode::BAD_REQUEST, format!("Password error: {e}")).into_response();
        }
    }

    // Save the updated user
    match user_repo.update(user_id, user.clone()) {
        Ok(()) => Json(user).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Get user's orders
///
/// Get orders from a specific user by their UUID
#[utoipa::path(
    get, 
    path = "/{user_id}/orders", 
    params(
        ("user_id" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "User found", body = User),
        (status = 500, description = "Database error"),
    ), 
    tag = super::USER_TAG
)]
async fn get_orders_from_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.broker().get_orders_for_user(&user_id) {
        Ok(orders) => Json(orders).into_response(),
        Err(_e) => StatusCode::INTERNAL_SERVER_ERROR.into_response(), // TODO: be finer here
    }
}
