use actix_web::web;

use self::{
    author_catalog_routes::author_routes,
    book_catalog_routes::book_routes,
    book_recommendation_route::book_recommendation_routes,
    book_search_route::book_search_routes,
    compare_service_routes::compare_routes,
    health_routes::health_routes,
    genre_analysis_routes::genre_analysis_routes,
    rating_catalog_routes::rating_routes,
    author_analytics_routes::author_analytics_routes, 
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
    health_routes(cfg);
    author_routes(cfg);
    book_routes(cfg);
    book_recommendation_routes(cfg);
    book_search_routes(cfg);
    rating_routes(cfg);
    compare_routes(cfg);
    genre_analysis_routes(cfg);
    author_analytics_routes(cfg);
}
