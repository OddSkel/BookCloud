use actix_web::{
    Error, HttpRequest, HttpResponse,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
};
use serde::Deserialize;
use serde_json::json;

// ── JWT claims we care about ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
struct RealmAccess {
    roles: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct Claims {
    realm_access: Option<RealmAccess>,
}

// ── Role constants ────────────────────────────────────────────────────────────

pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_USER: &str = "user";
pub const ROLE_READONLY: &str = "readonly";

// ── Token extraction + decoding ───────────────────────────────────────────────

fn decode_claims(token: &str) -> Option<Claims> {
    let payload = token.split('.').nth(1)?;
    // JWT payload is base64url-encoded without padding
    let bytes = base64_url_decode(payload).ok()?;
    serde_json::from_slice::<Claims>(&bytes).ok()
}

fn base64_url_decode(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.decode(input)
}

fn extract_bearer(req: &ServiceRequest) -> Option<String> {
    req.headers()
        .get("Authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_owned)
}

fn get_roles(req: &ServiceRequest) -> Vec<String> {
    extract_bearer(req)
        .and_then(|t| decode_claims(&t))
        .and_then(|c| c.realm_access)
        .map(|ra| ra.roles)
        .unwrap_or_default()
}

// ── Middleware factories ──────────────────────────────────────────────────────

/// Requires a valid JWT with ANY of the provided roles.
/// Call the specific helpers below instead of this directly.
async fn check_roles<B: MessageBody + 'static>(
    allowed: &[&str],
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let roles = get_roles(&req);

    // Must have at least one token at all
    if extract_bearer(&req).is_none() {
        return Ok(req.into_response(
            HttpResponse::Unauthorized()
                .json(json!({"error": "Missing or invalid Authorization header"}))
                .map_into_right_body(),
        ));
    }

    if allowed.iter().any(|r| roles.iter().any(|ur| ur == *r)) {
        next.call(req).await.map(|r| r.map_into_left_body())
    } else {
        Ok(req.into_response(
            HttpResponse::Forbidden()
                .json(json!({"error": "Insufficient role"}))
                .map_into_right_body(),
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public middleware functions — use these in routes/mod.rs
// ─────────────────────────────────────────────────────────────────────────────

/// Any authenticated user (readonly / user / admin).
pub async fn require_any_role<B: MessageBody + 'static>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    check_roles(&[ROLE_READONLY, ROLE_USER, ROLE_ADMIN], req, next).await
}

/// user or admin only (not readonly).
pub async fn require_user_role<B: MessageBody + 'static>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    check_roles(&[ROLE_USER, ROLE_ADMIN], req, next).await
}

/// admin only.
pub async fn require_admin_role<B: MessageBody + 'static>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    check_roles(&[ROLE_ADMIN], req, next).await
}

pub fn roles_from_request(req: &HttpRequest) -> Vec<String> {
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(decode_claims)
        .and_then(|c| c.realm_access)
        .map(|ra| ra.roles)
        .unwrap_or_default()
}

pub fn has_role(req: &HttpRequest, role: &str) -> bool {
    roles_from_request(req).iter().any(|r| r == role)
}
