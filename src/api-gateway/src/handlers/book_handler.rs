use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde_json::json;

use crate::auth::{ROLE_ADMIN, ROLE_USER, has_role};
use crate::grpc::{BookAddPayload, GrpcRegistry};

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
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'user' or 'admin' role"}));
    }
    let payload = match serde_json::from_slice::<BookAddPayload>(&body) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Invalid JSON: {}", e)}));
        }
    };
    match registry.add_book(payload).await {
        Ok(book) => HttpResponse::Created().json(book),
        Err(e) => crate::utils::map_error(e),
    }
}

pub async fn update_book_by_path(
    registry: web::Data<GrpcRegistry>,
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Bytes,
) -> impl Responder {
    if !has_role(&req, ROLE_USER) && !has_role(&req, ROLE_ADMIN) {
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'user' or 'admin' role"}));
    }

    let isbn = path.into_inner();

    let mut value = match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Invalid JSON: {}", e)}));
        }
    };

    let Some(obj) = value.as_object_mut() else {
        return HttpResponse::BadRequest().json(json!({"error": "Invalid JSON: expected object"}));
    };

    obj.insert("isbn".to_string(), serde_json::Value::String(isbn.clone()));

    let payload = match serde_json::from_value::<BookAddPayload>(value) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Invalid JSON: {}", e)}));
        }
    };

    match registry.update_book(Some(isbn), payload).await {
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
        return HttpResponse::Forbidden().json(json!({"error": "Requires 'admin' role"}));
    }
    match registry.delete_book(&path.into_inner()).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => crate::utils::map_error(e),
    }
}
