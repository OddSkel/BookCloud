use actix_web::web;
use crate::handlers::health_handler;
pub fn health_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_handler::health));
}