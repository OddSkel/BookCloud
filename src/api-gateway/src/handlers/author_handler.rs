use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::Deserialize;
use serde_json::json;

use crate::auth::{has_role, ROLE_ADMIN};
use crate::grpc::{AuthorAddPayload, GrpcRegistry};

#[derive(Deserialize)]
pub struct UpdateAuthorBody {
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

pub async fn get_author(registry: web::Data<GrpcRegistry>, path: web::Path<i64>) -> impl Responder {
    match registry.get_author(path.into_inner()).await {
        Ok(author) => HttpResponse::Ok().json(author),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn add_author(
    registry: web::Data<GrpcRegistry>,
    body: web::Json<AuthorAddPayload>,
    req: HttpRequest,
) -> impl Responder {
    if !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'admin' role"}));
    }
    match registry.add_author(body.into_inner()).await {
        Ok(author) => HttpResponse::Created().json(author),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn update_author_by_path(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<i64>,
    body: web::Json<UpdateAuthorBody>,
    req: HttpRequest,
) -> impl Responder {
    if !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'admin' role"}));
    }

    let author_id = path.into_inner();
    let payload = body.into_inner();

    if payload.name.is_none() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "At least one field (name) must be provided" }));
    }

    match registry.update_author(author_id, payload.name).await {
        Ok(authors) => HttpResponse::Ok().json(authors),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn delete_author(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<i64>,
    req: HttpRequest,
) -> impl Responder {
    if !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'admin' role"}));
    }
    match registry.delete_author(path.into_inner()).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => crate::utils::map_error(e),
    }
}
