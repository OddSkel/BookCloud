// src/routes/auth_routes.rs
use crate::handlers::auth_handler;
use actix_web::web;

pub fn auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(auth_handler::login))
            .route("/logout", web::post().to(auth_handler::logout))
            .route("/register", web::post().to(auth_handler::register)),
    );
}
