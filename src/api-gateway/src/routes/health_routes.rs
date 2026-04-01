use crate::handlers::{catalog_handler, health_handler};
use actix_web::web;

pub fn health_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_handler::health))
        .route(
            "/status/bookCatalog",
            web::get().to(catalog_handler::book_catalog_status),
        )
        .route(
            "/status/authorCatalog",
            web::get().to(catalog_handler::author_catalog_status),
        )
        .route(
            "/status/ratingCatalog",
            web::get().to(catalog_handler::rating_catalog_status),
        );
}
