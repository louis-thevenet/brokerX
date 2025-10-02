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
        .routes(routes!(get_user, put_user, post_user))
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
/// Create or update user by UUID
///
/// Create a new user with the specified UUID, or update an existing user.
/// For creation, all fields (firstname, surname, email, password) are required.
/// For updates, all fields are optional and only provided fields will be updated.
#[utoipa::path(
    put, 
    path = "/{user_id}", 
    params(
        ("user_id" = Uuid, Path, description = "User UUID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = User),
        (status = 201, description = "User created successfully", body = User),
        (status = 400, description = "Invalid request data or missing required fields for creation"),
        (status = 500, description = "Internal server error")
    ), 
    tag = super::USER_TAG
)]
async fn put_user(
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
            .is_ok_and(|o| o.is_some_and(|u| u.id.is_some_and(|id| id != user_id)))
        {
            return (StatusCode::BAD_REQUEST, "Email already in use").into_response();
        }
    }

    let (user, is_creation) = match user_repo.get(&user_id) {
        Ok(Some(user)) => {
            // Update existing user
            let mut updated_user = user;
            if let Some(firstname) = payload.firstname {
                updated_user.firstname = firstname;
            }
            if let Some(surname) = payload.surname {
                updated_user.surname = surname;
            }
            if let Some(email) = payload.email {
                updated_user.email = email;
            }
            if let Some(password) = payload.password {
                if let Err(e) = updated_user.update_password(&password) {
                    return (StatusCode::BAD_REQUEST, format!("Password error: {e}"))
                        .into_response();
                }
            }
            (updated_user, false) // false = not a creation, it's an update
        }
        Ok(None) => {
            // Create new user - all required fields must be provided for creation
            let Some(firstname) = payload.firstname else {
                return (
                    StatusCode::BAD_REQUEST,
                    "firstname is required for user creation",
                )
                    .into_response();
            };
            let Some(surname) = payload.surname else {
                return (
                    StatusCode::BAD_REQUEST,
                    "surname is required for user creation",
                )
                    .into_response();
            };
            let Some(email) = payload.email else {
                return (
                    StatusCode::BAD_REQUEST,
                    "email is required for user creation",
                )
                    .into_response();
            };
            let Some(password) = payload.password else {
                return (
                    StatusCode::BAD_REQUEST,
                    "password is required for user creation",
                )
                    .into_response();
            };
            let mut new_user = match User::new(email, password, firstname, surname, 0.0) {
                Ok(new_user) => new_user,
                Err(e) => {
                    return (StatusCode::BAD_REQUEST, format!("User creation error: {e}"))
                        .into_response();
                }
            };
            new_user.id = Some(user_id);

            match user_repo.insert(user_id, new_user.clone()) {
                Ok(()) => (new_user, true),

                Err(e) => {
                    return (StatusCode::BAD_REQUEST, format!("User creation error: {e}"))
                        .into_response();
                }
            }
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let status = if is_creation {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    (status, Json(user)).into_response()
}

/// Create a new user
///
/// Create a new user. All fields (firstname, surname, email, password) are required.
#[utoipa::path(
    post, 
    path = "/", 
    request_body = UpdateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = User),
        (status = 400, description = "Invalid request data or missing required fields for creation"),
        (status = 500, description = "Internal server error")
    ), 
    tag = super::USER_TAG
)]
async fn post_user(
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    let broker = state.broker();
    let mut user_repo = broker.get_user_repo();

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

            let Some(firstname) = payload.firstname else {
                return (
                    StatusCode::BAD_REQUEST,
                    "firstname is required for user creation",
                )
                    .into_response();
            };
            let Some(surname) = payload.surname else {
                return (
                    StatusCode::BAD_REQUEST,
                    "surname is required for user creation",
                )
                    .into_response();
            };
            let Some(email) = payload.email else {
                return (
                    StatusCode::BAD_REQUEST,
                    "email is required for user creation",
                )
                    .into_response();
            };
            let Some(password) = payload.password else {
                return (
                    StatusCode::BAD_REQUEST,
                    "password is required for user creation",
                )
                    .into_response();
            };
            let user_id = match user_repo.create_user(email, password, firstname, surname, 0.0) {
                Ok(id) => id,
                Err(e) => {
                    return (StatusCode::BAD_REQUEST, format!("User creation error: {e}"))
                        .into_response();
                }
            };


let Ok(Some(user)) = user_repo.get(&user_id) else {
                return (StatusCode::INTERNAL_SERVER_ERROR, "User retrieval error after creation").into_response();
            };

    (StatusCode::CREATED, Json(user)).into_response()
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
