use crate::handlers::health_handler::health;
use actix_web::web;

pub fn health_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/health").route("", web::get().to(health)));
}
