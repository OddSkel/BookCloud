use actix_web::{HttpResponse, Responder};
use std::env;

pub async fn health() -> impl Responder {
    let service_name = env::var("SERVICE_NAME").unwrap_or_else(|_| "author-catalog".to_string());

    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": service_name
    }))
}
