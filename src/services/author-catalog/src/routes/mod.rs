pub mod health_routes;

use actix_web::web;
use self::health_routes::health_routes;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    health_routes(cfg);
}