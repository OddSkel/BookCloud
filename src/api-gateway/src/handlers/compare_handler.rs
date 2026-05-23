use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;

use crate::grpc::{CompareFiltersPayload, GrpcRegistry};

#[derive(Deserialize, Clone, Default)]
pub struct CompareQuery {
    pub author_name: Option<String>,
    pub author_id: Option<i64>,
    #[serde(default)]
    pub genre_name: Vec<String>,
    #[serde(default)]
    pub genre_id: Vec<i64>,
    pub book_name: Option<String>,
    pub book_isbn: Option<i64>,
    pub pub_year_from: Option<i32>,
    pub pub_year_to: Option<i32>,
    pub pub_year: Option<i32>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub min_num_ratings: Option<i64>,
    pub max_num_ratings: Option<i64>,
    pub min_star_rating: Option<f64>,
    pub max_star_rating: Option<f64>,
    pub method: Option<String>,
    pub classic_threshold: Option<i32>,
    pub modern_threshold: Option<i32>,
}

impl From<CompareQuery> for CompareFiltersPayload {
    fn from(value: CompareQuery) -> Self {
        Self {
            author_name: value.author_name,
            author_id: value.author_id,
            genre_name: value.genre_name,
            genre_id: value.genre_id,
            book_name: value.book_name,
            book_isbn: value.book_isbn,
            pub_year_from: value.pub_year_from,
            pub_year_to: value.pub_year_to,
            pub_year: value.pub_year,
            page: value.page,
            page_size: value.page_size,
            min_num_ratings: value.min_num_ratings,
            max_num_ratings: value.max_num_ratings,
            min_star_rating: value.min_star_rating,
            max_star_rating: value.max_star_rating,
            method: value.method,
            classic_threshold: value.classic_threshold,
            modern_threshold: value.modern_threshold,
        }
    }
}

pub async fn popular_low_rated(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<CompareQuery>,
) -> impl Responder {
    match registry
        .get_popular_low_rated(query.into_inner().into())
        .await
    {
        Ok(payload) => HttpResponse::Ok().json(payload),
        Err(e) => {
            let safe = if e.len() > 200 {
                "Upstream service error".to_string()
            } else {
                e
            };
            crate::utils::map_error(safe)
        }
    }
}

pub async fn hidden_gems(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<CompareQuery>,
) -> impl Responder {
    match registry.get_hidden_gems(query.into_inner().into()).await {
        Ok(payload) => HttpResponse::Ok().json(payload),
        Err(e) => {
            let safe = if e.len() > 200 {
                "Upstream service error".to_string()
            } else {
                e
            };
            crate::utils::map_error(safe)
        }
    }
}

pub async fn correlation(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<CompareQuery>,
) -> impl Responder {
    match registry.get_correlation(query.into_inner().into()).await {
        Ok(payload) => HttpResponse::Ok().json(payload),
        Err(e) => {
            let safe = if e.len() > 200 {
                "Upstream service error".to_string()
            } else {
                e
            };
            crate::utils::map_error(safe)
        }
    }
}

pub async fn publishing_growth(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<CompareQuery>,
) -> impl Responder {
    match registry
        .get_publishing_growth(query.into_inner().into())
        .await
    {
        Ok(payload) => HttpResponse::Ok().json(payload),
        Err(e) => {
            let safe = if e.len() > 200 {
                "Upstream service error".to_string()
            } else {
                e
            };
            crate::utils::map_error(safe)
        }
    }
}

pub async fn eras(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<CompareQuery>,
) -> impl Responder {
    match registry.get_eras(query.into_inner().into()).await {
        Ok(payload) => HttpResponse::Ok().json(payload),
        Err(e) => {
            let safe = if e.len() > 200 {
                "Upstream service error".to_string()
            } else {
                e
            };
            crate::utils::map_error(safe)
        }
    }
}
