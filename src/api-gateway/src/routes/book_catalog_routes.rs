use crate::handlers::book_handler;
use actix_web::web;

pub fn book_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/books", web::get().to(book_handler::get_books))
        .route("/book", web::post().to(book_handler::add_book))
        .route("/book", web::put().to(book_handler::update_book))
        .route("/book/{book_isbn}", web::get().to(book_handler::get_book))
        .route(
            "/book/{book_isbn}",
            web::delete().to(book_handler::delete_book),
        );
}
