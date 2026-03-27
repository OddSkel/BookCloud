use actix_web::web;
use crate::handlers::health_handler::health;
use crate::handlers::author_handler;

pub fn health_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/health")
            .route("", web::get().to(health))
    );
}

pub fn get_authors(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/authors")
            .route("", web::get().to(author_handler))
    );
}

pub fn register_author(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/author")
            .route("", web::post().to(author_handler))
    );
}

pub fn edit_author(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/author")
            .route("/{id}", web::put().to(author_handler))
    );
}

pub fn delete_author(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/author")
            .route("/{id}", web::get().to(author_handler))
    );
}

pub fn get_author(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/author")
            .route("/{id}", web::get().to(author_handler))
    );
}