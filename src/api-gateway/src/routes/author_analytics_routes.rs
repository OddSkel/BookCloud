use actix_web::web;
use crate::handlers::author_analytics_handler::{
    author_performance, authors_consistency, authors_growth, rank_authors,
};

pub fn author_analytics_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/author-analytics")
            .route("/rank",        web::get().to(rank_authors))
            .route("/performance", web::get().to(author_performance))
            .route("/consistency", web::get().to(authors_consistency))
            .route("/growth",      web::get().to(authors_growth)),
    );
}