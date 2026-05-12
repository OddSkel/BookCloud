use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use crate::grpc::GrpcRegistry;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub genre: Option<String>,
    pub rating: Option<String>,
    pub popularity: Option<String>,
}

pub async fn book_recommendation(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    match registry.get_book_recommendation(query.into_inner()).await {
        Ok(json_value) => HttpResponse::Ok().json(json_value),
        Err(e) => crate::utils::map_error(e),
    }
}
