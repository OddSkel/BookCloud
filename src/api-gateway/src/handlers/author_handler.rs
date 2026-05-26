use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;

use crate::grpc::{AuthorAddPayload, GrpcRegistry};

#[derive(Deserialize)]
pub struct UpdateAuthorQuery {
    #[serde(alias = "Id", alias = "id", alias = "AuthorId")]
    pub author_id: i64,
    #[serde(alias = "Name")]
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct GetAuthorsRequest {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn get_authors(
    registry: web::Data<GrpcRegistry>,
    page: web::Query<GetAuthorsRequest>,
) -> impl Responder {
    let q = page.into_inner();
    match registry.get_authors(q.page, q.page_size).await {
        Ok(authors) => HttpResponse::Ok().json(authors),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn get_author(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<i64>,
) -> impl Responder {
    match registry.get_author(path.into_inner()).await {
        Ok(author) => HttpResponse::Ok().json(author),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn add_author(
    registry: web::Data<GrpcRegistry>,
    body: web::Json<AuthorAddPayload>,
) -> impl Responder {
    match registry.add_author(body.into_inner()).await {
        Ok(author) => HttpResponse::Created().json(author),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn update_author(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<UpdateAuthorQuery>,
) -> impl Responder {
    let q = query.into_inner();

    if q.name.is_none() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "At least one field (name) must be provided" }));
    }

    match registry.update_author(q.author_id, q.name).await {
        Ok(authors) => HttpResponse::Ok().json(authors),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn delete_author(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<i64>,
) -> impl Responder {
    match registry.delete_author(path.into_inner()).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => crate::utils::map_error(e),
    }
}
