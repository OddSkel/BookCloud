use actix_web::HttpResponse;
use serde_json::json;

pub fn map_error(e: String) -> HttpResponse {
    let lower = e.to_lowercase();
    let body = serde_json::json!({ "error": e });
    if lower.contains("not found") {
        HttpResponse::NotFound().json(body)
    } else if lower.contains("invalid") || lower.contains("missing") || lower.contains("bad") {
        HttpResponse::BadRequest().json(body)
    } else {
        HttpResponse::InternalServerError().json(body)
    }
}

pub fn map_genre_error(error: String) -> HttpResponse {
    let status = if error.to_ascii_lowercase().contains("not found") {
        actix_web::http::StatusCode::NOT_FOUND
    } else if error.to_ascii_lowercase().contains("invalid")
        || error.to_ascii_lowercase().contains("required")
    {
        actix_web::http::StatusCode::UNPROCESSABLE_ENTITY
    } else {
        actix_web::http::StatusCode::BAD_GATEWAY
    };

    HttpResponse::build(status).json(json!({ "error": error }))
}
