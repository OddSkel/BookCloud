use crate::handlers::book_search_handler::book_search;
use actix_web::web;

pub fn book_search_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/book-search", web::get().to(book_search));
}
