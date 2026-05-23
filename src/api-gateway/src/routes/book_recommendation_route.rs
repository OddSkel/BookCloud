use crate::handlers::book_rec_handler::book_recommendation;
use actix_web::web;

pub fn book_recommendation_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/book-recommendation", web::get().to(book_recommendation));
}
