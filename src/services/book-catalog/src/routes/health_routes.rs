use actix_web::web;
use crate::handlers::health_handler::health;

pub fn health_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/health")
            .route("", web::get().to(health))
    );
}