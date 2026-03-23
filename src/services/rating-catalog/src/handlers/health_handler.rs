use actix_web::{HttpResponse, Responder};
use std::env;

pub async fn health() -> impl Responder {
    let service_name = "rating-catalog";

    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": service_name
    }))
}
