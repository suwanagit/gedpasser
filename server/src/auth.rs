//! Profiles, secrets and sessions.
//!
//! Secrets (password or PIN) are hashed with Argon2id. A session is a random
//! 32-byte token in an HttpOnly cookie; only its SHA-256 is stored. Secrets
//! and tokens are never logged.

use std::sync::{Arc, OnceLock};

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use sha2::{Digest, Sha256};

use crate::AppState;
use crate::api::ApiError;
use crate::content::hex;

pub const COOKIE_NAME: &str = "ged_session";
pub const SESSION_DAYS: i64 = 30;
pub const MIN_SECRET_LEN: usize = 4;
pub const MAX_NAME_LEN: usize = 30;

/// Hash a secret with Argon2id (random salt). CPU-heavy: call via
/// `spawn_blocking`.
pub fn hash_secret(secret: &str) -> Result<String, String> {
    Argon2::default()
        .hash_password(secret.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| format!("hashing failed: {e}"))
}

/// Check a secret against a stored hash. CPU-heavy: call via `spawn_blocking`.
pub fn verify_secret(secret: &str, stored: &str) -> bool {
    match PasswordHash::new(stored) {
        Ok(parsed) => Argon2::default().verify_password(secret.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

/// A real hash to verify against when the profile doesn't exist, so an
/// unknown profile takes as long to reject as a wrong secret.
pub fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_secret("not-a-real-secret").expect("hash dummy secret"))
}

pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS random number generator failed");
    hex(&bytes)
}

pub fn token_hash(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

/// `Set-Cookie` value that stores a new session token.
pub fn session_cookie(token: &str) -> HeaderValue {
    // TODO(Phase 6): add `Secure` once the site is served over HTTPS.
    let max_age = SESSION_DAYS * 24 * 60 * 60;
    HeaderValue::from_str(&format!("{COOKIE_NAME}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}"))
        .expect("token is hex")
}

/// `Set-Cookie` value that removes the session cookie.
pub fn clear_cookie() -> HeaderValue {
    HeaderValue::from_static("ged_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0")
}

pub fn set_cookie_headers(value: HeaderValue) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(SET_COOKIE, value);
    h
}

/// The session token from the request's `Cookie` header, if any.
pub fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE_NAME)
        .map(|(_, v)| v.to_string())
        .filter(|v| !v.is_empty())
}

/// Create a session row for `user_id` and return the cookie token.
pub async fn start_session(state: &AppState, user_id: i64) -> Result<String, ApiError> {
    let token = new_token();
    sqlx::query("DELETE FROM sessions WHERE expires_at <= unixepoch()").execute(&state.pool).await?;
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES (?, ?, unixepoch() + ?)")
        .bind(token_hash(&token))
        .bind(user_id)
        .bind(SESSION_DAYS * 24 * 60 * 60)
        .execute(&state.pool)
        .await?;
    Ok(token)
}

/// The logged-in learner. Handlers that take this reject requests without
/// a valid session with 401.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: i64,
    pub display_name: String,
}

impl FromRequestParts<Arc<AppState>> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        let token = token_from_headers(&parts.headers).ok_or_else(not_logged_in)?;
        let row: Option<(i64, String)> = sqlx::query_as(
            "SELECT u.id, u.display_name FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = ? AND s.expires_at > unixepoch()",
        )
        .bind(token_hash(&token))
        .fetch_optional(&state.pool)
        .await?;
        let (id, display_name) = row.ok_or_else(not_logged_in)?;
        Ok(CurrentUser { id, display_name })
    }
}

fn not_logged_in() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, "not logged in")
}
