use crate::handlers::rating_handler;
use actix_web::web;

pub fn rating_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/ratings", web::get().to(rating_handler::get_ratings))
        .route(
            "/rating/{book_isbn}",
            web::post().to(rating_handler::add_rating),
        )
        .route(
            "/rating/{book_isbn}",
            web::put().to(rating_handler::update_rating),
        )
        .route(
            "/rating/{book_isbn}",
            web::get().to(rating_handler::get_rating),
        )
        .route(
            "/rating/{book_isbn}",
            web::delete().to(rating_handler::delete_rating),
        );
}
