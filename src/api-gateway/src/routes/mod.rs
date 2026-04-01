use actix_web::web;

use self::{
    author_catalog_routes::author_routes, book_catalog_routes::book_routes,
    compare_service_routes::compare_routes, health_routes::health_routes,
    genre_analysis_routes::genre_analysis_routes,
    rating_catalog_routes::rating_routes,
};

pub mod author_catalog_routes;
pub mod book_catalog_routes;
pub mod compare_service_routes;
pub mod genre_analysis_routes;
pub mod health_routes;
pub mod rating_catalog_routes;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    health_routes(cfg);
    author_routes(cfg);
    book_routes(cfg);
    rating_routes(cfg);
    compare_routes(cfg);
    genre_analysis_routes(cfg);
}
