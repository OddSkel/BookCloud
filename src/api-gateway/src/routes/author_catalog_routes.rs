use crate::handlers::author_handler;
use actix_web::web;

pub fn author_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/authors", web::get().to(author_handler::get_authors))
        .route("/author", web::post().to(author_handler::add_author))
        .route("/author", web::put().to(author_handler::update_author))
        .route(
            "/author/{author_id}",
            web::get().to(author_handler::get_author),
        )
        .route(
            "/author/{author_id}",
            web::delete().to(author_handler::delete_author),
        );
}
