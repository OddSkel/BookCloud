use actix_web::middleware::from_fn;
use actix_web::web;

use crate::auth::require_any_role;
use crate::handlers::{author_handler, book_handler, rating_handler};

use self::{
    author_analytics_routes::author_analytics_routes,
    author_catalog_routes::author_routes,
    book_recommendation_route::book_recommendation_routes,
    book_search_route::book_search_routes,
    compare_service_routes::compare_routes,
    genre_analysis_routes::genre_analysis_routes,
    health_routes::health_routes,
    rating_catalog_routes::rating_routes,
};

pub mod author_analytics_routes;
pub mod author_catalog_routes;
pub mod book_catalog_routes;
pub mod book_recommendation_route;
pub mod book_search_route;
pub mod compare_service_routes;
pub mod genre_analysis_routes;
pub mod health_routes;
pub mod rating_catalog_routes;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // ── Público ───────────────────────────────────────
            .configure(health_routes)
            // ── Tudo autenticado num único scope ──────────────
            .service(
                web::scope("")
                    .wrap(from_fn(require_any_role))
                    // Serviços analytics/search/etc
                    .configure(book_search_routes)
                    .configure(book_recommendation_routes)
                    .configure(compare_routes)
                    .configure(genre_analysis_routes)
                    .configure(author_analytics_routes)
                    // Books
                    .service(
                        web::resource("/books")
                            .route(web::get().to(book_handler::get_books)),
                    )
                    .service(
                        web::resource("/book")
                            .route(web::post().to(book_handler::add_book))
                            .route(web::put().to(book_handler::update_book)),
                    )
                    .service(
                        web::resource("/book/{book_isbn}")
                            .route(web::get().to(book_handler::get_book))
                            .route(web::delete().to(book_handler::delete_book)),
                    )
                    // Authors
                    .service(
                        web::resource("/authors")
                            .route(web::get().to(author_handler::get_authors)),
                    )
                    .service(
                        web::resource("/author/{author_id}")
                            .route(web::get().to(author_handler::get_author))
                            .route(web::delete().to(author_handler::delete_author)),
                    )
                    // Ratings
                    .service(
                        web::resource("/ratings")
                            .route(web::get().to(rating_handler::get_ratings)),
                    )
                    .service(
                        web::resource("/rating/{isbn}")
                            .route(web::get().to(rating_handler::get_rating)),
                    )
                    .service(
                        web::resource("/rating")
                            .route(web::post().to(rating_handler::add_rating)),
                    ),
            ),
    );
}
