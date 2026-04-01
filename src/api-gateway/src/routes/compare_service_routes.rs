use actix_web::web;

use crate::handlers::compare_handler;

pub fn compare_routes(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/compare-service/popular-low-rated",
        web::get().to(compare_handler::popular_low_rated),
    );
    cfg.route(
        "/compare-service/hidden-gems",
        web::get().to(compare_handler::hidden_gems),
    );
    cfg.route(
        "/compare-service/correlation",
        web::get().to(compare_handler::correlation),
    );
    cfg.route(
        "/compare-service/publishing-growth",
        web::get().to(compare_handler::publishing_growth),
    );
    cfg.route(
        "/compare-service/eras",
        web::get().to(compare_handler::eras),
    );
}
