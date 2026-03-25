pub mod health_routes;

use self::health_routes::health_routes;
use actix_web::web;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    health_routes(cfg);
}
