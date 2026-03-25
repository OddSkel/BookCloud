use actix_web::web;

use crate::handlers::catalog_handler::book_catalog_status;

pub fn book_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/book-catalog").route("/grpc/health", web::get().to(book_catalog_status)),
    );
}
