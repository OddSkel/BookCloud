use actix_web::web;
use crate::handlers::book_rec_handler::book_recommendation;

pub fn book_recommendation_routes(cfg: &mut web::ServiceConfig) {

    cfg.route("/book-recommendation", web::get().to(book_recommendation))
    
}
