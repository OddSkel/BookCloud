use actix_web::web;
use crate::handlers::book_search_handler::book_search;

pub fn author_analytics_routes(cfg: &mut web::ServiceConfig) {

    cfg.route("/book_search", web::get().to(book_search))
    
}
