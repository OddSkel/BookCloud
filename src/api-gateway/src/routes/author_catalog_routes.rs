use actix_web::web;

use crate::handlers::catalog_handler::author_catalog_status;

pub fn author_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/author-catalog").route("/grpc/health", web::get().to(author_catalog_status)),
    );
}
