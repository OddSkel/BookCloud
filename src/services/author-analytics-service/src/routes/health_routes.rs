use crate::handlers::health_handler;
use actix_web::web;
pub fn health_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_handler::health));
}
