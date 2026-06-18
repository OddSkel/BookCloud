use actix_web::middleware::from_fn;
use actix_web::web;

use crate::auth::require_any_role;

use self::{
    auth_routes::auth_routes, author_analytics_routes::author_analytics_routes,
    author_catalog_routes::author_routes, book_catalog_routes::book_routes,
    book_recommendation_route::book_recommendation_routes, book_search_route::book_search_routes,
    compare_service_routes::compare_routes, genre_analysis_routes::genre_analysis_routes,
    health_routes::health_routes, rating_catalog_routes::rating_routes,
};

pub mod auth_routes;
pub mod author_analytics_routes;
pub mod author_catalog_routes;
pub mod book_catalog_routes;
pub mod book_recommendation_route;
pub mod book_search_route;
pub mod compare_service_routes;
pub mod genre_analysis_routes;
pub mod health_routes;
pub mod rating_catalog_routes;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // ── Público ───────────────────────────────────────
            .configure(health_routes)
            .configure(auth_routes)
            // ── Tudo autenticado num único scope ──────────────
            .service(
                web::scope("")
                    .wrap(from_fn(require_any_role))
                    // Serviços analytics/search/etc
                    .configure(book_search_routes)
                    .configure(book_recommendation_routes)
                    .configure(compare_routes)
                    .configure(genre_analysis_routes)
                    .configure(author_analytics_routes)
                    // Books
                    .configure(book_routes)
                    // Authors
                    .configure(author_routes)
                    // Ratings
                    .configure(rating_routes),
            ),
    );
}
