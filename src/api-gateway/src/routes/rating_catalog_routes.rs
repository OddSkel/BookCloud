use actix_web::web;
use crate::handlers::rating_handler;

pub fn rating_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/ratings/{book_id}",    web::get().to(rating_handler::get_ratings))
        .route("/rating/{book_id}",     web::post().to(rating_handler::add_rating))
        .route("/rating/{book_id}",     web::put().to(rating_handler::update_rating))
        .route("/rating/{rating_id}",   web::get().to(rating_handler::get_rating))
        .route("/rating/{rating_id}",   web::delete().to(rating_handler::delete_rating));
}
