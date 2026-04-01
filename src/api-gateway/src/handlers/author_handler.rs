use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

use crate::grpc::{AuthorAddPayload, GrpcRegistry};

#[derive(Deserialize)]
pub struct UpdateAuthorQuery {
    #[serde(rename = "Id")]
    pub id: i32,
    #[serde(rename = "Name")]
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct GetAuthorsRequest {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn get_authors(
    registry: web::Data<GrpcRegistry>,
    page: web::Query<GetAuthorsRequest>
) -> impl Responder {
    let q = page.into_inner();
    match registry.get_authors(q.page, q.page_size).await {
        Ok(authors) => HttpResponse::Ok().json(authors),
        Err(e)      => HttpResponse::InternalServerError().json(json!({ "error": e })),
    }
}

pub async fn get_author(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
) -> impl Responder {
    match registry.get_author(&path.into_inner()).await {
        Ok(author) => HttpResponse::Ok().json(author),
        Err(e)     => HttpResponse::NotFound().json(json!({ "error": e })),
    }
}

pub async fn add_author(
    registry: web::Data<GrpcRegistry>,
    body: web::Json<AuthorAddPayload>,
) -> impl Responder {
    match registry.add_author(body.into_inner()).await {
        Ok(author) => HttpResponse::Created().json(author),
        Err(e)     => HttpResponse::UnprocessableEntity().json(json!({ "error": e })),
    }
}

pub async fn update_author(
    registry: web::Data<GrpcRegistry>,
    query: web::Query<UpdateAuthorQuery>,
) -> impl Responder {
    let q = query.into_inner();
    match registry.update_author(q.id, q.name).await {
        Ok(authors) => HttpResponse::Ok().json(authors),
        Err(e)      => HttpResponse::UnprocessableEntity().json(json!({ "error": e })),
    }
}

pub async fn delete_author(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
) -> impl Responder {
    match registry.delete_author(&path.into_inner()).await {
        Ok(_)  => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::NotFound().json(json!({ "error": e })),
    }
}
