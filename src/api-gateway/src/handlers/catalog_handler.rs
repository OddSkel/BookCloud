use actix_web::{HttpResponse, Responder, http::StatusCode, web};

use crate::grpc::{CatalogService, GrpcRegistry};

pub async fn book_catalog_status(registry: web::Data<GrpcRegistry>) -> impl Responder {
    service_status_response(CatalogService::BookCatalog, registry).await
}

pub async fn author_catalog_status(registry: web::Data<GrpcRegistry>) -> impl Responder {
    service_status_response(CatalogService::AuthorCatalog, registry).await
}

pub async fn rating_catalog_status(registry: web::Data<GrpcRegistry>) -> impl Responder {
    service_status_response(CatalogService::RatingCatalog, registry).await
}

pub async fn genre_analysis_status(registry: web::Data<GrpcRegistry>) -> impl Responder {
    service_status_response(CatalogService::GenreAnalysis, registry).await
}

async fn service_status_response(
    service: CatalogService,
    registry: web::Data<GrpcRegistry>,
) -> HttpResponse {
    let status = registry.health_status(service).await;
    let response_status = if status.connected {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    HttpResponse::build(response_status).json(serde_json::json!({
        "transport": "grpc",
        "status": status
    }))
}
