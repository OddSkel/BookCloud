use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::Deserialize;
use serde_json::json;

use crate::grpc::{GrpcRegistry, RatingAddPayload};
use crate::auth::{has_role, ROLE_ADMIN};

#[derive(Deserialize)]
pub struct GetRatingsQuery {
    pub page_number: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn get_ratings(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<GetRatingsQuery>,
) -> impl Responder {
    let q = query.into_inner();

    match registry.get_ratings(q.page_number, q.page_size).await {
        Ok(ratings) => HttpResponse::Ok().json(ratings),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn get_rating(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
) -> impl Responder {
    match registry.get_rating(&path.into_inner()).await {
        Ok(rating) => HttpResponse::Ok().json(rating),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn add_rating(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
    body: web::Json<RatingAddPayload>,
    req: HttpRequest,
) -> impl Responder {
    if !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden()
            .json(json!({"error": "Requires 'admin' role"}));
    }
    match registry
        .add_rating(&path.into_inner(), body.into_inner())
        .await
    {
        Ok(rating) => HttpResponse::Created().json(rating),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn update_rating(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
    body: web::Json<RatingAddPayload>,
) -> impl Responder {
    let payload = body.into_inner();

    match registry
        .update_rating(&path.into_inner(), payload.num_ratings, payload.star_rating)
        .await
    {
        Ok(ratings) => HttpResponse::Ok().json(ratings),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn delete_rating(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
) -> impl Responder {
    match registry.delete_rating(&path.into_inner()).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => crate::utils::map_error(e),
    }
}
