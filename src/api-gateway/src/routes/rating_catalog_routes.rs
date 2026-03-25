use actix_web::web;

use crate::handlers::catalog_handler::rating_catalog_status;

pub fn rating_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/rating-catalog").route("/grpc/health", web::get().to(rating_catalog_status)),
    );
}
