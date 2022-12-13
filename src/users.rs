use crate::error::Error;
use crate::error::ResultExt;
use axum::Extension;
use axum::{
    extract::{Json, Path, Query},
    http::StatusCode,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;

pub fn router() -> Router {
    Router::new()
        .route("/user/:name", get(get_user))
        .route("/user", get(get_users))
        .route("/user/:name", put(update_user))
        .route("/user", post(create_user))
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct User {
    username: String,
    email: String,
    bio: String,
}

#[derive(Deserialize)]
struct UserUpdate {
    email: Option<String>,
    bio: Option<String>,
}

#[derive(Deserialize)]
struct Pagination {
    offset: Option<i32>,
    limit: Option<i32>,
}

async fn get_user(
    Extension(pool): Extension<PgPool>,
    Path(name): Path<String>,
) -> Result<Json<User>, Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT username, email, bio 
        FROM users 
        WHERE username = $1
        "#,
    )
    .bind(name)
    .fetch_optional(&pool)
    .await?
    .ok_or(Error::NotFound)?;
    Ok(Json(user))
}

async fn get_users(
    Extension(pool): Extension<PgPool>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<User>>, Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT username, email, bio 
        FROM users 
        OFFSET $1 LIMIT $2
        "#,
    )
    .bind(pagination.offset.unwrap_or_default())
    .bind(pagination.limit.unwrap_or(i32::MAX))
    .fetch_all(&pool)
    .await?;
    Ok(Json(user))
}

async fn create_user(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<User>,
) -> Result<StatusCode, Error> {
    sqlx::query(
        r#"
        INSERT INTO users (username, email, bio) 
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(payload.username)
    .bind(payload.email)
    .bind(payload.bio)
    .execute(&pool)
    .await
    .on_constraint("user_username_key", |_| {
        Error::unprocessable_entity([("username", "already taken")])
    })
    .on_constraint("user_email_key", |_| {
        Error::unprocessable_entity([("email", "already taken")])
    })?;
    Ok(StatusCode::CREATED)
}

async fn update_user(
    Extension(pool): Extension<PgPool>,
    Path(name): Path<String>,
    Json(payload): Json<UserUpdate>,
) -> Result<StatusCode, Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET email = coalesce($1, users.email), bio = coalesce($2, users.bio)
        WHERE username = $3
        returning email, username, bio
        "#,
    )
    .bind(payload.email)
    .bind(payload.bio)
    .bind(name)
    .execute(&pool)
    .await?;
    Ok(StatusCode::ACCEPTED)
}
