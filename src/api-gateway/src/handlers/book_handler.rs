use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde_json::json;

use crate::grpc::{BookAddPayload, GrpcRegistry};
use crate::auth::{has_role, ROLE_ADMIN, ROLE_USER}; 

pub async fn get_books(registry: web::Data<GrpcRegistry>) -> impl Responder {
    match registry.get_books(None, None).await {
        Ok(books) => HttpResponse::Ok().json(books),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn get_book(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
) -> impl Responder {
    match registry.get_book(&path.into_inner()).await {
        Ok(book) => HttpResponse::Ok().json(book),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn add_book(
    registry: web::Data<GrpcRegistry>,
    req: HttpRequest,
    body: web::Bytes,
) -> impl Responder {
    if !has_role(&req, ROLE_USER) && !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden()
            .json(json!({"error": "Requires 'user' or 'admin' role"}));
    }
    let payload = match serde_json::from_slice::<BookAddPayload>(&body) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest()
            .json(json!({"error": format!("Invalid JSON: {}", e)})),
    };
    match registry.add_book(payload).await {
        Ok(book) => HttpResponse::Created().json(book),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn update_book(
    registry: web::Data<GrpcRegistry>,
    req: HttpRequest,
    body: web::Bytes, 
) -> impl Responder {
    if !has_role(&req, ROLE_USER) && !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden()
            .json(json!({"error": "Requires 'user' or 'admin' role"}));
    }

    let payload = match serde_json::from_slice::<BookAddPayload>(&body) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Invalid JSON: {}", e)}))
        }
    };

    let isbn = req
        .headers()
        .get("ISBN")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    match registry.update_book(isbn, payload).await {
        Ok(books) => HttpResponse::Ok().json(books),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn delete_book(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    if !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden()
            .json(json!({"error": "Requires 'admin' role"}));
    }
    match registry.delete_book(&path.into_inner()).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => crate::utils::map_error(e),
    }
}
