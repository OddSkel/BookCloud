use actix_web::web;

use self::{
    author_analytics_routes::author_analytics_routes,
    author_catalog_routes::author_routes,
    book_catalog_routes::book_routes,
    book_recommendation_route::book_recommendation_routes,
    book_search_route::book_search_routes,
    compare_service_routes::compare_routes,
    health_routes::health_routes,
    genre_analysis_routes::genre_analysis_routes,
    rating_catalog_routes::rating_routes,
};

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
            .configure(health_routes)
            .configure(author_routes)
            .configure(book_routes)
            .configure(book_recommendation_routes)
            .configure(book_search_routes)
            .configure(rating_routes)
            .configure(compare_routes)
            .configure(genre_analysis_routes)
            .configure(author_analytics_routes),
    );
}