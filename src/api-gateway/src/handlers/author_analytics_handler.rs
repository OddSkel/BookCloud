use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::grpc::GrpcRegistry;

#[derive(Deserialize)]
pub struct RankQuery {
    pub sort: Option<String>, // "avg_rating" | "total_ratings" (default)
}

#[derive(Deserialize)]
pub struct PerformanceQuery {
    pub author_name:   Option<String>,
    pub author_id:     Option<i32>,
    pub pub_year_from: Option<i32>,
    pub pub_year_to:   Option<i32>,
}

#[derive(Deserialize)]
pub struct AuthorQuery {
    pub author_name: Option<String>,
    pub author_id:   Option<i32>,
}

pub async fn rank_authors(
    registry: web::Data<GrpcRegistry>,
    query:    web::Query<RankQuery>,
) -> impl Responder {
    let sort = query.sort.as_deref().unwrap_or("total_ratings");

    // if sort == "avg_rating" send Some(0.0) to signal that field, else use total
    let (avg_rating, total_ratings) = if sort == "avg_rating" {
        (Some(0.0_f64), None)
    } else {
        (None, Some(0_i64))
    };

    match registry.rank_authors(avg_rating, total_ratings).await {
        Ok(authors) => HttpResponse::Ok().json(authors),
        Err(e)      => crate::utils::map_error(e),
    }
}

pub async fn author_performance(
    registry: web::Data<GrpcRegistry>,
    query:    web::Query<PerformanceQuery>,
) -> impl Responder {
    match registry.author_performance(
        query.author_name.clone(),
        query.author_id,
        query.pub_year_from,
        query.pub_year_to,
    ).await {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e)      => crate::utils::map_error(e),
    }
}

pub async fn authors_consistency(
    registry: web::Data<GrpcRegistry>,
    query:    web::Query<AuthorQuery>,
) -> impl Responder {
    match registry.authors_consistency(
        query.author_name.clone(),
        query.author_id,
    ).await {
        Ok(authors) => HttpResponse::Ok().json(authors),
        Err(e)      => crate::utils::map_error(e),
    }
}

pub async fn authors_growth(
    registry: web::Data<GrpcRegistry>,
    query:    web::Query<AuthorQuery>,
) -> impl Responder {
    match registry.authors_growth(
        query.author_name.clone(),
        query.author_id,
    ).await {
        Ok(authors) => HttpResponse::Ok().json(authors),
        Err(e)      => crate::utils::map_error(e),
    }
}
