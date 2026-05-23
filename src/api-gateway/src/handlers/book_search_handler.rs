use crate::grpc::GrpcRegistry;
use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub author: Option<String>,
    pub title: Option<String>,
    pub keywords: Option<String>,
}

pub async fn book_search(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    match registry.book_search(query.into_inner()).await {
        Ok(json_value) => HttpResponse::Ok().json(json_value),
        Err(e) => crate::utils::map_error(e),
    }
}
