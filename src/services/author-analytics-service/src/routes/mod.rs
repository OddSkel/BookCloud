use actix_web::web;
pub mod health_routes;
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    health_routes::health_routes(cfg);
}
