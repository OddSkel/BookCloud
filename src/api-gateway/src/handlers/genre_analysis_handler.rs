use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::Deserialize;
use serde_json::json;

use crate::auth::{ROLE_ADMIN, has_role};
use crate::grpc::{GenreAddPayload, GrpcRegistry};

#[derive(Deserialize)]
pub struct GetGenresQuery {
    pub sort_by: Option<i32>,
    pub ascending: Option<bool>,
    pub page_num: Option<i32>,
    pub page_size: Option<i32>,
}

#[derive(Deserialize)]
pub struct GenreTrendQuery {
    pub year_from: Option<i32>,
    pub year_to: Option<i32>,
}

pub async fn get_genres(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<GetGenresQuery>,
) -> impl Responder {
    let q = query.into_inner();

    match registry
        .get_genres(q.sort_by, q.ascending, q.page_num, q.page_size)
        .await
    {
        Ok(genres) => HttpResponse::Ok().json(genres),
        Err(error) => crate::utils::map_genre_error(error),
    }
}

pub async fn add_genre(
    registry: web::Data<GrpcRegistry>,
    body: web::Json<GenreAddPayload>,
    req: HttpRequest,
) -> impl Responder {
    if !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'admin' role"}));
    }

    match registry.add_genre(body.into_inner()).await {
        Ok(genre) => HttpResponse::Ok().json(genre),
        Err(error) => crate::utils::map_genre_error(error),
    }
}

pub async fn get_genre(registry: web::Data<GrpcRegistry>, path: web::Path<i64>) -> impl Responder {
    match registry.get_genre(path.into_inner()).await {
        Ok(genre) => HttpResponse::Ok().json(genre),
        Err(error) => crate::utils::map_genre_error(error),
    }
}

pub async fn update_genre(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<i64>,
    body: web::Json<GenreAddPayload>,
    req: HttpRequest,
) -> impl Responder {
    if !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'admin' role"}));
    }

    match registry
        .update_genre(path.into_inner(), body.into_inner())
        .await
    {
        Ok(genre) => HttpResponse::Ok().json(genre),
        Err(error) => crate::utils::map_genre_error(error),
    }
}

pub async fn delete_genre(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<i64>,
    req: HttpRequest,
) -> impl Responder {
    if !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'admin' role"}));
    }

    match registry.delete_genre(path.into_inner()).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(error) => crate::utils::map_genre_error(error),
    }
}

pub async fn get_genre_growth(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<i64>,
    query: web::Query<GenreTrendQuery>,
) -> impl Responder {
    let genre_id = path.into_inner();
    let q = query.into_inner();

    match registry
        .get_genre_growth(genre_id, q.year_from, q.year_to)
        .await
    {
        Ok(payload) => HttpResponse::Ok().json(payload),
        Err(error) => crate::utils::map_genre_error(error),
    }
}

pub async fn get_genre_popularity(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<i64>,
    query: web::Query<GenreTrendQuery>,
) -> impl Responder {
    let genre_id = path.into_inner();
    let q = query.into_inner();

    match registry
        .get_genre_popularity(genre_id, q.year_from, q.year_to)
        .await
    {
        Ok(payload) => HttpResponse::Ok().json(payload),
        Err(error) => crate::utils::map_genre_error(error),
    }
}
