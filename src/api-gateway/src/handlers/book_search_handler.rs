use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;

use crate::grpc::GrpcRegistry;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub author:   Option<String>,
    pub title:     Option<String>,
    pub keywords: Option<String>,
}

pub async fn book_search(
    registry: web::Data<GrpcRegistry>,
    query:    web::Query<SearchQuery>,
) -> impl Responder {
    match registry.book_search(query.into_inner()).await {
        Ok(json_value) => HttpResponse::Ok().json(json_value),
        Err(e)         => HttpResponse::InternalServerError().json(
            serde_json::json!({ "error": e })
        ),
    }
}
