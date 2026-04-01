use actix_web::web;
use crate::handlers::genre_analysis_handler;

pub fn genre_analysis_routes(cfg: &mut web::ServiceConfig) {
    cfg
        .route("/genres", web::get().to(genre_analysis_handler::get_genre));
        .route("/genres", web::post().to(genre_analysis_handler::add_genre));
        .route("/genres/{id}", web::get().to(genre_analysis_handler::get_genre_by_id));
        .route("/genres/{id}", web::put().to(genre_analysis_handler::update_genre));
        .route("/genres/{id}", web::delete().to(genre_analysis_handler::delete_genre));
        .route("/genres/{id}/growth", web::get().to(genre_analysis_handler::get_books_by_genre));
        .route("/genres/{id}/popularity", web::get().to(genre_analysis_handler::get_genre_growth));
}